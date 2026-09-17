//! Strongly typed identifiers shared by contracts.

use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! string_id {
    ($name:ident) => {
        #[doc = concat!("Stable identifier for `", stringify!($name), "` values.")]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates an identifier from its canonical string representation.
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Returns the canonical string representation.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

string_id!(TenantId);
string_id!(ScopeId);
string_id!(UserId);
string_id!(ClientId);
string_id!(ThreadId);
string_id!(RunId);
string_id!(OperationId);
string_id!(ActionId);
string_id!(EventId);
string_id!(MessageId);
