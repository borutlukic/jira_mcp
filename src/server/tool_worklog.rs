use rmcp::{
    RoleServer,
    handler::server::wrapper::Parameters,
    schemars,
    service::RequestContext,
    tool, tool_router,
};

use jira_api::types::{Worklog, WorklogIdsRequestBean};

/// Parse a flexible timestamp value into Unix milliseconds.
///
/// Accepts:
/// - Unix timestamp in milliseconds (`i64` string, value ≥ 10^10)
/// - Unix timestamp in seconds (`i64` string, value < 10^10)
/// - ISO 8601 / RFC 3339 datetime string (e.g. `2026-03-25T13:00:00+00:00`,
///   `2026-03-25T13:00:00 +00:00`, `2026-03-25T13:00:00`)
fn parse_timestamp_ms(input: &str) -> Result<i64, String> {
    let trimmed = input.trim();

    // Try as plain integer first.
    if let Ok(n) = trimmed.parse::<i64>() {
        // Heuristic: values >= 10^10 are already milliseconds; smaller ones are seconds.
        return if n >= 10_000_000_000 {
            Ok(n)
        } else {
            Ok(n * 1000)
        };
    }

    // Normalise the offset separator: "2026-03-25T13:00:00 +00:00" → "…+00:00"
    let normalised = trimmed.replacen(" +", "+", 1).replacen(" -", "-", 1);

    // Try RFC 3339 / ISO 8601 with timezone.
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&normalised) {
        return Ok(dt.timestamp_millis());
    }

    // Try common formats without timezone (assumed UTC).
    for fmt in &[
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d",
    ] {
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(&normalised, fmt) {
            return Ok(ndt.and_utc().timestamp_millis());
        }
        // NaiveDate for date-only fallback
        if let Ok(nd) = chrono::NaiveDate::parse_from_str(&normalised, fmt) {
            return Ok(nd.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis());
        }
    }

    Err(format!(
        "Cannot parse '{}' as a timestamp. \
        Use a Unix timestamp (ms or s) or an ISO 8601 date/datetime string.",
        input
    ))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct AddWorklogParams {
    /// Issue ID or key to log work against (e.g., 'PROJ-123')
    issue_id_or_key: String,
    /// Time spent in Jira duration format (e.g., '1h 30m', '2h', '30m')
    time_spent: String,
    /// Date and time when work started, in ISO 8601 format (e.g., '2024-01-15T09:00:00.000+0000'). Defaults to now if not provided.
    started: Option<String>,
    /// Optional comment describing the work done
    comment: Option<String>,
    /// How to adjust the remaining estimate: 'auto' (default, reduces by time_spent), 'leave' (unchanged), 'new' (set to new_estimate), 'manual' (reduce by reduce_by)
    adjust_estimate: Option<String>,
    /// New remaining estimate when adjust_estimate is 'new' (e.g., '2h')
    new_estimate: Option<String>,
    /// Amount to reduce the remaining estimate by when adjust_estimate is 'manual' (e.g., '1h')
    reduce_by: Option<String>,
}
use crate::server::jira::Jira;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GetWorklogsParams {
    /// Start of the time range. Accepts a Unix timestamp in milliseconds (e.g. 1700000000000),
    /// a Unix timestamp in seconds (e.g. 1700000000), or an ISO 8601 datetime string
    /// (e.g. '2026-03-25T13:00:00+00:00', '2026-03-25T13:00:00 +00:00', '2026-03-25').
    since: String,
    /// End of the time range (optional). Same formats accepted as for 'since'.
    until: Option<String>,
    /// Filter results to worklogs authored by this user (matched against username or display name, case-insensitive).
    user: Option<String>,
    /// Filter results to worklogs authored by any of these users (matched against username or display name, case-insensitive).
    users: Option<Vec<String>>,
    /// When true, returns a JSON array of worklog objects instead of plain text.
    structured: Option<bool>,
}

