use serde::{Deserialize, Serialize};

pub const DEFAULT_RETENTION_DAYS: i64 = 30;
pub const DEFAULT_MAX_STORAGE_MB: i64 = 512;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InsightTrailSettings {
    pub enabled: bool,
    pub capture_clipboard: bool,
    pub capture_terminal_history: bool,
    pub capture_copied_images: bool,
    pub create_incidents: bool,
    pub retention_days: i64,
    pub max_storage_mb: i64,
    pub excluded_applications: Vec<String>,
    pub updated_at: String,
}

impl Default for InsightTrailSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            capture_clipboard: true,
            capture_terminal_history: true,
            capture_copied_images: true,
            create_incidents: true,
            retention_days: DEFAULT_RETENTION_DAYS,
            max_storage_mb: DEFAULT_MAX_STORAGE_MB,
            excluded_applications: Vec::new(),
            updated_at: String::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct InsightTrailSearchRequest {
    pub query: String,
    pub event_type: String,
    pub limit: i64,
}

#[derive(Debug, Deserialize)]
pub struct InsightTrailNoteRequest {
    pub title: String,
    pub details: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct InsightTrailEvent {
    pub id: i64,
    pub event_type: String,
    pub title: String,
    pub details: String,
    pub source_application: String,
    pub severity: String,
    pub created_at: String,
    pub memory_id: Option<i64>,
    pub screenshot_path: Option<String>,
    pub incident_id: Option<i64>,
    pub session_id: Option<i64>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct InsightIncident {
    pub id: i64,
    pub title: String,
    pub status: String,
    pub summary: String,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub event_count: i64,
    pub recommended_steps: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct InsightTrailOverview {
    pub event_count: i64,
    pub active_incident_count: i64,
    pub screenshot_count: i64,
    pub error_signal_count: i64,
    pub capture_state: String,
    pub retention_days: i64,
}

#[derive(Debug, Clone)]
pub struct NewInsightTrailEvent {
    pub event_type: String,
    pub title: String,
    pub details: String,
    pub source_application: String,
    pub severity: String,
    pub memory_id: Option<i64>,
    pub screenshot_path: Option<String>,
    pub tags: Vec<String>,
    pub incident_signature: Option<String>,
    pub recommended_steps: Vec<String>,
}

pub fn event_from_memory(
    memory_id: i64,
    content: &str,
    content_type: &str,
    source_application: &str,
    operational_kind: &str,
    summary: &str,
    tags: &[String],
) -> NewInsightTrailEvent {
    let error_signal = is_error_signal(content, operational_kind);
    let is_terminal = source_application.starts_with("Terminal History")
        || operational_kind == "Terminal Command";
    let is_screenshot = content_type == "Image";
    let event_type = if is_screenshot {
        "Screenshot"
    } else if error_signal {
        "Error"
    } else if is_terminal {
        "Terminal"
    } else {
        "Clipboard"
    };
    let severity = if event_type == "Error" {
        if contains_any(content, &["panic", "fatal", "critical"]) {
            "Critical"
        } else {
            "Warning"
        }
    } else {
        "Info"
    };
    let title = match event_type {
        "Screenshot" => "Copied image added to timeline",
        "Error" => "Operational error signal",
        "Terminal" => "Terminal command recorded",
        _ => "Clipboard event recorded",
    };
    let details = non_empty_summary(summary, event_type);
    let clean_tags = clean_tags(tags);
    let incident_signature =
        (event_type == "Error").then(|| incident_signature(content, &clean_tags));
    let recommended_steps = incident_signature
        .as_deref()
        .map(troubleshooting_steps)
        .unwrap_or_default();

    NewInsightTrailEvent {
        event_type: event_type.to_string(),
        title: title.to_string(),
        details,
        source_application: source_application.to_string(),
        severity: severity.to_string(),
        memory_id: Some(memory_id),
        screenshot_path: is_screenshot.then(|| content.to_string()),
        tags: clean_tags,
        incident_signature,
        recommended_steps,
    }
}

pub fn manual_note(title: &str, details: &str, tags: &[String]) -> NewInsightTrailEvent {
    NewInsightTrailEvent {
        event_type: "Note".to_string(),
        title: title.trim().to_string(),
        details: details.trim().to_string(),
        source_application: "CYMOS".to_string(),
        severity: "Info".to_string(),
        memory_id: None,
        screenshot_path: None,
        tags: clean_tags(tags),
        incident_signature: None,
        recommended_steps: Vec::new(),
    }
}

pub fn settings_allow(settings: &InsightTrailSettings, event: &NewInsightTrailEvent) -> bool {
    if !settings.enabled {
        return false;
    }

    let source = event.source_application.to_lowercase();
    if settings
        .excluded_applications
        .iter()
        .map(|application| application.trim().to_lowercase())
        .filter(|application| !application.is_empty())
        .any(|application| source.contains(&application))
    {
        return false;
    }

    match event.event_type.as_str() {
        "Screenshot" => settings.capture_copied_images,
        "Terminal" => settings.capture_terminal_history,
        "Clipboard" | "Error" => settings.capture_clipboard,
        _ => true,
    }
}

pub fn clean_tags(tags: &[String]) -> Vec<String> {
    let mut clean = tags
        .iter()
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(|tag| tag.chars().take(64).collect::<String>())
        .collect::<Vec<_>>();
    clean.sort();
    clean.dedup();
    clean.truncate(12);
    clean
}

pub fn is_error_signal(content: &str, operational_kind: &str) -> bool {
    operational_kind == "Incident"
        || contains_any(
            content,
            &[
                "permission denied",
                "connection refused",
                "timed out",
                "timeout",
                "traceback",
                "exception",
                "fatal",
                "panic",
                "build failed",
                "command failed",
                "error:",
            ],
        )
}

fn incident_signature(content: &str, tags: &[String]) -> String {
    let error_class = if contains_any(content, &["permission denied", "access denied", "selinux"]) {
        "permission"
    } else if contains_any(content, &["connection refused", "connection reset"]) {
        "connection"
    } else if contains_any(content, &["timed out", "timeout"]) {
        "timeout"
    } else if contains_any(content, &["not found", "no such file", "command not found"]) {
        "not-found"
    } else if contains_any(content, &["panic", "fatal", "traceback", "exception"]) {
        "runtime"
    } else {
        "failure"
    };
    let normalized_tags = tags
        .iter()
        .map(|tag| tag.to_lowercase())
        .collect::<Vec<_>>();
    let domain = [
        "docker",
        "kubernetes",
        "nginx",
        "apache",
        "postgresql",
        "mysql",
        "systemd",
        "linux",
        "network",
    ]
    .iter()
    .find(|domain| normalized_tags.iter().any(|tag| tag == **domain))
    .map(|domain| (*domain).to_string())
    .unwrap_or_else(|| "operations".to_string());
    format!("{domain}:{error_class}")
}

fn troubleshooting_steps(signature: &str) -> Vec<String> {
    let mut steps = vec![
        "Review the linked memory and the events immediately before the signal.".to_string(),
        "Compare the current context with the most recent successful workflow.".to_string(),
    ];

    if signature.ends_with(":permission") {
        steps.push(
            "Verify the file owner, mode, service account, and SELinux context before retrying."
                .to_string(),
        );
    } else if signature.ends_with(":connection") {
        steps.push("Confirm the target service is running, listening, and reachable from the current host.".to_string());
    } else if signature.ends_with(":timeout") {
        steps.push("Check service health, network reachability, and recent load before increasing timeouts.".to_string());
    } else if signature.ends_with(":not-found") {
        steps.push(
            "Verify the command, path, package, and active environment match the intended system."
                .to_string(),
        );
    } else {
        steps.push("Preserve the exact error context and validate the affected configuration before retrying.".to_string());
    }

    steps
}

fn non_empty_summary(summary: &str, event_type: &str) -> String {
    let cleaned = summary.trim();
    if cleaned.is_empty() {
        format!("Local {event_type} event stored in InsightTrail.")
    } else {
        cleaned.chars().take(500).collect()
    }
}

fn contains_any(content: &str, values: &[&str]) -> bool {
    let lower = content.to_lowercase();
    values.iter().any(|value| lower.contains(value))
}

#[cfg(test)]
mod tests {
    use super::{event_from_memory, settings_allow, InsightTrailSettings};

    #[test]
    fn creates_an_incident_for_a_permission_error() {
        let event = event_from_memory(
            1,
            "nginx: permission denied by SELinux",
            "Text",
            "Terminal",
            "Incident",
            "Nginx access was denied.",
            &["Nginx".to_string(), "Linux".to_string()],
        );

        assert_eq!(event.event_type, "Error");
        assert_eq!(
            event.incident_signature.as_deref(),
            Some("nginx:permission")
        );
        assert_eq!(event.recommended_steps.len(), 3);
    }

    #[test]
    fn respects_copied_image_capture_setting() {
        let settings = InsightTrailSettings {
            capture_copied_images: false,
            ..InsightTrailSettings::default()
        };
        let event = event_from_memory(
            1,
            "/tmp/capture.png",
            "Image",
            "Unknown",
            "",
            "Image memory",
            &[],
        );

        assert!(!settings_allow(&settings, &event));
    }
}
