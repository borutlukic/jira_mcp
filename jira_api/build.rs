use openapi_to_rust::{SchemaAnalyzer, CodeGenerator, GeneratorConfig, SchemaAnalysis};
use openapi_to_rust::analysis::{AnalyzedSchema, SchemaType as AnalysisSchemaType};
use std::collections::HashSet;
use std::io::Read;
use std::path::PathBuf;

// https://dac-static.atlassian.com/server/jira/platform/jira_software_dc_11003_swagger.v3.json
const JIRA_OPENAPI: &str = "openapi/jira_software_dc_11003_swagger.v3.json";
const DEBUG_OUTPUT: bool = true;

fn rename_in_schema_type(schema_type: &mut openapi_to_rust::analysis::SchemaType, original_name: &str, new_name: &str) {
    use openapi_to_rust::analysis::SchemaType;
    match schema_type {
        SchemaType::Reference { target } => {
            if target == original_name {
                *target = new_name.to_string();
            }
        }
        SchemaType::Array { item_type } => {
            rename_in_schema_type(item_type, original_name, new_name);
        }
        SchemaType::Union { variants } => {
            for v in variants.iter_mut() {
                if v.target == original_name {
                    v.target = new_name.to_string();
                }
            }
        }
        SchemaType::Composition { schemas } => {
            for s in schemas.iter_mut() {
                if s.target == original_name {
                    s.target = new_name.to_string();
                }
            }
        }
        SchemaType::DiscriminatedUnion { variants, .. } => {
            for v in variants.iter_mut() {
                if v.schema_ref == original_name {
                    v.schema_ref = new_name.to_string();
                }
            }
        }
        SchemaType::Object { properties, .. } => {
            for (_, property) in properties.iter_mut() {
                rename_in_schema_type(&mut property.schema_type, original_name, new_name);
            }
        }
        _ => {}
    }
}

fn rename_schema(analysis: &mut SchemaAnalysis, original_name: String, new_name: String) {
    if DEBUG_OUTPUT { println!("cargo:warning=renaming api schema {original_name} to {new_name}"); }
    if let Some(mut schema) = analysis.schemas.remove(&original_name) {
        schema.name = new_name.clone();
        analysis.schemas.insert(new_name.clone(), schema);
    }

    for (_, schema) in analysis.schemas.iter_mut() {
        if schema.dependencies.remove(&original_name) {
            schema.dependencies.insert(new_name.clone());
        }
        match &mut schema.schema_type {
            openapi_to_rust::analysis::SchemaType::Object { required, properties, .. } => {
                if required.remove(&original_name) {
                    required.insert(new_name.clone());
                }
                for (_, property) in properties.iter_mut() {
                    rename_in_schema_type(&mut property.schema_type, &original_name, &new_name);
                }
            },
            _ => {},
        }
    }

    if let Some(deps) = analysis.dependencies.edges.remove(&"Option".to_string()) {
        analysis.dependencies.edges.insert(new_name.clone(), deps);
    }
    for (_, deps) in analysis.dependencies.edges.iter_mut() {
        if deps.remove(&original_name) {
            deps.insert(new_name.clone());
        }
    }
}

fn dedup_properties(analysis: &mut SchemaAnalysis) {
    use openapi_to_rust::analysis::SchemaType;
    use std::collections::HashSet;
    for (_, schema) in analysis.schemas.iter_mut() {
        if let SchemaType::Object { properties, .. } = &mut schema.schema_type {
            let mut seen: HashSet<String> = HashSet::new();
            let keys_to_remove: Vec<String> = properties
                .keys()
                .filter_map(|k| {
                    if seen.insert(k.to_lowercase()) {
                        None
                    } else {
                        Some(k.clone())
                    }
                })
                .collect();
            for key in keys_to_remove {
                if DEBUG_OUTPUT { println!("cargo:warning=removed duplicate property {} from schema {}", key, schema.name); }
                properties.remove(&key);
            }
        }
    }
}

fn fix_parameters(analysis: &mut SchemaAnalysis) {
    for (_, operation) in analysis.operations.iter_mut() {
        let path = &operation.path;
        // Collect all {param} names from the path template
        let path_params: std::collections::HashSet<String> = path
            .split('{')
            .skip(1)
            .filter_map(|s| s.split('}').next())
            .map(|s| s.to_lowercase())
            .collect();

        operation.parameters.retain(|p| {
            if p.location == "path" && !path_params.contains(&p.name.to_lowercase()) {
                if DEBUG_OUTPUT { println!("cargo:warning=removed parameter {} from api call {path}", p.name); }
                false
            } else {
                true
            }
        });
    }
}

