use jira_api::HttpClient;
use rmcp::ServiceExt;

use super::jira::Jira;

type Error = Box<dyn std::error::Error>;
type Result<T> = core::result::Result<T, Error>;

enum Transport {
    Stdio,
}

pub struct ServerBuilder {
    transport: Transport,
}

impl ServerBuilder {
    pub fn stdio_server() -> Self {
        Self {
            transport: Transport::Stdio,
        }
    }

    pub fn build(self, base_url: String, token: String) -> Result<Server> {
        let client = HttpClient::new()
            .with_base_url(base_url.clone())
            .with_api_key(token.clone());

        Ok(Server {
            transport: self.transport,
            handler: Jira::new(client, base_url, token),
        })
    }
}

pub struct Server {
    transport: Transport,
    handler: Jira,
}

impl Server {
    pub async fn run(self) -> Result<()> {
        match self.transport {
            Transport::Stdio => {
                tracing::info!("Running Jira MCP Server (stdio/stdout)");
                let service = self
                    .handler
                    .serve(rmcp::transport::io::stdio())
                    .await?;
                service.waiting().await?;
                Ok(())
            }
        }
    }
}