#[tool_router(router = worklog_tool_router, vis = "pub(crate)")]
impl Jira {
    #[tool(
        description = "Get all worklogs updated since a given Unix timestamp in milliseconds \
        (e.g., 1700000000000). Paginates automatically until all results are collected. \
        Returns worklog details including author, time spent, and dates."
    )]
    async fn jira_get_worklogs(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<GetWorklogsParams>,
    ) -> String {
        let client = self.get_jira_client(&ctx);

        let since_ms = match parse_timestamp_ms(&params.since) {
            Ok(ms) => ms,
            Err(e) => return e,
        };
        let until_ms: Option<i64> = match params.until.as_deref() {
            Some(s) => match parse_timestamp_ms(s) {
                Ok(ms) => Some(ms),
                Err(e) => return e,
            },
            None => None,
        };

        // Paginate through all changed worklog IDs. Each page's `until` timestamp
        // becomes the `since` for the next request, until `last_page` is true.
        let mut all_ids: Vec<i64> = Vec::new();
        let mut since = since_ms;

        loop {
            let page = match client.get_ids_of_worklogs_modified_since(Some(since)).await {
                Ok(p) => p,
                Err(e) => return self.jira_client_error_response(&e),
            };

            let ids: Vec<i64> = page
                .values
                .unwrap_or_default()
                .into_iter()
                .filter_map(|w| w.worklog_id)
                .collect();
            all_ids.extend(ids);

            // Both `last_page` and `is_last_page` appear in the response; treat either as done.
            let is_last = page.last_page.unwrap_or(false)
                || page.is_last_page.unwrap_or(false);

            if is_last {
                break;
            }

            match page.until {
                Some(next_since) => {
                    // Stop if the next page would start beyond the requested until boundary.
                    if let Some(until) = until_ms {
                        if next_since >= until {
                            break;
                        }
                    }
                    since = next_since;
                }
                None => break, // no next cursor — treat as last page
            }
        }

        if all_ids.is_empty() {
            return format!("No worklogs updated since {}.", params.since);
        }

        // Fetch full worklog details in chunks of 1000 (API limit per request).
        let mut worklogs: Vec<Worklog> = Vec::new();

        for chunk in all_ids.chunks(1000) {
            let request = WorklogIdsRequestBean { ids: Some(chunk.to_vec()) };
            match client.get_worklogs_for_ids(request).await {
                Ok(chunk_worklogs) => worklogs.extend(chunk_worklogs),
                Err(e) => return self.jira_client_error_response(&e),
            }
        }

        if worklogs.is_empty() {
            return format!("No worklog details returned for {} IDs.", all_ids.len());
        }

        // Build a combined, lowercased set of author filters from both `user` and `users`.
        let author_filter: Vec<String> = {
            let mut names: Vec<String> = Vec::new();
            if let Some(u) = &params.user {
                names.push(u.to_lowercase());
            }
            if let Some(us) = &params.users {
                names.extend(us.iter().map(|u| u.to_lowercase()));
            }
            names
        };

        let worklogs: Vec<&Worklog> = if author_filter.is_empty() {
            worklogs.iter().collect()
        } else {
            worklogs.iter().filter(|w| {
                w.author.as_ref().map(|a| {
                    let name = a.name.as_deref().unwrap_or("").to_lowercase();
                    let display = a.display_name.as_deref().unwrap_or("").to_lowercase();
                    author_filter.iter().any(|f| name == *f || display == *f)
                }).unwrap_or(false)
            }).collect()
        };

        if worklogs.is_empty() {
            return "No worklogs matched the specified author filter.".to_string();
        }

        if params.structured.unwrap_or(false) {
            let items: Vec<serde_json::Value> = worklogs.iter().map(|w| worklog_to_json(w)).collect();
            serde_json::Value::Array(items).to_string()
        } else {
            worklogs.iter().map(|w| format_worklog(w)).collect::<Vec<_>>().join("\n---\n")
        }
    }

    #[tool(description = "Log work on a Jira issue. time_spent uses Jira duration format (e.g. '1h 30m'). started is an ISO 8601 datetime; defaults to now if omitted.")]
    async fn jira_add_worklog(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<AddWorklogParams>,
    ) -> String {
        let request = Worklog {
            time_spent: Some(params.time_spent),
            started: params.started,
            comment: params.comment,
            author: None,
            created: None,
            id: None,
            issue_id: None,
            self_: None,
            time_spent_seconds: None,
            update_author: None,
            updated: None,
            visibility: None,
        };

        match self
            .get_jira_client(&ctx)
            .add_worklog(
                &params.issue_id_or_key,
                params.new_estimate.as_deref(),
                params.adjust_estimate.as_deref(),
                params.reduce_by.as_deref(),
                request,
            )
            .await
        {
            Ok(w) => {
                let id = w.id.unwrap_or_default();
                let time_spent = w.time_spent.unwrap_or_default();
                let started = w.started.unwrap_or_default();
                format!(
                    "Worklog added successfully.\nID: {}\nTime Spent: {}\nStarted: {}",
                    id, time_spent, started
                )
            }
            Err(e) => self.jira_client_error_response(&e),
        }
    }
}

fn worklog_to_json(worklog: &Worklog) -> serde_json::Value {
    serde_json::json!({
        "id": worklog.id,
        "issue_id": worklog.issue_id,
        "author": worklog.author.as_ref().and_then(|a| a.display_name.as_deref()),
        "time_spent": worklog.time_spent,
        "time_spent_seconds": worklog.time_spent_seconds,
        "started": worklog.started,
        "created": worklog.created,
        "updated": worklog.updated,
        "comment": worklog.comment,
    })
}

fn format_worklog(worklog: &Worklog) -> String {
    let mut out = String::new();

    if let Some(id) = &worklog.id {
        out.push_str(&format!("ID: {}\n", id));
    }
    if let Some(issue_id) = &worklog.issue_id {
        out.push_str(&format!("Issue ID: {}\n", issue_id));
    }
    if let Some(author) = &worklog.author {
        let name = author.display_name.as_deref().unwrap_or("Unknown");
        out.push_str(&format!("Author: {}\n", name));
    }
    if let Some(time_spent) = &worklog.time_spent {
        out.push_str(&format!("Time Spent: {}\n", time_spent));
    }
    if let Some(seconds) = worklog.time_spent_seconds {
        out.push_str(&format!("Time Spent (seconds): {}\n", seconds));
    }
    if let Some(started) = &worklog.started {
        out.push_str(&format!("Started: {}\n", started));
    }
    if let Some(created) = &worklog.created {
        out.push_str(&format!("Created: {}\n", created));
    }
    if let Some(updated) = &worklog.updated {
        out.push_str(&format!("Updated: {}\n", updated));
    }
    if let Some(comment) = &worklog.comment {
        if !comment.is_empty() {
            out.push_str(&format!("Comment: {}\n", comment));
        }
    }

    out
}
