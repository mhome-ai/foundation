//! Transport-independent ingress rules. Identity always comes from the authenticated host session.
use crate::{
    ErrorResponse, ExceptionCode, MwsMessage, MwsMessageType, ServiceCoreInput, ServiceCoreResponse,
};
use std::time::Duration;

pub const CORE_REQUEST_TIMEOUT: Duration = crate::timeouts::Operation::CoreRequest.timeout();
pub const IPC_TIMEOUT_GRACE: Duration =
    crate::timeouts::WaitProfile::HostIpc.grace(crate::timeouts::Operation::CoreRequest);
pub const SCRIPT_VALIDATION_TIMEOUT: Duration =
    crate::timeouts::Operation::ScriptValidation.timeout();
pub const SCRIPT_SAVE_TIMEOUT: Duration = crate::timeouts::Operation::ScriptSave.timeout();

/// Retains the existing ingress API; policy data and selectors have one shared source.
pub fn script_validation_timeout(target: &str, payload: &str) -> Option<Duration> {
    crate::timeouts::script_operation(target, payload).map(|operation| operation.timeout())
}

pub fn request_timeout(target: &str) -> Duration {
    crate::timeouts::ordinary_operation(target).timeout()
}

pub fn input_timeout(input: &ServiceCoreInput) -> Duration {
    match input {
        ServiceCoreInput::ClientRequest {
            target, payload, ..
        }
        | ServiceCoreInput::ConversationRequest {
            target, payload, ..
        }
        | ServiceCoreInput::NodeRequest {
            target, payload, ..
        } => script_validation_timeout(target, payload).unwrap_or_else(|| request_timeout(target)),
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
    #[test]
    fn rust_validation_budgets_follow_exact_routes_and_wire_envelopes() {
        let input =
            serde_json::json!({"script":{"script":"task: []","scripts":{"lighting":"source"}}});
        let call = serde_json::json!({"control":{"mode":"direct"},"input":input});
        for prefix in ["", "/invoke", "/dispatch", "/dispatch/agent"] {
            let payload = if prefix.is_empty() {
                call.clone()
            } else {
                serde_json::json!({"call":call})
            };
            assert_eq!(
                script_validation_timeout(
                    &format!("{prefix}/app/automation/verify"),
                    &payload.to_string()
                ),
                Some(SCRIPT_VALIDATION_TIMEOUT)
            );
        }
        for target in [
            "/bridge/automation/parse",
            "/bridge/automation/verify",
            "/bridge/automation/format",
            "/automation/format",
        ] {
            assert_eq!(
                script_validation_timeout(target, &input.to_string()),
                Some(SCRIPT_VALIDATION_TIMEOUT)
            );
        }
        for payload in [
            "invalid",
            "{}",
            r#"{"input":{"script":{"scripts":{}}}}"#,
            r#"{"input":{"script":{"scripts":[]}}}"#,
            r#"{"script":{"scripts":{"f":"source"}}}"#,
        ] {
            assert_eq!(
                script_validation_timeout("/app/automation/verify", payload),
                None
            );
        }
        for target in [
            "/app/automation/start",
            "/app/automation/verify/extra",
            "/app/room/list",
        ] {
            assert_eq!(script_validation_timeout(target, &call.to_string()), None);
        }
        let request = ServiceCoreInput::NodeRequest {
            target: "/app/automation/verify".into(),
            payload: call.to_string(),
            control: None,
            tenant_id: "tenant".into(),
            scope_id: "scope".into(),
            node_type: "test".into(),
            node_id: "node".into(),
        };
        assert_eq!(input_timeout(&request), SCRIPT_VALIDATION_TIMEOUT);
    }
    #[test]
    fn script_save_budgets_cover_build_install_and_stored_source_routes() {
        for prefix in ["", "/invoke", "/dispatch", "/dispatch/agent"] {
            let call = serde_json::json!({"input":{"script":{"scripts":{"f":"source"}}}});
            let wire = if prefix.is_empty() {
                call
            } else {
                serde_json::json!({"call":call})
            };
            assert_eq!(
                script_validation_timeout(
                    &format!("{prefix}/app/automation/save"),
                    &wire.to_string()
                ),
                Some(SCRIPT_SAVE_TIMEOUT)
            );
        }
        assert_eq!(
            script_validation_timeout(
                "/automation/save",
                r#"{"script":{"scripts":{"f":"source"}}}"#
            ),
            Some(SCRIPT_SAVE_TIMEOUT)
        );
        assert_eq!(
            script_validation_timeout("/automation/save", r#"{"script":{"scripts":{}}}"#),
            None
        );
        for target in [
            "/automation/snapshot/rollback",
            "/app/automation/snapshot/rollback",
            "/automation/overview/save",
        ] {
            assert_eq!(
                script_validation_timeout(target, "{}"),
                Some(SCRIPT_SAVE_TIMEOUT)
            );
        }
    }
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
            request_timeout("/app/plugin/camera/device/settings/update").as_secs(),
            60
        );
        assert_eq!(request_timeout("/harness/test/session/open").as_secs(), 150);
        assert!(IPC_TIMEOUT_GRACE > Duration::ZERO);
    }
}
