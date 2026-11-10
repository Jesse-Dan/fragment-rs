use crate::shared::config::PrivacyConfig;
use std::env;

pub trait Privacy {
    fn masked(&self) -> serde_json::Value;
}

pub struct FragmentPrivacyService;

impl FragmentPrivacyService {
    pub fn sanitize_for_response(value: &impl Privacy, _unmasked: &[&str]) -> serde_json::Value {
        value.masked()
    }

    pub fn sanitize_for_logging(value: &impl Privacy) -> serde_json::Value {
        value.masked()
    }

    pub fn mask_value(
        key: &str,
        value: &serde_json::Value,
        config: &PrivacyConfig,
    ) -> serde_json::Value {
        let mask_str = config.mask.as_deref().unwrap_or("[REDACTED]");

        // Get redact keys from environment variable or use sensible defaults
        let env_keys: Option<Vec<String>> = env::var("FRAGMENT_REDACT_KEYS").ok().map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|k| !k.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        });

        let redact_keys = env_keys.unwrap_or_else(|| {
            vec![
                "password".to_string(),
                "token".to_string(),
                "secret".to_string(),
                "ssn".to_string(),
                "credit_card".to_string(),
                "card_number".to_string(),
                "cvv".to_string(),
            ]
        });

        let key_lower = key.to_lowercase();
        if redact_keys.iter().any(|k| key_lower.contains(k)) {
            serde_json::Value::String(mask_str.to_string())
        } else {
            value.clone()
        }
    }

    pub fn sanitize_object(obj: &serde_json::Value, config: &PrivacyConfig) -> serde_json::Value {
        match obj {
            serde_json::Value::Object(map) => {
                let masked: serde_json::Map<String, serde_json::Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), Self::mask_value(k, v, config)))
                    .collect();
                serde_json::Value::Object(masked)
            }
            serde_json::Value::Array(arr) => serde_json::Value::Array(
                arr.iter()
                    .map(|v| Self::sanitize_object(v, config))
                    .collect(),
            ),
            _ => obj.clone(),
        }
    }
}
