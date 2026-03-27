I want to build this as a MCP Bundle, abbreviated as "MCPB". Please follow these steps:

1. **Read the specifications thoroughly:**
   - https://github.com/anthropics/mcpb/blob/main/README.md - MCPB architecture overview, capabilities, and integration patterns
   - https://github.com/anthropics/mcpb/blob/main/MANIFEST.md - Complete bundle manifest structure and field definitions

2. **Create a proper bundle structure:**
   - Generate a valid `manifest.json` following the MANIFEST.md spec
   - The MCP server is built from the workspace root using `make` (see `Makefile`)
   - Linux binary: built with `make linux` → placed at `packaging/server/jira-mcp-server-linux-x86_64`
   - Windows binary: built with `make windows` → placed at `packaging/server/jira-mcp-server.exe`
   - Both platforms: `make all` builds both and then runs `mcpb pack` to produce `target/jira-mcp-server.mcpb`

## Project Details

**Binary name:** `jira_mcp` (Cargo package name), but the output binaries are renamed:
- Linux: `jira-mcp-server-linux-x86_64`
- Windows: `jira-mcp-server.exe`

**CLI arguments** (defined in `src/main.rs`):
- `--jira-url <URL>` (also via env `JIRA_URL`) — Jira base URL, e.g. `https://jira.example.com`
- `--jira-token <TOKEN>` (also via env `JIRA_TOKEN`) — Jira personal access token

## manifest.json Guidelines

- `manifest_version`: `"0.3"`
- `name`: `"jira-mcp"`
- `display_name`: `"Jira MCP Server"`
- `version`: must match `version` field in root `Cargo.toml`
- `server.type`: `"binary"`
- `server.entry_point`: platform-specific — `"server/jira-mcp-server-linux-x86_64"` for Linux, `"server/jira-mcp-server.exe"` for Windows
- `server.mcp_config.command`: `"${__dirname}/server/jira-mcp-server-linux-x86_64"` (Linux) or `"${__dirname}/server/jira-mcp-server.exe"` (Windows)
- `server.mcp_config.args`: pass `--jira-url` and `--jira-token` using `${user_config.*}` variables
- `compatibility.platforms`: `["linux", "win32"]` — both Linux and Windows builds are included
- `user_config` fields:
  - `jira_url` — type `string`, required, title "Jira Base URL"
  - `jira_token` — type `string`, required, sensitive, title "Jira Personal Access Token"

## Tools (defined in `src/server/`)

List every `#[tool]` function found under `src/server/`:

**Before generating the manifest**, grep `src/server/` for `#[tool(` to get the up-to-date list and descriptions, as new tools may have been added since this file was written.

