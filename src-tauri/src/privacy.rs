use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PrivacySettings {
    pub protection_enabled: bool,
    pub capture_text: bool,
    pub capture_images: bool,
    pub block_sensitive_text: bool,
    pub updated_at: String,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            protection_enabled: true,
            capture_text: true,
            capture_images: true,
            block_sensitive_text: true,
            updated_at: String::new(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PrivacyStatus {
    pub settings: PrivacySettings,
    pub blocked_capture_count: i64,
    pub last_blocked_at: Option<String>,
}

pub enum TextCaptureDecision {
    Allow,
    Skip,
    Block(String),
}

pub fn text_capture_decision(settings: &PrivacySettings, content: &str) -> TextCaptureDecision {
    if !settings.capture_text {
        return TextCaptureDecision::Skip;
    }
    if !settings.protection_enabled || !settings.block_sensitive_text {
        return TextCaptureDecision::Allow;
    }

    sensitive_reason(content)
        .map(TextCaptureDecision::Block)
        .unwrap_or(TextCaptureDecision::Allow)
}

pub fn sensitive_reason(content: &str) -> Option<String> {
    let lower = content.to_lowercase();
    if lower.contains("-----begin") && lower.contains("private key-----") {
        return Some("Private key material detected".to_string());
    }
    if contains_aws_access_key(content) {
        return Some("Cloud access key pattern detected".to_string());
    }
    if lower.contains("github_pat_")
        || lower.contains("ghp_")
        || lower.contains("gho_")
        || lower.contains("ghs_")
    {
        return Some("Source-control token pattern detected".to_string());
    }
    if contains_long_prefixed_token(content, "sk-", 24) {
        return Some("API key pattern detected".to_string());
    }
    if contains_bearer_token(&lower) {
        return Some("Authorization token pattern detected".to_string());
    }
    if contains_jwt(content) {
        return Some("JWT token pattern detected".to_string());
    }
    if contains_sensitive_assignment(content) {
        return Some("Sensitive configuration value detected".to_string());
    }
    None
}

fn contains_aws_access_key(content: &str) -> bool {
    let bytes = content.as_bytes();
    bytes.windows(20).any(|candidate| {
        matches!(candidate.get(0..4), Some(prefix) if prefix == b"AKIA" || prefix == b"ASIA")
            && candidate[4..]
                .iter()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
    })
}

fn contains_long_prefixed_token(content: &str, prefix: &str, minimum_length: usize) -> bool {
    content
        .split_whitespace()
        .any(|token| token.starts_with(prefix) && token.len() >= minimum_length)
}

fn contains_bearer_token(lower: &str) -> bool {
    lower
        .split_whitespace()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|pair| {
            pair[0].trim_matches(|character: char| character == ':' || character == '=') == "bearer"
                && pair[1]
                    .trim_matches(|character: char| {
                        !character.is_ascii_alphanumeric() && character != '-' && character != '_'
                    })
                    .len()
                    >= 16
        })
        || lower.contains("authorization: basic ")
}

fn contains_jwt(content: &str) -> bool {
    content.split_whitespace().any(|candidate| {
        let parts = candidate
            .trim_matches(|character: char| {
                !character.is_ascii_alphanumeric()
                    && character != '.'
                    && character != '-'
                    && character != '_'
            })
            .split('.')
            .collect::<Vec<_>>();
        parts.len() == 3
            && parts.iter().all(|part| {
                part.len() >= 8
                    && part
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
            })
    })
}

fn contains_sensitive_assignment(content: &str) -> bool {
    const KEYS: &[&str] = &[
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
        "apikey",
        "private_key",
        "access_key",
        "authorization",
    ];

    content.lines().any(|line| {
        let lower = line.to_lowercase();
        KEYS.iter().any(|key| {
            lower.find(key).is_some_and(|position| {
                let suffix = line[position + key.len()..].trim_start();
                if !suffix.starts_with('=') && !suffix.starts_with(':') {
                    return false;
                }
                let value = suffix[1..].trim();
                value.len() >= 8
                    && !matches!(
                        value.to_lowercase().as_str(),
                        "example" | "changeme" | "placeholder"
                    )
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{sensitive_reason, text_capture_decision, PrivacySettings, TextCaptureDecision};

    #[test]
    fn blocks_common_secret_patterns_without_recording_the_value() {
        assert!(sensitive_reason("Authorization: Bearer 1234567890abcdefgh").is_some());
        assert!(sensitive_reason("-----BEGIN PRIVATE KEY-----").is_some());
        assert!(sensitive_reason("api_key=very-long-local-secret-value").is_some());
    }

    #[test]
    fn allows_operational_commands_that_only_reference_secrets() {
        let decision = text_capture_decision(
            &PrivacySettings::default(),
            "kubectl create secret generic app-config --from-file=config.yaml",
        );
        assert!(matches!(decision, TextCaptureDecision::Allow));
    }
}
