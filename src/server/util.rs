use jira_api::types::IssueBean;

pub(crate) fn render_adf(node: &serde_json::Value) -> String {
    if node.is_null() {
        return String::new();
    }
    let mut out = String::new();
    render_adf_node(node, &mut out, 0, "");
    out.trim().to_string()
}

fn render_adf_node(node: &serde_json::Value, out: &mut String, depth: usize, list_prefix: &str) {
    if node.is_null() {
        return;
    }

    let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("");

    let children = |out: &mut String, depth: usize, prefix: &str| {
        if let Some(arr) = node.get("content").and_then(|v| v.as_array()) {
            for child in arr {
                render_adf_node(child, out, depth, prefix);
            }
        }
    };

    match node_type {
        "doc" => children(out, depth, list_prefix),

        "paragraph" => {
            children(out, depth, list_prefix);
            out.push_str("\n\n");
        }

        "text" => {
            let raw = node.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let mut text = raw.to_string();
            if let Some(marks) = node.get("marks").and_then(|v| v.as_array()) {
                for mark in marks {
                    text = match mark.get("type").and_then(|v| v.as_str()).unwrap_or("") {
                        "strong"    => format!("**{}**", text),
                        "em"        => format!("*{}*", text),
                        "code"      => format!("`{}`", text),
                        "strike"    => format!("~~{}~~", text),
                        "underline" => format!("__{}__", text),
                        _           => text,
                    };
                }
            }
            out.push_str(&text);
        }

        "hardBreak" => out.push('\n'),

        "heading" => {
            let level = node.get("attrs")
                .and_then(|a| a.get("level"))
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as usize;
            out.push_str(&"#".repeat(level));
            out.push(' ');
            children(out, depth, list_prefix);
            out.push_str("\n\n");
        }

        "bulletList" => {
            if let Some(arr) = node.get("content").and_then(|v| v.as_array()) {
                for child in arr {
                    render_adf_node(child, out, depth, "- ");
                }
            }
        }

        "orderedList" => {
            if let Some(arr) = node.get("content").and_then(|v| v.as_array()) {
                for (i, child) in arr.iter().enumerate() {
                    let prefix = format!("{}. ", i + 1);
                    render_adf_node(child, out, depth, &prefix);
                }
            }
        }

        "listItem" => {
            if !list_prefix.is_empty() {
                out.push_str(&"  ".repeat(depth));
                out.push_str(list_prefix);
            }
            if let Some(arr) = node.get("content").and_then(|v| v.as_array()) {
                for child in arr {
                    render_adf_node(child, out, depth + 1, "");
                }
            }
        }

        "codeBlock" => {
            let language = node.get("attrs")
                .and_then(|a| a.get("language"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            out.push_str(&format!("```{}\n", language));
            children(out, depth, list_prefix);
            out.push_str("```\n\n");
        }

        "blockquote" => {
            let mut inner = String::new();
            if let Some(arr) = node.get("content").and_then(|v| v.as_array()) {
                for child in arr {
                    render_adf_node(child, &mut inner, depth, list_prefix);
                }
            }
            for line in inner.trim().split('\n') {
                out.push_str(&format!("> {}\n", line));
            }
            out.push('\n');
        }

        "rule" => out.push_str("---\n\n"),

        "table" => {
            out.push_str("\n[Table Content]\n");
            children(out, depth, list_prefix);
            out.push('\n');
        }

        "tableRow" => {
            out.push_str("| ");
            if let Some(arr) = node.get("content").and_then(|v| v.as_array()) {
                for child in arr {
                    render_adf_node(child, out, depth, list_prefix);
                    out.push_str(" | ");
                }
            }
            out.push('\n');
        }

        "tableHeader" | "tableCell" => children(out, depth, list_prefix),

        "mediaSingle" | "mediaGroup" => children(out, depth, list_prefix),

        "media" => {
            match node.get("attrs") {
                None => out.push_str("[Media/Image]"),
                Some(attrs) => {
                    let media_id  = attrs.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let media_type = attrs.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    let alt       = attrs.get("alt").and_then(|v| v.as_str()).unwrap_or("");

                    if !alt.is_empty() {
                        out.push_str(&format!("[Media: {}", alt));
                    } else if !media_type.is_empty() {
                        out.push_str(&format!("[Media: {}", media_type));
                    } else {
                        out.push_str("[Media");
                    }

                    if let (Some(w), Some(h)) = (
                        attrs.get("width").and_then(|v| v.as_f64()),
                        attrs.get("height").and_then(|v| v.as_f64()),
                    ) {
                        out.push_str(&format!(" ({}x{})", w as i64, h as i64));
                    }

                    if !media_id.is_empty() {
                        out.push_str(&format!(" | id={}", media_id));
                    }
                    out.push(']');
                }
            }
        }

        "mention" => {
            if let Some(text) = node.get("attrs")
                .and_then(|a| a.get("text"))
                .and_then(|v| v.as_str())
            {
                out.push_str(&format!("@{}", text));
            }
        }

        "emoji" => {
            if let Some(short_name) = node.get("attrs")
                .and_then(|a| a.get("shortName"))
                .and_then(|v| v.as_str())
            {
                out.push_str(short_name);
            }
        }

        "inlineCard" => {
            if let Some(url) = node.get("attrs")
                .and_then(|a| a.get("url"))
                .and_then(|v| v.as_str())
            {
                out.push_str(url);
            }
        }

        _ => children(out, depth, list_prefix),
    }
}

pub(crate) fn format_user(user: &serde_json::Value) -> String {
    let display_name = user.get("displayName").and_then(|v| v.as_str()).unwrap_or("");
    let email = user.get("emailAddress").and_then(|v| v.as_str()).unwrap_or("");
    if email.is_empty() {
        display_name.to_string()
    } else {
        format!("{} ({})", display_name, email)
    }
}

pub(crate) fn format_issue(issue: &IssueBean) -> String {
    let mut out = String::new();

    if let Some(key) = &issue.key {
        out.push_str(&format!("Key: {}\n", key));
    }
    if let Some(id) = &issue.id {
        out.push_str(&format!("ID: {}\n", id));
    }
    if let Some(url) = &issue.self_ {
        out.push_str(&format!("URL: {}\n", url));
    }

    if let Some(fields) = issue.fields.as_ref().map(|f| &f.additional_properties) {
        // Summary
        if let Some(summary) = fields.get("summary").and_then(|v| v.as_str()) {
            if !summary.is_empty() {
                out.push_str(&format!("Summary: {}\n", summary));
            }
        }

        // Description: DataCenter returns a plain string (wiki markup); Cloud returns an ADF object.
        match fields.get("description") {
            Some(serde_json::Value::String(s)) => {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    out.push_str(&format!("Description:\n{}\n", trimmed));
                }
            }
            Some(desc @ serde_json::Value::Object(_)) => {
                let rendered = render_adf(desc);
                if !rendered.is_empty() {
                    out.push_str(&format!("Description:\n{}\n", rendered));
                }
            }
            _ => {}
        }

        // Issue Type
        if let Some(issue_type) = fields.get("issuetype").and_then(|v| v.as_object()) {
            if let Some(name) = issue_type.get("name").and_then(|v| v.as_str()) {
                out.push_str(&format!("Type: {}\n", name));
            }
            if let Some(desc) = issue_type.get("description").and_then(|v| v.as_str()) {
                if !desc.is_empty() {
                    out.push_str(&format!("Type Description: {}\n", desc));
                }
            }
        }

        // Status
        if let Some(status) = fields.get("status").and_then(|v| v.as_object()) {
            if let Some(name) = status.get("name").and_then(|v| v.as_str()) {
                out.push_str(&format!("Status: {}\n", name));
            }
            if let Some(desc) = status.get("description").and_then(|v| v.as_str()) {
                if !desc.is_empty() {
                    out.push_str(&format!("Status Description: {}\n", desc));
                }
            }
        }

        // Priority
        match fields.get("priority") {
            Some(p) if !p.is_null() => {
                if let Some(name) = p.get("name").and_then(|v| v.as_str()) {
                    out.push_str(&format!("Priority: {}\n", name));
                }
            }
            _ => out.push_str("Priority: None\n"),
        }

        // Resolution
        if let Some(resolution) = fields.get("resolution").and_then(|v| v.as_object()) {
            if let Some(name) = resolution.get("name").and_then(|v| v.as_str()) {
                out.push_str(&format!("Resolution: {}\n", name));
                if let Some(desc) = resolution.get("description").and_then(|v| v.as_str()) {
                    if !desc.is_empty() {
                        out.push_str(&format!("Resolution Description: {}\n", desc));
                    }
                }
            }
        }

        // Resolution Date
        if let Some(date) = fields.get("resolutiondate").and_then(|v| v.as_str()) {
            if !date.is_empty() {
                out.push_str(&format!("Resolution Date: {}\n", date));
            }
        }

        // Reporter
        match fields.get("reporter") {
            Some(r) if !r.is_null() => out.push_str(&format!("Reporter: {}\n", format_user(r))),
            _ => out.push_str("Reporter: Unassigned\n"),
        }

        // Assignee
        match fields.get("assignee") {
            Some(a) if !a.is_null() => out.push_str(&format!("Assignee: {}\n", format_user(a))),
            _ => out.push_str("Assignee: Unassigned\n"),
        }

        // Creator
        if let Some(creator) = fields.get("creator") {
            if !creator.is_null() {
                out.push_str(&format!("Creator: {}\n", format_user(creator)));
            }
        }

        // Dates
        for (key, label) in [
            ("created", "Created"),
            ("updated", "Updated"),
            ("lastViewed", "Last Viewed"),
            ("statuscategorychangedate", "Status Category Change Date"),
        ] {
            if let Some(val) = fields.get(key).and_then(|v| v.as_str()) {
                if !val.is_empty() {
                    out.push_str(&format!("{}: {}\n", label, val));
                }
            }
        }

        // Project
        if let Some(project) = fields.get("project").and_then(|v| v.as_object()) {
            if let Some(name) = project.get("name").and_then(|v| v.as_str()) {
                out.push_str(&format!("Project: {}", name));
                if let Some(key) = project.get("key").and_then(|v| v.as_str()) {
                    out.push_str(&format!(" ({})", key));
                }
                out.push('\n');
            }
        }

        // Parent
        if let Some(parent) = fields.get("parent").and_then(|v| v.as_object()) {
            if let Some(key) = parent.get("key").and_then(|v| v.as_str()) {
                out.push_str(&format!("Parent: {}", key));
                if let Some(summary) = parent
                    .get("fields").and_then(|v| v.as_object())
                    .and_then(|f| f.get("summary"))
                    .and_then(|v| v.as_str())
                {
                    if !summary.is_empty() {
                        out.push_str(&format!(" - {}", summary));
                    }
                }
                out.push('\n');
            }
        }

        // Work Ratio
        if let Some(wr) = fields.get("workratio").and_then(|v| v.as_i64()) {
            if wr > 0 {
                out.push_str(&format!("Work Ratio: {}\n", wr));
            }
        }

        // Labels
        if let Some(labels) = fields.get("labels").and_then(|v| v.as_array()) {
            let label_strs: Vec<&str> = labels.iter().filter_map(|v| v.as_str()).collect();
            if !label_strs.is_empty() {
                out.push_str(&format!("Labels: {}\n", label_strs.join(", ")));
            }
        }

        // Components
        if let Some(components) = fields.get("components").and_then(|v| v.as_array()) {
            if !components.is_empty() {
                out.push_str("Components:\n");
                for comp in components {
                    if let Some(name) = comp.get("name").and_then(|v| v.as_str()) {
                        out.push_str(&format!("- {}", name));
                        if let Some(desc) = comp.get("description").and_then(|v| v.as_str()) {
                            if !desc.is_empty() {
                                out.push_str(&format!(" ({})", desc));
                            }
                        }
                        out.push('\n');
                    }
                }
            }
        }

        // Fix Versions
        if let Some(fix_versions) = fields.get("fixVersions").and_then(|v| v.as_array()) {
            if !fix_versions.is_empty() {
                out.push_str("Fix Versions:\n");
                for v in fix_versions {
                    if let Some(name) = v.get("name").and_then(|v| v.as_str()) {
                        out.push_str(&format!("- {}", name));
                        if let Some(desc) = v.get("description").and_then(|v| v.as_str()) {
                            if !desc.is_empty() {
                                out.push_str(&format!(" ({})", desc));
                            }
                        }
                        out.push('\n');
                    }
                }
            }
        }

        // Affected Versions
        if let Some(versions) = fields.get("versions").and_then(|v| v.as_array()) {
            if !versions.is_empty() {
                out.push_str("Affected Versions:\n");
                for v in versions {
                    if let Some(name) = v.get("name").and_then(|v| v.as_str()) {
                        out.push_str(&format!("- {}", name));
                        if let Some(desc) = v.get("description").and_then(|v| v.as_str()) {
                            if !desc.is_empty() {
                                out.push_str(&format!(" ({})", desc));
                            }
                        }
                        out.push('\n');
                    }
                }
            }
        }

        // Security Level
        if let Some(security) = fields.get("security").and_then(|v| v.as_object()) {
            if let Some(name) = security.get("name").and_then(|v| v.as_str()) {
                out.push_str(&format!("Security Level: {}\n", name));
            }
        }

        // Subtasks
        if let Some(subtasks) = fields.get("subtasks").and_then(|v| v.as_array()) {
            if !subtasks.is_empty() {
                out.push_str("Subtasks:\n");
                for subtask in subtasks {
                    if let Some(key) = subtask.get("key").and_then(|v| v.as_str()) {
                        out.push_str(&format!("- {}", key));
                        if let Some(f) = subtask.get("fields").and_then(|v| v.as_object()) {
                            if let Some(summary) = f.get("summary").and_then(|v| v.as_str()) {
                                if !summary.is_empty() {
                                    out.push_str(&format!(": {}", summary));
                                }
                            }
                            if let Some(status_name) = f
                                .get("status").and_then(|v| v.as_object())
                                .and_then(|s| s.get("name"))
                                .and_then(|v| v.as_str())
                            {
                                out.push_str(&format!(" [{}]", status_name));
                            }
                        }
                        out.push('\n');
                    }
                }
            }
        }

        // Issue Links
        if let Some(links) = fields.get("issuelinks").and_then(|v| v.as_array()) {
            if !links.is_empty() {
                out.push_str("Issue Links:\n");
                for link in links {
                    let link_type = link.get("type").and_then(|v| v.as_object());

                    if let Some(outward) = link.get("outwardIssue").and_then(|v| v.as_object()) {
                        let label = link_type
                            .and_then(|t| t.get("outward"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("outward");
                        if let Some(key) = outward.get("key").and_then(|v| v.as_str()) {
                            out.push_str(&format!("- {} {}", label, key));
                            if let Some(summary) = outward
                                .get("fields").and_then(|v| v.as_object())
                                .and_then(|f| f.get("summary"))
                                .and_then(|v| v.as_str())
                            {
                                if !summary.is_empty() {
                                    out.push_str(&format!(": {}", summary));
                                }
                            }
                            out.push('\n');
                        }
                    }

                    if let Some(inward) = link.get("inwardIssue").and_then(|v| v.as_object()) {
                        let label = link_type
                            .and_then(|t| t.get("inward"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("inward");
                        if let Some(key) = inward.get("key").and_then(|v| v.as_str()) {
                            out.push_str(&format!("- {} {}", label, key));
                            if let Some(summary) = inward
                                .get("fields").and_then(|v| v.as_object())
                                .and_then(|f| f.get("summary"))
                                .and_then(|v| v.as_str())
                            {
                                if !summary.is_empty() {
                                    out.push_str(&format!(": {}", summary));
                                }
                            }
                            out.push('\n');
                        }
                    }
                }
            }
        }

        // Watchers
        if let Some(watches) = fields.get("watches").and_then(|v| v.as_object()) {
            if let Some(count) = watches.get("watchCount").and_then(|v| v.as_i64()) {
                out.push_str(&format!("Watchers: {}\n", count));
            }
        }

        // Votes
        if let Some(votes_obj) = fields.get("votes").and_then(|v| v.as_object()) {
            if let Some(votes) = votes_obj.get("votes").and_then(|v| v.as_i64()) {
                out.push_str(&format!("Votes: {}\n", votes));
            }
        }

        // Attachments
        if let Some(attachments) = fields.get("attachment").and_then(|v| v.as_array()) {
            if !attachments.is_empty() {
                out.push_str("Attachments:\n");
                for att in attachments {
                    let name = att.get("filename")
                        .or_else(|| att.get("title"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let id = att.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let mime = att.get("mimeType").and_then(|v| v.as_str()).unwrap_or("");
                    let size = att.get("size").and_then(|v| v.as_i64()).unwrap_or(0);
                    out.push_str(&format!("- {} (ID: {}, Type: {}, Size: {} bytes)\n", name, id, mime, size));
                }
            }
        }

        // Comments
        if let Some(comment_obj) = fields.get("comment").and_then(|v| v.as_object()) {
            if let Some(total) = comment_obj.get("total").and_then(|v| v.as_i64()) {
                if total > 0 {
                    out.push_str(&format!("Comments: {} total\n", total));
                }
            }
        }

        // Worklogs
        if let Some(worklog_obj) = fields.get("worklog").and_then(|v| v.as_object()) {
            if let Some(total) = worklog_obj.get("total").and_then(|v| v.as_i64()) {
                if total > 0 {
                    out.push_str(&format!("Worklogs: {} entries\n", total));
                }
            }
        }
    }

    // Available Transitions
    if let Some(transitions) = &issue.transitions {
        if !transitions.is_empty() {
            out.push_str("\nAvailable Transitions:\n");
            for t in transitions {
                if let (Some(name), Some(id)) = (&t.name, &t.id) {
                    out.push_str(&format!("- {} (ID: {})\n", name, id));
                }
            }
        }
    }

    // Story point estimate from changelog (last value wins)
    if let Some(changelog) = &issue.changelog {
        if let Some(histories) = &changelog.histories {
            let mut story_point = String::new();
            for history in histories {
                if let Some(items) = &history.items {
                    for item in items {
                        if item.field.as_deref() == Some("Story point estimate") {
                            if let Some(val) = &item.to_string {
                                if !val.is_empty() {
                                    story_point = val.clone();
                                }
                            }
                        }
                    }
                }
            }
            if !story_point.is_empty() {
                out.push_str(&format!("Story Point Estimate: {}\n", story_point));
            }
        }
    }

    out
}
