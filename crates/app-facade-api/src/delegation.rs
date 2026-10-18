//! Private authenticated cloud-to-Core envelope, never accepted from Client input.
use crate::FacadeCall;
use core_api::client_identity::ClientPrincipal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum DelegatedAuthority {
    Caller { principal: ClientPrincipal },
    Service {},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DelegatedCall {
    pub authority: DelegatedAuthority,
    pub call: FacadeCall,
}