fn rename_body_schema_type(analysis: &mut SchemaAnalysis, from_type: &str, to_type: &str) {
    use openapi_to_rust::analysis::RequestBodyContent;
    for (_, operation) in analysis.operations.iter_mut() {
        if let Some(RequestBodyContent::Json { schema_name }) = &mut operation.request_body {
            if schema_name == from_type {
                if DEBUG_OUTPUT { println!("cargo:warning=renamed request body schema {} -> {} in api call {}", from_type, to_type, operation.path); }
                *schema_name = to_type.to_string();
            }
        }
    }
}

/// Injects a synthetic array-of-T schema into the analysis so the generator can
/// produce `type <name> = Vec<<item_schema>>;` and use it as a return type.
fn inject_array_schema(analysis: &mut SchemaAnalysis, name: &str, item_schema: &str) {
    println!("cargo:warning=injecting array schema {} = Vec<{}>", name, item_schema);
    let mut deps = HashSet::new();
    deps.insert(item_schema.to_string());
    let schema = AnalyzedSchema {
        name: name.to_string(),
        original: serde_json::Value::Null,
        schema_type: AnalysisSchemaType::Array {
            item_type: Box::new(AnalysisSchemaType::Reference {
                target: item_schema.to_string(),
            }),
        },
        dependencies: deps,
        nullable: false,
        description: None,
        default: None,
    };
    analysis.schemas.insert(name.to_string(), schema);
    analysis.dependencies.add_dependency(name.to_string(), item_schema.to_string());
}

/// Replaces the 200 response schema of all operations matching a given path and HTTP method.
fn fix_response_schema_type(analysis: &mut SchemaAnalysis, path: &str, method: &str, new_schema: &str) {
    let method_upper = method.to_uppercase();
    for (_, op) in analysis.operations.iter_mut() {
        if op.path == path && op.method == method_upper {
            let old = op.response_schemas.insert("200".to_string(), new_schema.to_string());
            println!(
                "cargo:warning=fixed response schema for {} {} from {:?} to {}",
                method_upper, path, old, new_schema
            );
        }
    }
}

const JIRA_OPENAPI_URL: &str = "https://dac-static.atlassian.com/server/jira/platform/jira_software_dc_11003_swagger.v3.json";

fn download_spec_if_missing() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::Path::new(JIRA_OPENAPI);
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    println!("cargo:warning=Downloading Jira OpenAPI spec from {}", JIRA_OPENAPI_URL);
    let response = ureq::get(JIRA_OPENAPI_URL).call()?;
    let mut body = String::new();
    response.into_reader().read_to_string(&mut body)?;
    std::fs::write(path, body)?;
    println!("cargo:warning=Saved Jira OpenAPI spec to {}", JIRA_OPENAPI);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed={}", JIRA_OPENAPI);
    println!("cargo:rerun-if-changed=build.rs");

    download_spec_if_missing()?;

    let spec_content = std::fs::read_to_string(JIRA_OPENAPI)?;
    let spec_value: serde_json::Value = serde_json::from_str(&spec_content)?;

    let mut analyzer = SchemaAnalyzer::new(spec_value)?;
    let mut analysis = analyzer.analyze()?;

    // Atlassian has created a struct called Option which interfers with rust's std::Option so we rename it
    rename_schema(&mut analysis, "Option".to_string(), "OptionBasic".to_string());
    
    // Atlassian's OpenAPI is broken and some structs repeat items of the same name
    dedup_properties(&mut analysis);

    // Atlassian's OpenAPI is broken and repeats function call parameters
    fix_parameters(&mut analysis);

    // Fix incorrect request body schema casing
    rename_body_schema_type(&mut analysis, "worklog", "Worklog");

    // POST /api/2/worklog/list returns an array of worklogs, not a single one.
    // Inject a synthetic WorklogList = Vec<Worklog> schema and point the operation at it.
    inject_array_schema(&mut analysis, "WorklogList", "Worklog");
    fix_response_schema_type(&mut analysis, "/api/2/worklog/list", "POST", "WorklogList");

    let config = GeneratorConfig {
        spec_path: PathBuf::from(JIRA_OPENAPI),
        output_dir: PathBuf::from("src/generated"),
        module_name: "jira_api".to_string(),
        enable_sse_client: false,
        enable_async_client: true,
        tracing_enabled: true,
        ..Default::default()
    };

    let generator = CodeGenerator::new(config);
    let result = generator.generate_all(&mut analysis)?;
    generator.write_files(&result)?;

    Ok(())
}
