# Jira MCP Server

A Rust-based MCP (Model Context Protocol) server that provides tools for interacting with a Jira DataCenter instance. It is distributed as an `.mcpb` (MCP Bundle) package.

## Project Structure

- **`src/`** — The `jira_mcp` binary crate: the MCP server executable. See `src/CLAUDE.md` for details.
- **`packaging/`** — Assets and layout for the `.mcpb` bundle. See `packaging/CLAUDE.md` for details.

The Jira API client library lives in a separate repo: [borutlukic/jira_api](https://github.com/borutlukic/jira_api) (branch `dc_11003`). It is referenced as a git dependency in `Cargo.toml`.

## Key Dependencies

- [`rmcp`](https://crates.io/crates/rmcp) — MCP server/client framework with stdio transport
- [`tokio`](https://crates.io/crates/tokio) — Async runtime
- [`clap`](https://crates.io/crates/clap) — CLI argument parsing (also reads from env vars)
- [`reqwest`](https://crates.io/crates/reqwest) / `reqwest-middleware` / `reqwest-tracing` — HTTP client stack
- [`serde`](https://crates.io/crates/serde) / `serde_json` — Serialization

## Build System (Makefile)

All build targets use cross-compilation. Run `make setup` once to install the required Rust targets.

| Target | Description |
|--------|-------------|
| `make setup` | Install cross-compilation targets and clone the macOS SDK (`phracker/MacOSX-SDKs` → `~/macos-sdk`) |
| `make linux` | Build release binary for Linux → `packaging/server/jira-mcp-server-linux-x86_64` |
| `make windows` | Build release binary for Windows → `packaging/server/jira-mcp-server.exe` |
| `make mac-x86` | Build release binary for macOS x86_64 → `packaging/server/jira-mcp-server-macos-x86_64` (uses osxcross) |
| `make mac-arm` | Build release binary for macOS aarch64 → `packaging/server/jira-mcp-server-macos-aarch64` (uses osxcross) |
| `make pack` | Run `mcpb pack` to produce `target/jira-mcp-server.mcpb` (requires all four binaries) |
| `make all` | Run `linux` + `windows` + `mac-x86` + `mac-arm` + `pack` |
| `make clean` | Remove build artifacts and output binaries |

macOS builds use [osxcross](https://github.com/tpoechtrager/osxcross) pre-installed in the devcontainer at `/opt/osxcross`. The toolchain is built from `MacOSX11.3.sdk` (sourced from `phracker/MacOSX-SDKs`) and supports both x86_64 and aarch64.

## Runtime Configuration

The server accepts two required arguments (also configurable via environment variables):

| Argument | Env var | Description |
|----------|---------|-------------|
| `--jira-url <URL>` | `JIRA_URL` | Base URL of the Jira DataCenter instance |
| `--jira-token <TOKEN>` | `JIRA_TOKEN` | Personal access token for authentication |

## Distribution (.mcpb Bundle)

The bundle is produced by `make pack` using the `mcpb` CLI tool. The bundle layout lives under `packaging/` and includes a `manifest.json`, an `icon.png`, and the compiled server binaries. See `packaging/CLAUDE.md` for the full bundle spec.
