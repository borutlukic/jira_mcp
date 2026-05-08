use jira_api::HttpClient;
use rmcp::{
    ServerHandler,
    handler::server::tool::ToolRouter,
    model::ServerInfo,
    service::RequestContext,
    RoleServer,
};

pub struct Jira {
    client: HttpClient,
    base_url: String,
    token: String,
    pub(crate) tool_router: ToolRouter<Self>,
}

impl Jira {
    pub fn new(client: HttpClient, base_url: String, token: String) -> Self {
        Self {
            tool_router: Self::search_tool_router()
                + Self::user_tool_router()
                + Self::issue_tool_router()
                + Self::worklog_tool_router()
                + Self::project_tool_router()
                + Self::field_tool_router(),
            client,
            base_url,
            token,
        }
    }

    pub(crate) fn get_jira_client(&self, _ctx: &RequestContext<RoleServer>) -> &HttpClient {
        &self.client
    }

    pub(crate) fn base_url(&self) -> &str {
        &self.base_url
    }

    pub(crate) fn token(&self) -> &str {
        &self.token
    }

    pub(crate) fn jira_client_error_response(&self, e: &jira_api::HttpError) -> String {
        format!("Error: {e}")
    }
}

#[rmcp::tool_handler]
impl ServerHandler for Jira {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(rmcp::model::Implementation::new(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
    }
}
