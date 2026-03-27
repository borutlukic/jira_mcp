# Jira MCP Server

A Rust-based [MCP](https://modelcontextprotocol.io) server for interacting with a Jira DataCenter instance. Distributed as an `.mcpb` (MCP Bundle) package for easy installation.

## Tools

| Tool | Description |
|------|-------------|
| `jira_get_current_user` | Get current user information |
| `jira_search_issue` | Search issues using JQL |
| `jira_get_issue` | Get an issue by ID or key, including comments |
| `jira_create_issue` | Create a new issue |
| `jira_update_issue` | Update issue fields or transition status |
| `jira_add_comment` | Add a comment to an issue |
| `jira_get_transitions` | List available workflow transitions for an issue |
| `jira_get_worklogs` | Get all worklogs updated since a given timestamp |
| `jira_add_worklog` | Log work on an issue |
| `jira_get_all_projects` | List all accessible projects |
| `jira_get_all_project_types` | List all project types on the instance |

## Installation

Install via the `.mcpb` bundle using the [`mcpb`](https://github.com/anthropics/mcpb) CLI:

```sh
mcpb install jira-mcp-server.mcpb
```

You will be prompted for your Jira base URL and personal access token.

## Configuration

| Parameter | Env var | Description |
|-----------|---------|-------------|
| `--jira-url <URL>` | `JIRA_URL` | Base URL of your Jira DataCenter instance (e.g. `https://jira.example.com`) |
| `--jira-token <TOKEN>` | `JIRA_TOKEN` | Personal access token for authentication |

To generate a personal access token in Jira DataCenter: **Profile → Personal Access Tokens → Create token**.

## Building from Source

Requires Rust and the `mcpb` CLI. Run `make setup` once to install cross-compilation targets.

```sh
make setup   # install cross-compilation targets (once)
make all     # build Linux + Windows binaries and produce .mcpb bundle
```

| Target | Description |
|--------|-------------|
| `make linux` | Build Linux binary → `packaging/server/jira-mcp-server-linux-x86_64` |
| `make windows` | Build Windows binary → `packaging/server/jira-mcp-server.exe` |
| `make pack` | Package both binaries into `target/jira-mcp-server.mcpb` |
| `make clean` | Remove build artifacts |

## Dependencies

- [`jira_api`](https://github.com/borutlukic/jira_api) — Rust client for the Jira DataCenter REST API (branch `dc_11003`)
- [`rmcp`](https://crates.io/crates/rmcp) — MCP server framework
- [`tokio`](https://crates.io/crates/tokio) — Async runtime
- [`clap`](https://crates.io/crates/clap) — CLI argument parsing
