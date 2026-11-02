use mhome_script_api::*;
use serde_json::{json, Value};

fn corpus() -> Value {
    serde_json::from_str(include_str!("../fixtures/javascript-v1.json")).unwrap()
}

#[test]
fn shared_contract_corpus() {
    let fixture = corpus();
    for value in fixture["validContracts"].as_array().unwrap() {
        let contract: ScriptContract = serde_json::from_value(value.clone()).unwrap();
        contract.validate().unwrap();
        assert_eq!(serde_json::to_value(contract).unwrap(), *value);
    }
    for value in fixture["invalidContracts"].as_array().unwrap() {
        let result = serde_json::from_value::<ScriptContract>(value.clone())
            .map_err(|e| e.to_string())
            .and_then(|contract| contract.validate().map_err(|e| e.to_string()));
        assert!(result.is_err(), "accepted invalid contract: {value}");
    }
}

#[test]
fn shared_reference_corpus() {
    let fixture = corpus();
    for value in fixture["validReferences"].as_array().unwrap() {
        let reference: ScriptRef = serde_json::from_value(value.clone()).unwrap();
        reference.validate().unwrap();
        assert_eq!(serde_json::to_value(reference).unwrap(), *value);
    }
    for value in fixture["invalidReferences"].as_array().unwrap() {
        let result = serde_json::from_value::<ScriptRef>(value.clone())
            .map_err(|e| e.to_string())
            .and_then(|reference| reference.validate().map_err(|e| e.to_string()));
        assert!(result.is_err(), "accepted invalid reference: {value}");
    }
}

#[test]
fn new_instruction_has_no_legacy_alias() {
    let fixture = corpus();
    assert_eq!(fixture["instruction"]["opcode"], SCRIPT_CALL_OPCODE);
    assert_eq!(
        fixture["instruction"]["operands"].as_array().unwrap().len(),
        SCRIPT_CALL_OPERANDS
    );
    for opcode in fixture["instruction"]["rejectedLegacyOpcodes"]
        .as_array()
        .unwrap()
    {
        assert_ne!(opcode.as_u64().unwrap(), u64::from(SCRIPT_CALL_OPCODE));
    }
}

#[test]
fn list_nesting_is_bounded_without_flattening() {
    let mut contract: ScriptContract =
        serde_json::from_value(corpus()["validContracts"][1].clone()).unwrap();
    let mut ty = FieldType::Scalar(ScalarType::Number);
    for _ in 0..MAX_CONTRACT_DEPTH {
        ty = FieldType::list(ty);
    }
    contract.input.push(Field {
        name: "items".into(),
        ty,
    });
    contract.validate().unwrap();
    contract.input[0].ty = FieldType::list(contract.input[0].ty.clone());
    assert!(contract.validate().is_err());
}

fn reference(index: usize) -> ScriptRef {
    serde_json::from_value(corpus()["validReferences"][index].clone()).unwrap()
}

fn artifact(reference: &ScriptRef) -> ScriptArtifact {
    ScriptArtifact {
        format: ArtifactFormat::JavascriptEsm,
        sha256: reference.sha256().into(),
        size: 512,
        sdk_version: "0.1.0".into(),
        contract: match reference {
            ScriptRef::Automation { .. } => {
                Some(serde_json::from_value(corpus()["validContracts"][0].clone()).unwrap())
            }
            ScriptRef::Integration { .. } => None,
        },
    }
}

#[test]
fn automation_cannot_execute_unbound_or_incompatible_metadata() {
    let reference = reference(1);
    let original = artifact(&reference);
    original.validate_for(&reference).unwrap();
    let mut changed = original.clone();
    changed.contract = None;
    assert!(changed.validate_for(&reference).is_err());
    let mut changed = original.clone();
    changed.contract.as_mut().unwrap().sdk_version = "0.2.0".into();
    assert!(changed.validate_for(&reference).is_err());
    let mut changed = original;
    changed.sha256 = "c".repeat(64);
    assert!(changed.validate_for(&reference).is_err());
}

#[test]
fn integration_authority_remains_in_role_schema() {
    let reference = reference(0);
    let mut artifact = artifact(&reference);
    artifact.validate_for(&reference).unwrap();
    artifact.contract =
        Some(serde_json::from_value(corpus()["validContracts"][0].clone()).unwrap());
    assert!(artifact.validate_for(&reference).is_err());
}

#[test]
fn artifact_size_is_bounded_and_wasm_format_is_rejected() {
    let reference = reference(0);
    let mut artifact = artifact(&reference);
    for size in [1, MAX_ARTIFACT_BYTES] {
        artifact.size = size;
        artifact.validate_for(&reference).unwrap();
    }
    for size in [0, MAX_ARTIFACT_BYTES + 1, usize::MAX] {
        artifact.size = size;
        assert!(artifact.validate_for(&reference).is_err());
    }
    let mut value = serde_json::to_value(artifact).unwrap();
    value["format"] = json!("wasm");
    assert!(serde_json::from_value::<ScriptArtifact>(value).is_err());
}

