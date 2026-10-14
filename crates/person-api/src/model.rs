use serde::{Deserialize, Serialize};
/// Immutable identity of comparable embeddings. Changing preprocessing requires a new id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmbeddingSpace {
    pub id: String,
    pub dimensions: u32,
    pub recognizer_sha256: String,
    pub preprocessing: String,
    pub normalized: bool,
    pub metric: String,
}
impl EmbeddingSpace {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.is_empty()
            || self.id.len() > 256
            || self.dimensions == 0
            || self.dimensions as usize > crate::MAX_VECTOR_DIMENSIONS
            || self.recognizer_sha256.len() != 64
            || !self
                .recognizer_sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
            || self.preprocessing.is_empty()
            || self.preprocessing.len() > 1024
            || !self.normalized
            || self.metric != "cosine"
        {
            return Err("unsupported embedding contract");
        }
        Ok(())
    }
}
/// Naming state is independent of recognition certainty and never part of an identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PersonKind {
    Known,
    Unknown,
}

/// Canonical Space-owned reference. Display labels and naming state may change.
pub fn valid_person_id(value: &str) -> bool {
    let Some(id) = value.strip_prefix("h:") else {
        return false;
    };
    id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Person {
    pub id: String,
    pub name: String,
    pub kind: PersonKind,
    pub short_code: String,
    pub revision: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}
/// A user-facing face group; retained recognition evidence stays private to Core.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FaceCluster {
    pub id: String,
    pub person_id: String,
    pub name: String,
    pub kind: PersonKind,
    pub revision: String,
    pub representative_sample_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonStatus {
    pub epoch: String,
    pub revision: String,
    pub person_count: u32,
    pub cluster_count: u32,
    pub sample_count: u32,
    pub image_bytes: u64,
    pub image_limit_bytes: u64,
    pub unknown_retention_days: u32,
    pub models: Vec<EmbeddingSpace>,
}

/// Full base-type value shared by catalogs and current-state capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PersonValue {
    pub id: String,
    pub uid: String,
    pub display_name: String,
    pub kind: PersonKind,
}
impl From<Person> for PersonValue {
    fn from(person: Person) -> Self {
        Self {
            uid: person.id.clone(),
            id: person.id,
            display_name: person.name,
            kind: person.kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_format_is_independent_of_name_and_kind() {
        let id = "h:12345678-1234-4abc-8abc-123456789abc";
        assert!(valid_person_id(id));
        for invalid in [
            "h:AC2F",
            "p:12345678-1234-4abc-8abc-123456789abc",
            "h:12345678-1234-4ABC-8abc-123456789abc",
            "h:12345678-1234-4abc-8abc-123456789abc-extra",
        ] {
            assert!(!valid_person_id(invalid), "{invalid}");
        }
        let mut person = Person {
            id: id.into(),
            name: "Stranger AC2F".into(),
            short_code: "AC2F".into(),
            kind: PersonKind::Unknown,
            revision: "1".into(),
            created_at_ms: 1,
            updated_at_ms: 1,
        };
        let unknown = PersonValue::from(person.clone());
        person.name = "Alice".into();
        person.kind = PersonKind::Known;
        let known = PersonValue::from(person);
        assert_eq!(unknown.id, known.id);
        assert_eq!(known.id, known.uid);
        let value = serde_json::to_value(known).unwrap();
        assert_eq!(value["displayName"], "Alice");
        assert_eq!(value["kind"], "known");
        assert!(value.get("shortCode").is_none());
    }
}
