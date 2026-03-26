This crate exposes the Jira DataCenter REST API as a Rust client library.
It is generated at build time from the Jira OpenAPI spec using the `openapi-to-rust` crate.

## Source Files

### `src/lib.rs`
Re-exports everything from the generated module:
```rust
pub mod generated;
pub use generated::*;
```

### `build.rs`
Downloads the OpenAPI spec if missing, runs analysis with several fixes, then generates code into `src/generated/`.

#### Constants
- `JIRA_OPENAPI: &str = "openapi/jira_software_dc_11003_swagger.v3.json"` — local path for the spec
- `JIRA_OPENAPI_URL: &str = "https://dac-static.atlassian.com/server/jira/platform/jira_software_dc_11003_swagger.v3.json"` — download URL
- `DEBUG_OUTPUT: bool = true` — emits `cargo:warning` messages for each fix applied

#### `download_spec_if_missing()`
- Returns early if the file already exists.
- Creates the `openapi/` directory if needed.
- Downloads using `ureq::get(...).call()`, reads the body into a `String`, writes to `JIRA_OPENAPI`.

#### Analysis fixes (applied in this order after `analyzer.analyze()`)

1. **`rename_schema(analysis, "Option", "OptionBasic")`**
   Renames the schema named `Option` to `OptionBasic` everywhere it appears:
   - Removes the entry from `analysis.schemas`, updates `schema.name`, re-inserts under the new name.
   - For all other schemas: removes/re-inserts in `schema.dependencies`; recurses into `SchemaType::Object` properties via `rename_in_schema_type`.
   - Updates `analysis.dependencies.edges`: renames the key `"Option"` and removes/re-inserts the name from every dependency set.
   - `rename_in_schema_type` handles: `Reference { target }`, `Array { item_type }`, `Union { variants }` (updates `v.target`), `Composition { schemas }` (updates `s.target`), `DiscriminatedUnion { variants }` (updates `v.schema_ref`), `Object { properties }` (recurses into each property's `schema_type`).

2. **`dedup_properties(analysis)`**
   For every schema whose type is `SchemaType::Object`, collects property keys that are duplicates (case-insensitive), then removes them from the `properties` map.

3. **`fix_parameters(analysis)`**
   For each operation, collects path parameter names from `{param}` segments in `operation.path` (lowercased), then calls `operation.parameters.retain(...)` to remove any parameter whose `location == "path"` but whose name is not in that set.

4. **`rename_body_schema_type(analysis, "worklog", "Worklog")`**
   For each operation, if `operation.request_body` is `Some(RequestBodyContent::Json { schema_name })` and `schema_name == "worklog"`, replaces it with `"Worklog"`.

5. **`inject_array_schema(analysis, "WorklogList", "Worklog")`**
   Creates a synthetic schema `WorklogList = Vec<Worklog>` by inserting an `AnalyzedSchema` with `SchemaType::Array { item_type: Reference("Worklog") }` into `analysis.schemas` and registering the dependency edge `WorklogList → Worklog`. The generator then emits `pub type WorklogList = Vec<Worklog>;` in `types.rs`.

6. **`fix_response_schema_type(analysis, "/api/2/worklog/list", "POST", "WorklogList")`**
   Iterates all operations; for each one whose `path` and `method` match, inserts `"WorklogList"` as the `"200"` entry in `operation.response_schemas`, overwriting the incorrect `"worklog"` entry from the spec. This causes `get_worklogs_for_ids` to be generated with return type `HttpResult<WorklogList>` (i.e. `HttpResult<Vec<Worklog>>`) instead of the incorrect `HttpResult<Worklog>`.

#### `GeneratorConfig`
```rust
GeneratorConfig {
    spec_path: PathBuf::from(JIRA_OPENAPI),
    output_dir: PathBuf::from("src/generated"),
    module_name: "jira_api".to_string(),
    enable_sse_client: false,
    enable_async_client: true,
    tracing_enabled: true,
    ..Default::default()
}
```

#### `main()` sequence
1. `println!("cargo:rerun-if-changed={}", JIRA_OPENAPI)`
2. `download_spec_if_missing()?`
3. Read and parse the spec as `serde_json::Value`
4. `SchemaAnalyzer::new(spec_value)?.analyze()?`
5. Apply the four fixes above
6. Build `GeneratorConfig`, create `CodeGenerator`, call `generate_all(&mut analysis)` then `write_files(&result)`

## `Cargo.toml` key dependencies
- **build-dependencies**: `openapi-to-rust = "0.1.1"`, `serde_json = "1.0"`, `ureq = "2"`
- **dependencies**: `reqwest`, `reqwest-middleware`, `reqwest-tracing`, `serde`, `serde_json`, `serde_urlencoded = "0.7"`, `thiserror`
