# Jira MCP Server

A Rust-based MCP (Model Context Protocol) server that provides tools for interacting with a Jira DataCenter instance. It is distributed as an `.mcpb` (MCP Bundle) package.

## Workspace Structure

This is a Cargo workspace with two crates:

- **`src/`** — The `jira_mcp` binary crate: the MCP server executable. See `src/CLAUDE.md` for details.
- **`jira_api/`** — A library crate providing the Jira API client. See `jira_api/CLAUDE.md` for details.
- **`packaging/`** — Assets and layout for the `.mcpb` bundle. See `packaging/CLAUDE.md` for details.

## Key Dependencies

- [`rmcp`](https://crates.io/crates/rmcp) — MCP server/client framework with stdio transport
- [`tokio`](https://crates.io/crates/tokio) — Async runtime
- [`clap`](https://crates.io/crates/clap) — CLI argument parsing (also reads from env vars)
- [`reqwest`](https://crates.io/crates/reqwest) / `reqwest-middleware` / `reqwest-tracing` — HTTP client stack (shared workspace deps)
- [`serde`](https://crates.io/crates/serde) / `serde_json` — Serialization (shared workspace deps)

## Build System (Makefile)

All build targets use cross-compilation. Run `make setup` once to install the required Rust targets.

| Target | Description |
|--------|-------------|
| `make setup` | Install cross-compilation targets (`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-gnu`) |
| `make linux` | Build release binary for Linux → `packaging/server/jira-mcp-server-linux-x86_64` |
| `make windows` | Build release binary for Windows → `packaging/server/jira-mcp-server.exe` |
| `make pack` | Run `mcpb pack` to produce `target/jira-mcp-server.mcpb` (requires both binaries) |
| `make all` | Run `linux` + `windows` + `pack` |
| `make clean` | Remove build artifacts and output binaries |

## Runtime Configuration

The server accepts two required arguments (also configurable via environment variables):

| Argument | Env var | Description |
|----------|---------|-------------|
| `--jira-url <URL>` | `JIRA_URL` | Base URL of the Jira DataCenter instance |
| `--jira-token <TOKEN>` | `JIRA_TOKEN` | Personal access token for authentication |

## Distribution (.mcpb Bundle)

The bundle is produced by `make pack` using the `mcpb` CLI tool. The bundle layout lives under `packaging/` and includes a `manifest.json`, an `icon.png`, and the compiled server binaries. See `packaging/CLAUDE.md` for the full bundle spec.
