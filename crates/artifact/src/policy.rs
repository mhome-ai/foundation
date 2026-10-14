use serde::{Deserialize, Serialize};

/// How callers distinguish resources containing the same bytes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactIdentity {
    #[default]
    Auto,
    Man,
}

impl ArtifactIdentity {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Man => "man",
        }
    }
    pub fn from_code(value: &str) -> Result<Self, crate::ArtifactReferenceError> {
        match value {
            "auto" => Ok(Self::Auto),
            "man" => Ok(Self::Man),
            _ => Err(crate::invalid("unsupported artifact identity")),
        }
    }
    /// Manual IDs are supplied and retained verbatim by callers.
    pub fn validate_creation_id(
        self,
        id: Option<&str>,
    ) -> Result<(), crate::ArtifactReferenceError> {
        match (self, id) {
            (Self::Man, Some(id)) if !id.is_empty() && id.len() <= 128
                && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_')) => Ok(()),
            (Self::Man, _) => Err(crate::invalid("man identity requires a caller-supplied creation ID of 1-128 ASCII letters, digits, hyphens or underscores")),
            (Self::Auto, None) => Ok(()),
            (Self::Auto, Some(_)) => Err(crate::invalid("creation ID requires man identity")),
        }
    }
}

/// Retention is independent of identity and physical storage technology.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactRetention {
    #[default]
    Std,
    Tmp,
}

impl ArtifactRetention {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Std => "std",
            Self::Tmp => "tmp",
        }
    }
    pub const fn is_temporary(self) -> bool {
        matches!(self, Self::Tmp)
    }
    pub fn from_code(value: &str) -> Result<Self, crate::ArtifactReferenceError> {
        match value {
            "std" => Ok(Self::Std),
            "tmp" => Ok(Self::Tmp),
            _ => Err(crate::invalid("unsupported artifact retention")),
        }
    }
}

/// Both independent policy dimensions participate in resource identity.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactPolicy {
    pub identity: ArtifactIdentity,
    pub retention: ArtifactRetention,
}

impl ArtifactPolicy {
    pub const AUTO_STD: Self = Self::new(ArtifactIdentity::Auto, ArtifactRetention::Std);
    pub const AUTO_TMP: Self = Self::new(ArtifactIdentity::Auto, ArtifactRetention::Tmp);
    pub const MAN_STD: Self = Self::new(ArtifactIdentity::Man, ArtifactRetention::Std);
    pub const MAN_TMP: Self = Self::new(ArtifactIdentity::Man, ArtifactRetention::Tmp);
    pub const fn new(identity: ArtifactIdentity, retention: ArtifactRetention) -> Self {
        Self {
            identity,
            retention,
        }
    }
    pub const fn is_temporary(self) -> bool {
        self.retention.is_temporary()
    }
    pub fn validate_creation_id(
        self,
        id: Option<&str>,
    ) -> Result<(), crate::ArtifactReferenceError> {
        self.identity.validate_creation_id(id)
    }
}
