//! Transport-independent ingress rules. Identity always comes from the authenticated host session.
use crate::{
    ErrorResponse, ExceptionCode, MwsMessage, MwsMessageType, ServiceCoreInput, ServiceCoreResponse,
};
use std::time::Duration;

pub const CORE_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
pub const IPC_TIMEOUT_GRACE: Duration = Duration::from_secs(5);

pub fn request_timeout(target: &str) -> Duration {
    match target {
        "/harness/test/session/open" => Duration::from_secs(150),
        _ => CORE_REQUEST_TIMEOUT,
    }
}

pub fn input_timeout(input: &ServiceCoreInput) -> Duration {
    match input {
        ServiceCoreInput::ClientRequest { target, .. }
        | ServiceCoreInput::ConversationRequest { target, .. }
        | ServiceCoreInput::NodeRequest { target, .. } => request_timeout(target),
        _ => CORE_REQUEST_TIMEOUT,
    }
}

pub struct NodeSession<'a> {
    pub connection_key: &'a str,
    pub tenant_id: &'a str,
    pub scope_id: &'a str,
    pub node_type: &'a str,
    pub node_id: &'a str,
}

pub fn node_input(
    request: &MwsMessage,
    session: NodeSession<'_>,
) -> Result<ServiceCoreInput, ErrorResponse> {
    let target = request.target.as_deref().unwrap_or_default();
    if target.is_empty() {
        return Err(ErrorResponse::new(
            ExceptionCode::BadRequest,
            "target is required",
        ));
    }
    let payload = request
        .payload
        .as_deref()
        .filter(|p| !p.is_empty())
        .unwrap_or("{}");
    if target == crate::node::status::STATUS_CHANGED_TARGET {
        let status = serde_json::from_str(payload).map_err(|error| {
            ErrorResponse::new(
                ExceptionCode::BadRequest,
                format!("invalid Node status payload: {error}"),
            )
        })?;
        return Ok(ServiceCoreInput::NodeStatusObserved {
            connection_key: session.connection_key.into(),
            status: Box::new(status),
        });
    }
    Ok(ServiceCoreInput::NodeRequest {
        target: target.into(),
        payload: payload.into(),
        control: request.control.clone(),
        tenant_id: session.tenant_id.into(),
        scope_id: session.scope_id.into(),
        node_type: session.node_type.into(),
        node_id: session.node_id.into(),
    })
}

/// Apply at the outer boundary, including authorization and validation failures.
pub fn node_reply<T>(message_type: &str, response: Option<T>) -> Option<T> {
    if message_type == MwsMessageType::NODE_DATA {
        None
    } else {
        response
    }
}

