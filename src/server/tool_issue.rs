use rmcp::{
    RoleServer,
    handler::server::wrapper::Parameters,
    schemars,
    service::RequestContext,
    tool, tool_router,
};
use std::collections::BTreeMap;

use jira_api::types::{
    CommentJsonBean, IssueUpdateBean, IssueUpdateBeanFields, TransitionBean,
};
use crate::server::jira::Jira;
use crate::server::util::format_issue;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GetIssueParams {
    /// Issue ID or key (e.g., 'PROJ-123')
    issue_id_or_key: String,
    /// Comma-separated list of fields to retrieve (e.g., 'summary,status,assignee'). If not specified, all fields are returned.
    fields: Option<String>,
    /// Comma-separated list of fields to expand (e.g., 'transitions,changelog,subtasks'). Defaults to 'transitions,changelog,subtasks,description,comment'.
    expand: Option<String>,
    /// Comma-separated list of issue properties to return (e.g., 'prop1,prop2').
    properties: Option<String>,
    /// Maximum number of comments to include. Defaults to 10. Use 0 to fetch all comments.
    comment_limit: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct CreateIssueParams {
    /// Project key (e.g., 'PROJ')
    project_key: String,
    /// Issue summary/title
    summary: String,
    /// Issue type name (e.g., 'Bug', 'Story', 'Task', 'Sub-task')
    issue_type: String,
    /// Issue description (plain text)
    description: Option<String>,
    /// Username of the assignee
    assignee: Option<String>,
    /// Priority name (e.g., 'High', 'Medium', 'Low')
    priority: Option<String>,
    /// Comma-separated list of component names (e.g., 'Backend,API')
    components: Option<String>,
    /// Comma-separated list of labels
    labels: Option<String>,
    /// Parent issue key, used when creating a sub-task
    parent_key: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct UpdateIssueParams {
    /// Issue ID or key (e.g., 'PROJ-123')
    issue_id_or_key: String,
    /// New summary/title
    summary: Option<String>,
    /// New description (plain text)
    description: Option<String>,
    /// Username of the new assignee. Pass an empty string to unassign.
    assignee: Option<String>,
    /// Priority name (e.g., 'High', 'Medium', 'Low')
    priority: Option<String>,
    /// Comma-separated list of component names (replaces existing components)
    components: Option<String>,
    /// Comma-separated list of labels (replaces existing labels)
    labels: Option<String>,
    /// Target status name to transition the issue to (e.g., 'In Progress', 'Done')
    status: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct AddCommentParams {
    /// Issue ID or key (e.g., 'PROJ-123')
    issue_id_or_key: String,
    /// Comment body text
    body: String,
}

#[tool_router(router = issue_tool_router, vis = "pub(crate)")]
impl Jira {
    #[tool(description = "Get a Jira issue by ID or key, including all fields and comments")]
    async fn jira_get_issue(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<GetIssueParams>,
    ) -> String {
        let client = self.get_jira_client(&ctx);

        let expand = Some(
            params.expand.unwrap_or_else(|| {
                "transitions,changelog,subtasks,description,comment".to_string()
            })
        );

        let issue = match client
            .get_issue(
                &params.issue_id_or_key,
                expand.as_deref(),
                params.fields.as_deref(),
                None::<String>,
                params.properties.as_deref(),
            )
            .await
        {
            Ok(issue) => issue,
            Err(e) => return self.jira_client_error_response(&e),
        };

        let mut out = format_issue(&issue);

        // Fetch comments with optional limit
        let comment_limit = params.comment_limit.unwrap_or(10);
        let max_results = if comment_limit == 0 {
            None::<String>  // 0 means fetch all
        } else {
            Some(comment_limit.to_string())
        };

        match client
            .get_comments(
                &params.issue_id_or_key,
                None::<String>,
                max_results.as_deref(),
                Some("created"),
                None::<String>,
            )
            .await
        {
            Ok(comments_bean) => {
                let total = comments_bean.total.unwrap_or(0);
                let comments = comments_bean.comments.unwrap_or_default();
                if !comments.is_empty() {
                    let header = if comment_limit > 0 && total > comment_limit as i32 {
                        format!(
                            "\n--- Comments (showing {} of {}) ---\n",
                            comments.len(),
                            total
                        )
                    } else {
                        format!("\n--- Comments ({}) ---\n", comments.len())
                    };
                    out.push_str(&header);
                    for comment in &comments {
                        out.push_str(&format_comment(comment));
                        out.push_str("---\n");
                    }
                }
            }
            Err(_) => {} // best-effort; issue data already returned above
        }

        out
    }

    #[tool(description = "Create a new Jira issue in a project")]
    async fn jira_create_issue(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<CreateIssueParams>,
    ) -> String {
        let mut fields: BTreeMap<String, serde_json::Value> = BTreeMap::new();

        fields.insert("project".to_string(), serde_json::json!({"key": params.project_key}));
        fields.insert("summary".to_string(), serde_json::Value::String(params.summary));
        fields.insert("issuetype".to_string(), serde_json::json!({"name": params.issue_type}));

        if let Some(desc) = params.description {
            fields.insert("description".to_string(), serde_json::Value::String(desc));
        }
        if let Some(assignee) = params.assignee {
            fields.insert("assignee".to_string(), serde_json::json!({"name": assignee}));
        }
        if let Some(priority) = params.priority {
            fields.insert("priority".to_string(), serde_json::json!({"name": priority}));
        }
        if let Some(components_str) = params.components {
            let components: Vec<serde_json::Value> = components_str
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| serde_json::json!({"name": s}))
                .collect();
            if !components.is_empty() {
                fields.insert("components".to_string(), serde_json::Value::Array(components));
            }
        }
        if let Some(labels_str) = params.labels {
            let labels: Vec<serde_json::Value> = labels_str
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| serde_json::Value::String(s.to_string()))
                .collect();
            if !labels.is_empty() {
                fields.insert("labels".to_string(), serde_json::Value::Array(labels));
            }
        }
        if let Some(parent_key) = params.parent_key {
            fields.insert("parent".to_string(), serde_json::json!({"key": parent_key}));
        }

        let request = IssueUpdateBean {
            fields: Some(IssueUpdateBeanFields { additional_properties: fields }),
            history_metadata: None,
            properties: None,
            transition: None,
            update: None,
        };

        match self.get_jira_client(&ctx).create_issue(None, request).await {
            Ok(response) => {
                let key = response.key.unwrap_or_default();
                let id = response.id.unwrap_or_default();
                let url = response.self_.unwrap_or_default();
                format!("Issue created successfully.\nKey: {}\nID: {}\nURL: {}", key, id, url)
            }
            Err(e) => self.jira_client_error_response(&e),
        }
    }

    #[tool(description = "Update fields of an existing Jira issue. Only the fields you provide will be changed. To transition the issue status, provide a 'status' value matching an available transition name.")]
    async fn jira_update_issue(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<UpdateIssueParams>,
    ) -> String {
        let client = self.get_jira_client(&ctx);
        let mut results: Vec<String> = Vec::new();

        // Handle status transition separately — requires its own API call
        if let Some(target_status) = params.status {
            match client.get_transitions(&params.issue_id_or_key, None::<String>).await {
                Err(e) => return self.jira_client_error_response(&e),
                Ok(meta) => {
                    let transitions = meta.transitions.unwrap_or_default();
                    let target_lower = target_status.to_lowercase();

                    let matched = transitions.iter().find(|t| {
                        t.name.as_deref()
                            .map(|n| n.to_lowercase() == target_lower)
                            .unwrap_or(false)
                    });

                    match matched {
                        None => {
                            let available: Vec<&str> = transitions
                                .iter()
                                .filter_map(|t| t.name.as_deref())
                                .collect();
                            return format!(
                                "Unknown status '{}'. Available transitions: {}",
                                target_status,
                                available.join(", ")
                            );
                        }
                        Some(transition) => {
                            let transition_id = transition.id.clone().unwrap_or_default();
                            let request = IssueUpdateBean {
                                transition: Some(TransitionBean {
                                    id: Some(transition_id),
                                    description: None,
                                    fields: None,
                                    name: None,
                                    opsbar_sequence: None,
                                    to: None,
                                }),
                                fields: None,
                                history_metadata: None,
                                properties: None,
                                update: None,
                            };
                            match client.do_transition(&params.issue_id_or_key, request).await {
                                Ok(()) => results.push(format!("Status transitioned to '{}'.", target_status)),
                                Err(e) => return self.jira_client_error_response(&e),
                            }
                        }
                    }
                }
            }
        }

        // Handle field updates
        let mut fields: BTreeMap<String, serde_json::Value> = BTreeMap::new();

        if let Some(summary) = params.summary {
            fields.insert("summary".to_string(), serde_json::Value::String(summary));
        }
        if let Some(desc) = params.description {
            fields.insert("description".to_string(), serde_json::Value::String(desc));
        }
        if let Some(assignee) = params.assignee {
            // Empty string means unassign
            let value = if assignee.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::json!({"name": assignee})
            };
            fields.insert("assignee".to_string(), value);
        }
        if let Some(priority) = params.priority {
            fields.insert("priority".to_string(), serde_json::json!({"name": priority}));
        }
        if let Some(components_str) = params.components {
            let components: Vec<serde_json::Value> = components_str
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| serde_json::json!({"name": s}))
                .collect();
            fields.insert("components".to_string(), serde_json::Value::Array(components));
        }
        if let Some(labels_str) = params.labels {
            let labels: Vec<serde_json::Value> = labels_str
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| serde_json::Value::String(s.to_string()))
                .collect();
            fields.insert("labels".to_string(), serde_json::Value::Array(labels));
        }

        if !fields.is_empty() {
            let request = IssueUpdateBean {
                fields: Some(IssueUpdateBeanFields { additional_properties: fields }),
                history_metadata: None,
                properties: None,
                transition: None,
                update: None,
            };
            match client.edit_issue(&params.issue_id_or_key, None::<String>, request).await {
                Ok(()) => results.push("Fields updated successfully.".to_string()),
                Err(e) => return self.jira_client_error_response(&e),
            }
        }

        if results.is_empty() {
            "No fields provided to update.".to_string()
        } else {
            format!("Issue {}. {}", params.issue_id_or_key, results.join(" "))
        }
    }

    #[tool(description = "Add a comment to a Jira issue")]
    async fn jira_add_comment(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<AddCommentParams>,
    ) -> String {
        let comment = CommentJsonBean {
            body: Some(params.body),
            author: None,
            created: None,
            id: None,
            properties: None,
            rendered_body: None,
            self_: None,
            update_author: None,
            updated: None,
            visibility: None,
        };

        match self
            .get_jira_client(&ctx)
            .add_comment(&params.issue_id_or_key, None::<String>, comment)
            .await
        {
            Ok(c) => {
                let id = c.id.unwrap_or_default();
                let created = c.created.unwrap_or_default();
                let url = c.self_.unwrap_or_default();
                format!(
                    "Comment added successfully.\nID: {}\nCreated: {}\nURL: {}",
                    id, created, url
                )
            }
            Err(e) => self.jira_client_error_response(&e),
        }
    }

    #[tool(description = "List available workflow transitions for a Jira issue. Use this to discover valid status values before calling jira_update_issue with a 'status' field.")]
    async fn jira_get_transitions(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<GetIssueParams>,
    ) -> String {
        match self
            .get_jira_client(&ctx)
            .get_transitions(&params.issue_id_or_key, None::<String>)
            .await
        {
            Ok(meta) => {
                let transitions = meta.transitions.unwrap_or_default();
                if transitions.is_empty() {
                    return format!("No transitions available for {}.", params.issue_id_or_key);
                }
                let mut out = format!(
                    "Available transitions for {}:\n",
                    params.issue_id_or_key
                );
                for t in &transitions {
                    let id = t.id.as_deref().unwrap_or("?");
                    let name = t.name.as_deref().unwrap_or("?");
                    let to_status = t.to.as_ref()
                        .and_then(|s| s.name.as_deref())
                        .unwrap_or("?");
                    out.push_str(&format!("- {} (ID: {}) → {}\n", name, id, to_status));
                }
                out
            }
            Err(e) => self.jira_client_error_response(&e),
        }
    }
}

fn format_comment(comment: &CommentJsonBean) -> String {
    let mut out = String::new();

    if let Some(author) = &comment.author {
        let name = author.display_name.as_deref().unwrap_or("Unknown");
        out.push_str(&format!("Author: {}\n", name));
    }
    if let Some(created) = &comment.created {
        out.push_str(&format!("Created: {}\n", created));
    }
    if let Some(updated) = &comment.updated {
        if updated != comment.created.as_deref().unwrap_or("") {
            out.push_str(&format!("Updated: {}\n", updated));
        }
    }
    if let Some(body) = &comment.body {
        if !body.is_empty() {
            out.push_str(&format!("Body:\n{}\n", body));
        }
    }

    out
}
