use crate::model::{Availability, Entity};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedactionPolicy {
    #[serde(default)]
    pub literal_secrets: Vec<String>,
}

impl RedactionPolicy {
    pub fn redact(&self, text: &str) -> Result<String, regex::Error> {
        let mut output = text.to_owned();
        let mut secrets: Vec<_> = self
            .literal_secrets
            .iter()
            .filter(|s| !s.is_empty())
            .collect();
        secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
        for secret in secrets {
            output = output.replace(secret, "[REDACTED]");
        }
        static PATTERNS: std::sync::OnceLock<Result<Vec<regex::Regex>, regex::Error>> =
            std::sync::OnceLock::new();
        let patterns = PATTERNS.get_or_init(|| [
            r"(?i)(?:Bearer\s+)[A-Za-z0-9._~+/=-]+",
            r"\b(?:sk-[A-Za-z0-9_-]{12,}|gh[pousr]_[A-Za-z0-9_]{12,}|github_pat_[A-Za-z0-9_]{12,}|AKIA[A-Z0-9]{16})",
            r"(?i)(?:api[_-]?key|access[_-]?token|password|secret)\s*[:=]\s*[^\s,;]+",
            r"https?://[^\s/@]+:[^\s/@]+@",
        ].into_iter().map(regex::Regex::new).collect());
        for pattern in patterns.as_ref().map_err(Clone::clone)? {
            output = pattern.replace_all(&output, "[REDACTED]").into_owned();
        }
        Ok(output)
    }

    /// Apply one policy to every textual surface, including identifiers and locators.
    pub fn entity(&self, entity: &Entity) -> Result<Entity, crate::store::Error> {
        fn walk(
            value: &mut serde_json::Value,
            policy: &RedactionPolicy,
        ) -> Result<(), crate::store::Error> {
            match value {
                serde_json::Value::String(s) => *s = policy.redact(s)?,
                serde_json::Value::Array(xs) => {
                    for x in xs {
                        walk(x, policy)?;
                    }
                }
                serde_json::Value::Object(xs) => {
                    for x in xs.values_mut() {
                        walk(x, policy)?;
                    }
                }
                _ => {}
            }
            Ok(())
        }
        let mut value = serde_json::to_value(entity)?;
        walk(&mut value, self)?;
        let mut sanitized: Entity = serde_json::from_value(value)?;
        if let (Entity::Record(before), Entity::Record(after)) = (entity, &mut sanitized)
            && (before.body != after.body || before.title != after.title)
            && after.availability == Availability::Available
        {
            after.availability = Availability::Redacted;
        }
        Ok(sanitized)
    }
}
