use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    Storage {
        operation: String,
        detail: String,
        retryable: bool,
    },
}

impl AppError {
    pub fn storage(operation: impl Into<String>, detail: impl Into<String>) -> Self {
        let detail = detail.into();
        Self::Storage {
            operation: operation.into(),
            retryable: is_transient_storage_error(&detail),
            detail,
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::Storage {
                operation,
                retryable,
                ..
            } if *retryable => format!("{operation} is temporarily unavailable. Please retry."),
            Self::Storage { operation, .. } => {
                format!("{operation} could not be completed. Please retry.")
            }
        }
    }

    pub fn log(&self) {
        match self {
            Self::Storage {
                operation,
                detail,
                retryable,
            } => eprintln!(
                "{}",
                json!({
                    "event": "cymos.command.failure",
                    "category": "storage",
                    "operation": operation,
                    "retryable": retryable,
                    "detail": detail,
                })
            ),
        }
    }
}

pub fn is_transient_storage_error(detail: &str) -> bool {
    let normalized = detail.to_lowercase();
    normalized.contains("database is locked")
        || normalized.contains("database is busy")
        || normalized.contains("timed out")
        || normalized.contains("temporarily unavailable")
}

#[cfg(test)]
mod tests {
    use super::{is_transient_storage_error, AppError};

    #[test]
    fn marks_sqlite_lock_contention_as_retryable() {
        let error = AppError::storage("Runbook update", "database is locked");
        assert_eq!(
            error.user_message(),
            "Runbook update is temporarily unavailable. Please retry."
        );
        assert!(is_transient_storage_error("database is busy"));
    }

    #[test]
    fn keeps_non_transient_storage_failures_generic() {
        let error = AppError::storage("Runbook update", "constraint violation");
        assert_eq!(
            error.user_message(),
            "Runbook update could not be completed. Please retry."
        );
        assert!(!is_transient_storage_error("constraint violation"));
    }
}
