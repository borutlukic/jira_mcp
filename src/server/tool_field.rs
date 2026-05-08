use rmcp::{
    RoleServer,
    handler::server::wrapper::Parameters,
    schemars,
    service::RequestContext,
    tool, tool_router,
};

use jira_api::types::FieldBean;
use crate::server::jira::Jira;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GetFieldsParams {
    /// If true, return raw JSON (array of FieldBean) instead of formatted text.
    structured: Option<bool>,
    /// If true, only include custom fields (custom == true). Defaults to false.
    custom_only: Option<bool>,
    /// Case-insensitive substring filter; keeps only fields whose name, id, or
    /// clauseNames contain this text.
    search: Option<String>,
}

#[tool_router(router = field_tool_router, vis = "pub(crate)")]
impl Jira {
    #[tool(description = "List Jira field metadata (system + custom). Use this to discover customfield_* IDs needed for JQL queries or jira_update_issue. Set 'search: <text>' to return only fields whose name, id, or clauseNames contain the text (case-insensitive). Set 'custom_only: true' to return only custom fields. Set 'structured: true' to receive raw JSON instead of formatted text.")]
    async fn jira_get_fields(
        &self,
        _ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<GetFieldsParams>,
    ) -> String {
        // The typed `HttpClient::get_fields()` is broken upstream — it's typed
        // `HttpResult<FieldBean>` (singular), but `/rest/api/2/field` returns a
        // JSON array. Bypass it with a direct request that deserializes as Vec.
        let url = format!("{}/api/2/field", self.base_url().trim_end_matches('/'));
        let response = match reqwest::Client::new()
            .get(&url)
            .bearer_auth(self.token())
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => return format!("Error: HTTP request failed: {e}"),
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return format!("Error: {} {}", status, body);
        }

        let fields: Vec<FieldBean> = match response.json().await {
            Ok(v) => v,
            Err(e) => return format!("Error: Failed to deserialize response: {e}"),
        };

        let search = params
            .search
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
            .map(str::to_lowercase);

        let mut filtered: Vec<&FieldBean> = fields
            .iter()
            .filter(|f| !params.custom_only.unwrap_or(false) || f.custom == Some(true))
            .filter(|f| match &search {
                None => true,
                Some(q) => {
                    let name_hit = f
                        .name
                        .as_deref()
                        .is_some_and(|n| n.to_lowercase().contains(q));
                    let id_hit = f
                        .id
                        .as_deref()
                        .is_some_and(|i| i.to_lowercase().contains(q));
                    let clause_hit = f
                        .clause_names
                        .as_ref()
                        .is_some_and(|cs| cs.iter().any(|c| c.to_lowercase().contains(q)));
                    name_hit || id_hit || clause_hit
                }
            })
            .collect();

        filtered.sort_by(|a, b| {
            let custom_a = a.custom.unwrap_or(false);
            let custom_b = b.custom.unwrap_or(false);
            custom_a
                .cmp(&custom_b)
                .then_with(|| {
                    a.name
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .cmp(&b.name.as_deref().unwrap_or("").to_lowercase())
                })
        });

        if params.structured.unwrap_or(false) {
            return serde_json::to_string_pretty(&filtered)
                .unwrap_or_else(|_| format_fields_text(&filtered));
        }
        format_fields_text(&filtered)
    }
}

fn format_fields_text(fields: &[&FieldBean]) -> String {
    if fields.is_empty() {
        return "No fields found.\n".to_string();
    }

    let mut out = format!("Fields ({}):\n\n", fields.len());
    for f in fields {
        let id = f.id.as_deref().unwrap_or("?");
        let name = f.name.as_deref().unwrap_or("?");
        let is_custom = f.custom.unwrap_or(false);
        out.push_str(&format!("- {} (id: {}, custom: {})\n", name, id, if is_custom { "yes" } else { "no" }));

        if let Some(schema) = &f.schema {
            let mut parts: Vec<String> = Vec::new();
            if let Some(t) = schema.type_.as_deref() {
                parts.push(format!("type={}", t));
            }
            if let Some(items) = schema.items.as_deref() {
                parts.push(format!("items={}", items));
            }
            if let Some(custom_key) = schema.custom.as_deref() {
                parts.push(format!("custom={}", custom_key));
            }
            if !parts.is_empty() {
                out.push_str(&format!("    schema: {}\n", parts.join(", ")));
            }
        }

        if let Some(clauses) = &f.clause_names {
            if !clauses.is_empty() {
                out.push_str(&format!("    clauseNames: {}\n", clauses.join(", ")));
            }
        }
    }
    out
}
