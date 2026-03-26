
use clap::Parser;
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self}
};

mod server;

type Error = Box<dyn std::error::Error>;
type Result<T> = core::result::Result<T, Error>;

#[derive(Parser)]
struct Args {
    /// Jira base URL (e.g. https://jira.example.com/rest)
    #[arg(long, env = "JIRA_URL")]
    jira_url: String,

    /// Jira personal access token
    #[arg(long, env = "JIRA_TOKEN")]
    jira_token: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let tracing_log_writer = tracing_subscriber::fmt::writer::BoxMakeWriter::new(std::io::stderr);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(
            tracing_subscriber::fmt::layer().with_writer(tracing_log_writer)
        )
        .init();

    let server = server::ServerBuilder::stdio_server()
        .build(args.jira_url, args.jira_token)?;

    server.run().await?;

    Ok(())
}
