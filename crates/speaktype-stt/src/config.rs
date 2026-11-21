use serde::{Deserialize, Serialize};

/// How transcription is served: the bundled local whisper-server, a
/// user-configured whisper.cpp server, or a cloud service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProviderConfig {
    #[serde(rename_all = "camelCase")]
    Local { model: String },
    #[serde(rename_all = "camelCase")]
    Server { base_url: String },
    #[serde(rename_all = "camelCase")]
    Cloud {
        service: CloudService,
        base_url: String,
        model: String,
        fallback_to_local: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum CloudService {
    OpenAi,
    Groq,
    Deepgram,
    Custom,
}

impl CloudService {
    /// Pre-filled address for known services; `None` for Custom.
    pub fn default_base_url(&self) -> Option<&'static str> {
        match self {
            CloudService::OpenAi => Some("https://api.openai.com/v1"),
            CloudService::Groq => Some("https://api.groq.com/openai/v1"),
            CloudService::Deepgram => Some("https://api.deepgram.com/v1"),
            CloudService::Custom => None,
        }
    }

    /// Keychain account holding this service's API key.
    pub fn secret_account(&self) -> &'static str {
        match self {
            CloudService::OpenAi => "stt.openai",
            CloudService::Groq => "stt.groq",
            CloudService::Deepgram => "stt.deepgram",
            CloudService::Custom => "stt.custom",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_variants_round_trip_through_json() {
        let local = ProviderConfig::Local {
            model: "base".to_string(),
        };
        let json = serde_json::to_string(&local).unwrap();
        assert_eq!(json, r#"{"kind":"local","model":"base"}"#);
        assert_eq!(
            serde_json::from_str::<ProviderConfig>(&json).unwrap(),
            local
        );

        let server = ProviderConfig::Server {
            base_url: "http://127.0.0.1:9000/inference".to_string(),
        };
        let json = serde_json::to_string(&server).unwrap();
        assert_eq!(
            json,
            r#"{"kind":"server","baseUrl":"http://127.0.0.1:9000/inference"}"#
        );
        assert_eq!(
            serde_json::from_str::<ProviderConfig>(&json).unwrap(),
            server
        );

        let cloud = ProviderConfig::Cloud {
            service: CloudService::Groq,
            base_url: "https://api.groq.com/openai/v1".to_string(),
            model: "whisper-large-v3".to_string(),
            fallback_to_local: true,
        };
        let json = serde_json::to_string(&cloud).unwrap();
        assert_eq!(
            json,
            r#"{"kind":"cloud","service":"groq","baseUrl":"https://api.groq.com/openai/v1","model":"whisper-large-v3","fallbackToLocal":true}"#
        );
        assert_eq!(
            serde_json::from_str::<ProviderConfig>(&json).unwrap(),
            cloud
        );
    }

    #[test]
    fn default_base_url_holds_the_plan_values() {
        assert_eq!(
            CloudService::OpenAi.default_base_url(),
            Some("https://api.openai.com/v1")
        );
        assert_eq!(
            CloudService::Groq.default_base_url(),
            Some("https://api.groq.com/openai/v1")
        );
        assert_eq!(
            CloudService::Deepgram.default_base_url(),
            Some("https://api.deepgram.com/v1")
        );
        assert_eq!(CloudService::Custom.default_base_url(), None);
    }

    #[test]
    fn cloud_service_names_round_trip() {
        for (service, name) in [
            (CloudService::OpenAi, "openai"),
            (CloudService::Groq, "groq"),
            (CloudService::Deepgram, "deepgram"),
            (CloudService::Custom, "custom"),
        ] {
            let json = serde_json::to_string(&service).unwrap();
            assert_eq!(json, format!(r#""{name}""#));
            assert_eq!(
                serde_json::from_str::<CloudService>(&json).unwrap(),
                service
            );
        }
    }

    #[test]
    fn cloud_service_secret_accounts() {
        assert_eq!(CloudService::OpenAi.secret_account(), "stt.openai");
        assert_eq!(CloudService::Groq.secret_account(), "stt.groq");
        assert_eq!(CloudService::Deepgram.secret_account(), "stt.deepgram");
        assert_eq!(CloudService::Custom.secret_account(), "stt.custom");
    }
}
