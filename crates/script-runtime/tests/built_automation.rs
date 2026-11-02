use mhome_script_api::{ScriptArtifact, ScriptValue as V};
use mhome_script_runtime::{
    Admission, CancellationToken, InvocationControl, NoHost, ScriptLimits, ScriptWorkers,
    WorkerConfig,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[tokio::test(flavor = "current_thread")]
async fn actual_typescript_build_executes_in_quickjs_with_its_declared_contract() {
    let metadata: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/lighting.build.json")).unwrap();
    let artifact: ScriptArtifact = serde_json::from_value(metadata["artifact"].clone()).unwrap();
    let contract = artifact.contract.unwrap();
    let object = |fields: Vec<(&str, V)>| {
        V::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
    };
    let delay = object(vec![
        ("_type", V::String("Duration".into())),
        ("unit", V::String("ms".into())),
        ("value", V::Number(0.125)),
    ]);
    let image = object(vec![(
        "uri",
        V::String("artifact://fictional-test/image".into()),
    )]);
    let settings = object(vec![("nested", object(vec![("exact", V::U64(u64::MAX))]))]);
    let input = object(vec![
        ("samples", V::List(vec![V::Number(2.0), V::Number(4.0)])),
        ("delay", delay.clone()),
        ("image", image.clone()),
        ("settings", settings.clone()),
    ]);
    contract.validate_input(&input).unwrap();
    let pool = ScriptWorkers::new(WorkerConfig {
        threads: 1,
        requests: 1,
        hooks: 1,
    })
    .unwrap();
    let result = pool
        .invoke(
            include_str!("../fixtures/lighting.js").into(),
            vec![input],
            Arc::new(NoHost),
            InvocationControl::new(
                Instant::now() + Duration::from_secs(2),
                CancellationToken::new(),
            ),
            ScriptLimits::default(),
            Admission::Request,
        )
        .await
        .unwrap();
    contract.validate_output(&result).unwrap();
    assert_eq!(
        result,
        object(vec![
            ("average", V::Number(3.0)),
            ("delay", delay),
            ("image", image),
            ("settings", settings)
        ])
    );
    pool.shutdown().await;
}

#[tokio::test(flavor = "current_thread")]
async fn javascript_number_and_bigint_outputs_remain_distinct_for_contract_validation() {
    use mhome_script_api::{Field, FieldType, ScalarType, ScriptContract};
    let mut contract = ScriptContract {
        version: 1,
        sdk_version: "0.1.0".into(),
        input: vec![],
        output: vec![Field {
            name: "n".into(),
            ty: FieldType::Scalar(ScalarType::Number),
        }],
    };
    for (source, valid_number) in [
        ("export default () => ({ n: 1 })", true),
        ("export default () => ({ n: 1n })", false),
        ("export default () => ({ n: 18446744073709551615n })", false),
    ] {
        let value = mhome_script_runtime::invoke(
            source,
            vec![],
            Arc::new(NoHost),
            InvocationControl::new(
                Instant::now() + Duration::from_secs(2),
                CancellationToken::new(),
            ),
            ScriptLimits::default(),
        )
        .await
        .unwrap();
        contract.output[0].ty = FieldType::Scalar(ScalarType::Number);
        assert_eq!(
            contract.validate_output(&value).is_ok(),
            valid_number,
            "{value:?}"
        );
        contract.output[0].ty = FieldType::Scalar(ScalarType::Value);
        contract.validate_output(&value).unwrap();
    }
}
