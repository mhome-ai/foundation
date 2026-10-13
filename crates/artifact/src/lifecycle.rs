use serde::{Deserialize, Serialize};

/// Retention and ownership policy; part of resource identity, never the content digest.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactLifecycle {
    #[default]
    Std,
    Tmp,
    Man,
}

impl ArtifactLifecycle {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Std => "std",
            Self::Tmp => "tmp",
            Self::Man => "man",
        }
    }

    pub const fn is_temporary(self) -> bool {
        matches!(self, Self::Tmp)
    }

    pub fn from_code(value: &str) -> Result<Self, crate::ArtifactReferenceError> {
        match value {
            "std" => Ok(Self::Std),
            "tmp" => Ok(Self::Tmp),
            "man" => Ok(Self::Man),
            _ => Err(crate::invalid("unsupported artifact lifecycle")),
        }
    }

    /// Manual callers may reserve a 128-bit creation ID for retry/recovery before uploading.
    pub fn validate_creation_id(
        self,
        id: Option<&str>,
    ) -> Result<(), crate::ArtifactReferenceError> {
        if let Some(id) = id {
            if self != Self::Man
                || id.len() != 32
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            {
                return Err(crate::invalid(
                    "creation ID must be 32 lowercase hex characters and requires man lifecycle",
                ));
            }
        }
        Ok(())
    }
}
