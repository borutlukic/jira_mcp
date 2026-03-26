use rmcp::{
    RoleServer,
    service::RequestContext,
    tool, tool_router,
};

use jira_api::types::{ProjectBean, ProjectTypeBean};
use crate::server::jira::Jira;

#[tool_router(router = project_tool_router, vis = "pub(crate)")]
impl Jira {
    #[tool(description = "List all Jira projects accessible to the current user")]
    async fn jira_get_all_projects(
        &self,
        ctx: RequestContext<RoleServer>,
    ) -> String {
        match self.get_jira_client(&ctx).get_all_projects(None, None::<String>, None, None).await {
            Ok(project) => format_project(&project),
            Err(e) => self.jira_client_error_response(&e),
        }
    }

    #[tool(description = "List all Jira project types available on this instance")]
    async fn jira_get_all_project_types(
        &self,
        ctx: RequestContext<RoleServer>,
    ) -> String {
        match self.get_jira_client(&ctx).get_all_project_types().await {
            Ok(project_type) => format_project_type(&project_type),
            Err(e) => self.jira_client_error_response(&e),
        }
    }
}

fn format_project(project: &ProjectBean) -> String {
    let mut out = String::new();

    if let Some(key) = &project.key {
        out.push_str(&format!("Key: {}\n", key));
    }
    if let Some(id) = &project.id {
        out.push_str(&format!("ID: {}\n", id));
    }
    if let Some(name) = &project.name {
        out.push_str(&format!("Name: {}\n", name));
    }
    if let Some(desc) = &project.description {
        if !desc.is_empty() {
            out.push_str(&format!("Description: {}\n", desc));
        }
    }
    if let Some(archived) = project.archived {
        if archived {
            out.push_str("Archived: true\n");
        }
    }

    out
}

fn format_project_type(project_type: &ProjectTypeBean) -> String {
    let mut out = String::new();

    if let Some(key) = &project_type.key {
        out.push_str(&format!("Key: {}\n", key));
    }
    if let Some(formatted_key) = &project_type.formatted_key {
        out.push_str(&format!("Formatted Key: {}\n", formatted_key));
    }
    if let Some(color) = &project_type.color {
        if !color.is_empty() {
            out.push_str(&format!("Color: {}\n", color));
        }
    }

    out
}