#[test]
fn internal_outcome_does_not_accept_both_success_and_failure() {
    let success: ScriptOutcome<Value> =
        serde_json::from_value(json!({"status":"success","data":{"ok":false}})).unwrap();
    assert!(matches!(success, ScriptOutcome::Success { .. }));
    for value in [
        json!({"ok":true,"data":{}}),
        json!({"status":"success","data":{},"error":{"code":"X","message":"x"}}),
        json!({"status":"failure","error":{"code":"X","message":"x"},"data":{}}),
    ] {
        assert!(serde_json::from_value::<ScriptOutcome<Value>>(value).is_err());
    }
}

#[test]
fn runtime_contract_requires_fields_lists_and_exact_number_compatibility() {
    let contract = ScriptContract {
        version: 1,
        sdk_version: "0.1.0".into(),
        input: vec![
            Field {
                name: "values".into(),
                ty: FieldType::list(FieldType::list(FieldType::Scalar(ScalarType::Number))),
            },
            Field {
                name: "dynamic".into(),
                ty: FieldType::Scalar(ScalarType::Value),
            },
        ],
        output: vec![],
    };
    let valid = ScriptValue::Object(
        [
            (
                "values".into(),
                ScriptValue::List(vec![ScriptValue::List(vec![ScriptValue::Number(1.5)])]),
            ),
            ("dynamic".into(), ScriptValue::U64(u64::MAX)),
        ]
        .into_iter()
        .collect(),
    );
    contract.validate_input(&valid).unwrap();
    let ScriptValue::Object(mut fields) = valid else {
        unreachable!()
    };
    fields.remove("dynamic");
    assert!(contract
        .validate_input(&ScriptValue::Object(fields.clone()))
        .is_err());
    fields.insert("dynamic".into(), ScriptValue::Null);
    for invalid in [
        ScriptValue::List(vec![ScriptValue::Number(1.0)]),
        ScriptValue::List(vec![ScriptValue::List(vec![ScriptValue::I64(i64::MAX)])]),
        ScriptValue::List(vec![ScriptValue::List(vec![ScriptValue::String(
            "1".into(),
        )])]),
    ] {
        fields.insert("values".into(), invalid);
        assert!(contract
            .validate_input(&ScriptValue::Object(fields.clone()))
            .is_err());
    }
    assert!(contract
        .validate_output(&ScriptValue::Object(Default::default()))
        .is_ok());
    assert!(contract
        .validate_output(&ScriptValue::List(vec![]))
        .is_err());
}

#[test]
fn runtime_domain_contracts_validate_units_and_integral_timestamps() {
    let mut contract = ScriptContract {
        version: 1,
        sdk_version: "0.1.0".into(),
        input: vec![],
        output: vec![Field {
            name: "result".into(),
            ty: FieldType::Scalar(ScalarType::Temperature),
        }],
    };
    let wrap = |value| ScriptValue::Object([("result".into(), value)].into_iter().collect());
    let quantity = |tag: &str, unit: &str, value| {
        ScriptValue::Object(
            [
                ("_type".into(), ScriptValue::String(tag.into())),
                ("unit".into(), ScriptValue::String(unit.into())),
                ("value".into(), ScriptValue::Number(value)),
            ]
            .into_iter()
            .collect(),
        )
    };
    assert!(contract
        .validate_output(&wrap(quantity("Temperature", "F", 72.123456789)))
        .is_ok());
    assert!(contract
        .validate_output(&wrap(quantity("Temperature", "°F", 72.0)))
        .is_err());
    contract.output[0].ty = FieldType::Scalar(ScalarType::Duration);
    assert!(contract
        .validate_output(&wrap(quantity("Duration", "ms", 0.125)))
        .is_ok());
    assert!(contract
        .validate_output(&wrap(quantity("Duration", "s", 1.0)))
        .is_err());
    contract.output[0].ty = FieldType::Scalar(ScalarType::Timestamp);
    for (value, expected) in [
        (ScriptValue::I64(i64::MIN), true),
        (ScriptValue::Number(0.5), false),
        (ScriptValue::U64(u64::MAX), false),
        (ScriptValue::Number(1.0), true),
    ] {
        let timestamp = ScriptValue::Object(
            [
                ("_type".into(), ScriptValue::String("Timestamp".into())),
                ("value".into(), value),
            ]
            .into_iter()
            .collect(),
        );
        assert_eq!(contract.validate_output(&wrap(timestamp)).is_ok(), expected);
    }
}
