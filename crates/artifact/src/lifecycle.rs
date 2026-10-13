use serde::{Deserialize, Serialize};

/// Retention and resource identity policy; never part of the content digest.
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

    /// Manual identities are supplied and retained verbatim by callers.
    pub fn validate_creation_id(
        self,
        id: Option<&str>,
    ) -> Result<(), crate::ArtifactReferenceError> {
        match (self, id) {
            (Self::Man, Some(id))
                if !id.is_empty()
                    && id.len() <= 128
                    && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_')) =>
            {
                Ok(())
            }
            (Self::Man, _) => Err(crate::invalid(
                "man lifecycle requires a caller-supplied creation ID of 1-128 ASCII letters, digits, hyphens or underscores",
            )),
            (_, None) => Ok(()),
            (_, Some(_)) => Err(crate::invalid("creation ID requires man lifecycle")),
        }
    }
}
