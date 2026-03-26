use rmcp::{
    RoleServer,
    handler::server::wrapper::Parameters,
    schemars,
    service::RequestContext,
    tool, tool_router,
};

use crate::server::jira::Jira;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GetCurrentUserParams {
    /// When true, returns a JSON object with name, display_name, and email fields instead of a plain text message.
    structured: Option<bool>,
}

#[tool_router(router = user_tool_router, vis = "pub(crate)")]
impl Jira {
    #[tool(description = "Get current Jira user information. Pass structured=true to get a JSON object with name, display_name, and email.")]
    async fn jira_get_current_user(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<GetCurrentUserParams>,
    ) -> String {
        match self.get_jira_client(&ctx).get_user().await {
            Ok(user) => {
                if params.structured.unwrap_or(false) {
                    let obj = serde_json::json!({
                        "name": user.name.unwrap_or_default(),
                        "display_name": user.display_name.unwrap_or_default(),
                        "email": user.email_address.unwrap_or_default(),
                    });
                    obj.to_string()
                } else {
                    format!("Your name on Jira is {}", user.display_name.unwrap_or_default())
                }
            }
            Err(e) => self.jira_client_error_response(&e),
        }
    }
}