pub fn node_response(
    request: &MwsMessage,
    scope_id: &str,
    result: Result<Option<ServiceCoreResponse>, ErrorResponse>,
) -> Option<MwsMessage> {
    let result = result.and_then(|response| match response {
        Some(ServiceCoreResponse::NodeRequest { response_payload }) => {
            Ok(response_payload.unwrap_or_else(|| "{}".into()))
        }
        Some(ServiceCoreResponse::Ack { handled: true })
            if request.target.as_deref() == Some(crate::node::status::STATUS_CHANGED_TARGET) =>
        {
            Ok("{}".into())
        }
        _ => Err(ErrorResponse::new(
            ExceptionCode::Internal,
            "unexpected Core response for Node request",
        )),
    });
    let (payload, error) = match result {
        Ok(payload) => (Some(payload), None),
        Err(error) => (None, Some(error)),
    };
    node_reply(
        request.r#type.as_deref().unwrap_or_default(),
        Some(MwsMessage {
            scope_id: Some(scope_id.into()),
            from: None,
            target: request.target.clone(),
            sig: request.sig.clone(),
            r#type: Some(MwsMessageType::NODE_RESP.into()),
            payload,
            error,
            control: None,
            client_info: None,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(target: &str, kind: &str, payload: &str) -> MwsMessage {
        serde_json::from_value(serde_json::json!({"target":target,"type":kind,"payload":payload,"scopeId":"untrusted","sig":"s"})).unwrap()
    }
    fn session() -> NodeSession<'static> {
        NodeSession {
            connection_key: "connection",
            tenant_id: "tenant",
            scope_id: "scope",
            node_type: "camera",
            node_id: "node",
        }
    }
    #[test]
    fn regular_node_input_uses_authenticated_identity() {
        match node_input(&request("/x", MwsMessageType::NODE_REQ, ""), session()).unwrap() {
            ServiceCoreInput::NodeRequest {
                tenant_id,
                scope_id,
                node_id,
                payload,
                ..
            } => {
                assert_eq!(
                    (
                        tenant_id.as_str(),
                        scope_id.as_str(),
                        node_id.as_str(),
                        payload.as_str()
                    ),
                    ("tenant", "scope", "node", "{}")
                );
            }
            _ => panic!("wrong input"),
        }
    }
    #[test]
    fn invalid_status_is_not_forwarded_as_an_ordinary_request() {
        let error = node_input(
            &request(
                crate::node::status::STATUS_CHANGED_TARGET,
                MwsMessageType::NODE_DATA,
                "{}",
            ),
            session(),
        )
        .unwrap_err();
        assert_eq!(error.error, ExceptionCode::BadRequest.as_str());
    }
    #[test]
    fn status_uses_the_transport_connection_not_payload_identity() {
        let payload = serde_json::json!({
            "contract":crate::node::STATUS_CONTRACT,"serviceId":"camera","nodeType":"camera",
            "processGeneration":"process","revision":1,"process":"running","updatedAtMs":1,
            "instance":{"nodeId":"node","runtimeId":"r","hubId":"h","tenantId":"tenant","scopeId":"scope",
                "identity":"commissioned","hub":"resolved","connection":"connected","runtime":"ready",
                "runtimeGeneration":1,"connectionGeneration":1,"revision":1,"effective":"online","updatedAtMs":1},
            "runtime":{"runtimeId":"r","runtime":"ready","generation":1,"revision":1,"updatedAtMs":1},
            "connectionKey":"forged"
        });
        let request = request(
            crate::node::status::STATUS_CHANGED_TARGET,
            MwsMessageType::NODE_REQ,
            &payload.to_string(),
        );
        match node_input(&request, session()).unwrap() {
            ServiceCoreInput::NodeStatusObserved {
                connection_key,
                status,
            } => {
                assert_eq!(connection_key, "connection");
                assert_eq!(status.instance.node_id, "node");
            }
            _ => panic!("status must use the status manager input"),
        }
        let response = node_response(
            &request,
            "scope",
            Ok(Some(ServiceCoreResponse::Ack { handled: true })),
        )
        .unwrap();
        assert_eq!(response.payload.as_deref(), Some("{}"));
        assert!(response.error.is_none());
    }
    #[test]
    fn windows_pipe_names_are_bounded_and_workdir_specific() {
        use crate::external_core_pipe_name as name;
        use std::path::Path;
        assert_eq!(
            name(Path::new("C:/Meow/dev/core.sock")),
            name(Path::new(r"c:\meow\dev\core.sock"))
        );
        assert_ne!(
            name(Path::new("C:/Meow/dev/core.sock")),
            name(Path::new("C:/Meow/prod/core.sock"))
        );
        assert!(name(Path::new(&"a".repeat(5000))).len() < 256);
    }
    #[test]
    fn reports_never_reply_and_requests_preserve_error_details() {
        let error = ErrorResponse::new(ExceptionCode::PreconditionFail, "expired")
            .with_details(serde_json::json!({"preparedActionStatus":"expired"}));
        for target in ["", "/x", crate::node::status::STATUS_CHANGED_TARGET] {
            assert!(node_response(
                &request(target, MwsMessageType::NODE_DATA, "{}"),
                "scope",
                Err(error.clone())
            )
            .is_none());
        }
        let response = node_response(
            &request("/x", MwsMessageType::NODE_REQ, "{}"),
            "scope",
            Err(error.clone()),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(response.error.unwrap()).unwrap(),
            serde_json::to_value(error).unwrap()
        );
        assert!(node_reply(MwsMessageType::NODE_DATA, Some("unauthorized")).is_none());
    }
    #[test]
    fn ipc_budget_covers_core_budget_including_long_requests() {
        assert_eq!(
            request_timeout("/app/plugin/camera/watch/set").as_secs(),
            60
        );
        assert_eq!(request_timeout("/harness/test/session/open").as_secs(), 150);
        assert!(IPC_TIMEOUT_GRACE > Duration::ZERO);
    }
}
