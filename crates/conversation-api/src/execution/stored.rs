//! Fixed private persistence encoding. Network contracts use Space names directly.
//! These codecs preserve existing stored identities without accepting retired wire fields.
use super::*;
use crate::ClientPrincipal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(remote = "SpaceContext", deny_unknown_fields)]
struct StoredSpace {
    tenant_id: Option<TenantId>,
    #[serde(rename = "scope_id")]
    space_id: SpaceId,
}
#[derive(Serialize, Deserialize)]
#[serde(
    remote = "ClientPrincipal",
    tag = "kind",
    rename_all = "camelCase",
    deny_unknown_fields
)]
enum StoredPrincipal {
    User {
        #[serde(rename = "userId")]
        user_id: String,
    },
    Space {
        #[serde(rename = "tenantId")]
        tenant_id: String,
        #[serde(rename = "scopeId")]
        space_id: String,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(remote = "Actor", deny_unknown_fields)]
struct StoredActor {
    #[serde(with = "principal")]
    principal: ClientPrincipal,
    credential_id: Option<String>,
    client_id: Option<ClientId>,
}
#[derive(Serialize, Deserialize)]
#[serde(remote = "InvocationContext", deny_unknown_fields)]
struct StoredContext {
    #[serde(rename = "scope", with = "space")]
    space: SpaceContext,
    #[serde(with = "actor")]
    actor: Actor,
    surface_id: ConversationSurface,
    thread_id: ThreadId,
    run_id: RunId,
    operation_id: OperationId,
    deadline_unix_ms: Option<u64>,
    traceparent: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(remote = "QueuedRun")]
struct StoredQueuedRun {
    #[serde(with = "context")]
    context: InvocationContext,
    message: Message,
    options: RunOptions,
}
#[derive(Serialize, Deserialize)]
#[serde(remote = "DurableEvent")]
struct StoredEvent {
    id: EventId,
    sequence: u64,
    #[serde(with = "context")]
    context: InvocationContext,
    event: AgentEvent,
}
macro_rules! codec {
    ($module:ident, $value:ty, $stored:ident) => {
        pub mod $module {
            use super::*;
            pub fn serialize<S: serde::Serializer>(
                value: &$value,
                serializer: S,
            ) -> Result<S::Ok, S::Error> {
                $stored::serialize(value, serializer)
            }
            pub fn deserialize<'de, D: serde::Deserializer<'de>>(
                deserializer: D,
            ) -> Result<$value, D::Error> {
                $stored::deserialize(deserializer)
            }
            pub fn to_string(value: &$value) -> Result<String, serde_json::Error> {
                #[derive(Serialize)]
                struct Encoded<'a>(#[serde(serialize_with = "serialize")] &'a $value);
                serde_json::to_string(&Encoded(value))
            }
            pub fn from_str(value: &str) -> Result<$value, serde_json::Error> {
                #[derive(Deserialize)]
                struct Encoded(#[serde(deserialize_with = "deserialize")] $value);
                serde_json::from_str::<Encoded>(value).map(|encoded| encoded.0)
            }
        }
    };
}
codec!(space, SpaceContext, StoredSpace);
codec!(principal, ClientPrincipal, StoredPrincipal);
codec!(actor, Actor, StoredActor);
codec!(context, InvocationContext, StoredContext);
codec!(queued_run, QueuedRun, StoredQueuedRun);
codec!(event, DurableEvent, StoredEvent);

/// Optional outbox event encoded with the same fixed private record shape.
pub mod optional_event {
    use super::*;
    #[derive(Serialize, Deserialize)]
    struct Encoded(#[serde(with = "event")] DurableEvent);
    pub fn to_string(value: &Option<DurableEvent>) -> Result<String, serde_json::Error> {
        serde_json::to_string(&value.clone().map(Encoded))
    }
    pub fn from_str(value: &str) -> Result<Option<DurableEvent>, serde_json::Error> {
        serde_json::from_str::<Option<Encoded>>(value).map(|value| value.map(|event| event.0))
    }
}
