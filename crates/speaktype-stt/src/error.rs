/// One failed provider call, normalised so callers can retry, cancel, or show
/// copy without knowing which provider they talked to.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SttError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("rate limited (retry after {retry_after_ms:?} ms)")]
    RateLimited { retry_after_ms: Option<u64> },
    #[error("unreachable")]
    Unreachable,
    #[error("timeout")]
    Timeout,
    #[error("server error {status}")]
    Server { status: u16 },
    #[error("unreadable response: {0}")]
    BadResponse(String),
    #[error("cancelled")]
    Cancelled,
    #[error("not configured")]
    NotConfigured,
}

impl SttError {
    /// Plain-language copy for the user. `service_name` is the provider's
    /// display name ("Groq", "On this Mac", …).
    pub fn user_message(&self, service_name: &str) -> String {
        match self {
            SttError::Unauthorized => format!("{service_name} didn't accept the API key."),
            SttError::RateLimited { .. } => format!("{service_name} is busy. Trying again…"),
            SttError::Unreachable => format!("Couldn't reach {service_name}."),
            SttError::Timeout => format!("{service_name} took too long to answer."),
            SttError::Server { status } => {
                format!("{service_name} had a problem (error {status}).")
            }
            SttError::BadResponse(_) => {
                format!("{service_name} sent an answer SpeakType couldn't read.")
            }
            SttError::Cancelled => "Cancelled.".to_string(),
            SttError::NotConfigured => {
                "Choose a speech service in Settings → Providers.".to_string()
            }
        }
    }

    /// Whether the retry policy may try this call again.
    pub fn is_transient(&self) -> bool {
        match self {
            SttError::RateLimited { .. } | SttError::Unreachable | SttError::Timeout => true,
            SttError::Server { status } => *status >= 500,
            _ => false,
        }
    }

    /// The server's `Retry-After` in ms, when a 429 sent one as seconds.
    pub fn retry_after_ms(&self) -> Option<u64> {
        match self {
            SttError::RateLimited { retry_after_ms } => *retry_after_ms,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_transient_is_true_only_for_transient_kinds() {
        assert!(SttError::RateLimited {
            retry_after_ms: None
        }
        .is_transient());
        assert!(SttError::RateLimited {
            retry_after_ms: Some(1000)
        }
        .is_transient());
        assert!(SttError::Unreachable.is_transient());
        assert!(SttError::Timeout.is_transient());
        assert!(SttError::Server { status: 500 }.is_transient());
        assert!(SttError::Server { status: 503 }.is_transient());
        assert!(!SttError::Server { status: 404 }.is_transient());
        assert!(!SttError::Server { status: 429 }.is_transient());
        assert!(!SttError::Unauthorized.is_transient());
        assert!(!SttError::BadResponse("missing text".into()).is_transient());
        assert!(!SttError::Cancelled.is_transient());
        assert!(!SttError::NotConfigured.is_transient());
    }

    #[test]
    fn user_message_holds_the_exact_plan_copy() {
        assert_eq!(
            SttError::Unauthorized.user_message("Groq"),
            "Groq didn't accept the API key."
        );
        assert_eq!(
            SttError::RateLimited {
                retry_after_ms: None
            }
            .user_message("Groq"),
            "Groq is busy. Trying again…"
        );
        assert_eq!(
            SttError::RateLimited {
                retry_after_ms: Some(2000)
            }
            .user_message("Groq"),
            "Groq is busy. Trying again…"
        );
        assert_eq!(
            SttError::Unreachable.user_message("Groq"),
            "Couldn't reach Groq."
        );
        assert_eq!(
            SttError::Timeout.user_message("Groq"),
            "Groq took too long to answer."
        );
        assert_eq!(
            SttError::Server { status: 500 }.user_message("Groq"),
            "Groq had a problem (error 500)."
        );
        assert_eq!(
            SttError::BadResponse("missing text".into()).user_message("Groq"),
            "Groq sent an answer SpeakType couldn't read."
        );
        assert_eq!(
            SttError::NotConfigured.user_message("Groq"),
            "Choose a speech service in Settings → Providers."
        );
        assert_eq!(SttError::Cancelled.user_message("Groq"), "Cancelled.");
    }
}
