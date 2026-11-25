use conversation_api::execution::{
    ExecutionContext, SuggestionInvocationContext, SuggestionPayer, SuggestionTrigger,
};
use serde_json::json;

fn context() -> SuggestionInvocationContext {
    serde_json::from_value(json!({
        "space":{"tenant_id":"tenant-1","space_id":"space-1"},
        "runId":"run-1","operationId":"operation-1", "deadlineUnixMs":5000,
        "trigger":{"type":"SCHEDULED"}
    }))
    .unwrap()
}

#[test]
fn scheduled_payment_resolves_current_owner_without_impersonation() {
    let context = context();
    assert_eq!(
        context.payer().unwrap(),
        SuggestionPayer::CurrentSpaceOwner(context.space.clone())
    );
    let value = serde_json::to_value(context).unwrap();
    for field in [
        "actor",
        "userId",
        "clientId",
        "credentialId",
        "threadId",
        "surfaceId",
        "payer",
    ] {
        assert!(value.get(field).is_none(), "{field}");
    }
}

#[test]
fn explicit_payment_belongs_to_the_authenticated_requesting_user() {
    let mut context = context();
    context.trigger = SuggestionTrigger::Explicit {
        user_id: "member-b".into(),
    };
    assert_eq!(
        context.payer().unwrap(),
        SuggestionPayer::User("member-b".into())
    );
    let child = context.with_operation_id("operation-2".into());
    assert_eq!(child.payer(), context.payer());
    assert_eq!(child.space(), context.space());
    assert_eq!(child.run_id(), context.run_id());
    assert_eq!(child.operation_id().as_str(), "operation-2");
    assert_eq!(context.operation_id().as_str(), "operation-1");
}

#[test]
fn invalid_authority_cannot_resolve_a_payer() {
    let mut c = context();
    c.space.tenant_id = None;
    assert!(c.payer().is_err());
    let mut c = context();
    c.space.space_id = " ".into();
    assert!(c.payer().is_err());
    let mut c = context();
    c.deadline_unix_ms = 0;
    assert!(c.validate().is_err());
    let mut c = context();
    c.trigger = SuggestionTrigger::Explicit { user_id: "".into() };
    assert!(c.payer().is_err());
}

#[test]
fn scheduled_trigger_cannot_smuggle_a_user_or_client_identity() {
    for trigger in [
        json!({"type":"SCHEDULED","userId":"forged"}),
        json!({"type":"EXPLICIT"}),
        json!({"type":"EXPLICIT","userId":"member-b","payer":"owner"}),
    ] {
        let mut value = serde_json::to_value(context()).unwrap();
        value["trigger"] = trigger;
        assert!(serde_json::from_value::<SuggestionInvocationContext>(value).is_err());
    }
}

#[test]
fn execution_schema_and_types_agree_on_admission_shapes() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../schema/execution/suggestion.v1.schema.json"
    ))
    .unwrap();
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/suggestion/conformance.json")).unwrap();
    let validator = jsonschema::validator_for(
        &json!({"$defs":schema["$defs"],"$ref":"#/$defs/SuggestionInvocationContext"}),
    )
    .unwrap();
    for (group, valid) in [("valid", true), ("invalid", false)] {
        for case in corpus[group].as_array().unwrap() {
            assert_eq!(validator.is_valid(&case["value"]), valid, "schema {case}");
            let actual =
                serde_json::from_value::<SuggestionInvocationContext>(case["value"].clone())
                    .is_ok_and(|v| v.validate().is_ok());
            assert_eq!(actual, valid, "typed {case}");
        }
    }
}
