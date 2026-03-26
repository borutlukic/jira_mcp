use rmcp::{
    RoleServer,
    handler::server::wrapper::Parameters,
    schemars,
    service::RequestContext,
    tool, tool_router,
};

use jira_api::types::SearchRequestBean;
use crate::server::jira::Jira;
use crate::server::util::format_issue;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SearchIssueParams {
    /// JQL query string (e.g., 'project = SHTP AND status = "In Progress"')
    jql: String,
    /// Comma-separated list of fields to retrieve (e.g., 'summary,status,assignee'). If not specified, all fields are returned.
    fields: Option<String>,
    /// Comma-separated list of fields to expand for additional details (e.g., 'transitions,changelog,subtasks,description').
    expand: Option<String>,
}

#[tool_router(router = search_tool_router, vis = "pub(crate)")]
impl Jira {
    #[tool(description = "Search for Jira issues using JQL (Jira Query Language). Returns key details like summary, status, assignee, and priority for matching issues")]
    async fn jira_search_issue(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<SearchIssueParams>,
    ) -> String {
        let fields = params.fields.map(|f| {
            f.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>()
        });

        let expand = Some(params.expand.map(|e| {
            e.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>()
        }).unwrap_or_else(|| {
            vec![
                "transitions".to_string(),
                "changelog".to_string(),
                "subtasks".to_string(),
                "description".to_string(),
            ]
        }));

        let request = SearchRequestBean {
            jql: Some(params.jql),
            fields,
            expand,
            max_results: None,
            start_at: None,
            validate_query: None,
        };

        match self.get_jira_client(&ctx).search_using_search_request(request).await {
            Ok(results) => {
                let issues = results.issues.unwrap_or_default();
                if issues.is_empty() {
                    return "No issues found.".to_string();
                }
                issues.iter().map(format_issue).collect::<Vec<_>>().join("\n---\n\n")
            }
            Err(e) => self.jira_client_error_response(&e),
        }
    }
}
