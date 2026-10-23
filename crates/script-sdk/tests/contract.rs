use mhome_script_sdk::{
    codec::{self, Limits},
    guest,
    prelude::*,
    Field, FieldType as T, ScriptContract,
};

type Reading = f64;
#[derive(Debug, PartialEq, ScriptInput, ScriptOutput)]
struct Data {
    enabled: bool,
    label: std::string::String,
    reading: Reading,
    at: Timestamp,
    delay: Duration,
    temperature: Temperature,
    image: Image,
    object: Object,
    value: Value,
    values: Vec<Value>,
    objects: Vec<Object>,
    matrix: Vec<Vec<f64>>,
    r#type: String,
}
fn data() -> Data {
    Data {
        enabled: true,
        label: "test".into(),
        reading: 3.5,
        at: Timestamp {
            epoch_millis: i64::MAX,
        },
        delay: Duration::from_millis(i64::MIN),
        temperature: Temperature::from_celsius(22.5),
        image: Image {
            uri: "artifact://space/image".into(),
        },
        object: object! {"nested" => object! {"x" => vec![1.0,2.0]}},
        value: Value::Null,
        values: vec![
            Value::Null,
            true.into(),
            object! {"anything" => "works"}.into(),
            vec![1.0].into(),
        ],
        objects: vec![object! {}, object! {"a" => false}],
        matrix: vec![vec![], vec![1.0, 2.0]],
        r#type: "raw identifier".into(),
    }
}
#[test]
fn rust_types_define_metadata_and_lossless_conversion() {
    let expected: Vec<Field> = [
        ("enabled", T::Bool),
        ("label", T::String),
        ("reading", T::Number),
        ("at", T::Timestamp),
        ("delay", T::Duration),
        ("temperature", T::Temperature),
        ("image", T::Image),
        ("object", T::Object),
        ("value", T::Value),
        ("values", T::List(Box::new(T::Value))),
        ("objects", T::List(Box::new(T::Object))),
        ("matrix", T::List(Box::new(T::List(Box::new(T::Number))))),
        ("type", T::String),
    ]
    .into_iter()
    .map(|(name, ty)| Field {
        name: name.into(),
        ty,
    })
    .collect();
    let contract = ScriptContract::for_script::<Data, Data>();
    assert_eq!(contract.input, expected);
    assert_eq!(contract.output, expected);
    assert_eq!(contract.sdk_version, mhome_script_sdk::SDK_VERSION);
    assert_eq!(
        ScriptContract::decode(
            &guest::contract_bytes::<Data, Data>().unwrap(),
            Limits::default()
        )
        .unwrap(),
        contract
    );
    let wire = codec::encode(&data().into_value(), Limits::default()).unwrap();
    assert_eq!(
        Data::from_value(codec::decode(&wire, Limits::default()).unwrap()).unwrap(),
        data()
    );
}
#[test]
fn strict_roots_and_typed_fields_reject_bad_inputs_before_run() {
    let valid = Object::try_from(data().into_value()).unwrap();
    let mut missing = valid.clone();
    missing.remove("value"); // Dynamic does not mean optional.
    let mut extra = valid.clone();
    extra.insert("unknown".into(), Value::Null);
    let mut cases: Vec<(Value, &str)> = vec![
        (Value::Null, "argument object"),
        (vec![1.0].into(), "argument object"),
        (missing.into(), "missing argument value"),
        (extra.into(), "unexpected arguments: unknown"),
    ];
    for (field, value, message) in [
        ("reading", "3.5".into(), "argument reading: expected Number"),
        (
            "object",
            vec![1.0].into(),
            "argument object: expected Object",
        ),
        (
            "image",
            object! {"uri" => true}.into(),
            "argument image: expected String",
        ),
        (
            "objects",
            vec![Value::Null].into(),
            "argument objects: [0]: expected Object",
        ),
        (
            "matrix",
            vec![1.0].into(),
            "argument matrix: [0]: expected Array",
        ),
        (
            "matrix",
            vec![vec!["wrong"]].into(),
            "argument matrix: [0]: [0]: expected Number",
        ),
    ] {
        let mut bad = valid.clone();
        bad.insert(field.into(), value);
        cases.push((bad.into(), message));
    }
    for (input, expected) in cases {
        let error = guest::run_typed::<Data, Data>(input, |_| {
            panic!("invalid arguments must not reach run")
        })
        .unwrap_err();
        assert!(
            error.message.contains(expected),
            "{error}; expected {expected}"
        );
    }
}
#[test]
fn image_accepts_existing_schema_fields_and_empty_objects_are_explicit() {
    let image = Image::try_from(Value::Object(
        object! {"uri" => "artifact://x", "id" => "1", "displayName" => "picture"},
    ))
    .unwrap();
    assert_eq!(image.uri, "artifact://x");
    #[derive(ScriptInput, ScriptOutput)]
    struct Empty {}
    assert_eq!(ScriptContract::for_script::<Empty, Empty>().input, vec![]);
    assert_eq!(
        Empty::from_value(object! {}.into()).unwrap().into_value(),
        object! {}.into()
    );
    assert!(Empty::from_value(object! {"x" => 1.0}.into()).is_err());
}
#[test]
fn contract_wire_shape_is_explicit_and_versions_are_checked() {
    #[derive(ScriptInput, ScriptOutput)]
    struct Root {
        items: Vec<Vec<f64>>,
    }
    let expected: Value = object! {
        "version" => 1.0, "sdkVersion" => mhome_script_sdk::SDK_VERSION,
        "input" => vec![object! {"name" => "items", "type" => object! {"list" => object! {"list" => "number"}}}],
        "output" => vec![object! {"name" => "items", "type" => object! {"list" => object! {"list" => "number"}}}],
    }.into();
    let bytes = guest::contract_bytes::<Root, Root>().unwrap();
    assert_eq!(codec::decode(&bytes, Limits::default()).unwrap(), expected);
    let mut root = Object::try_from(expected).unwrap();
    for (key, value) in [
        ("version", 2.0.into()),
        ("sdkVersion", "".into()),
        ("input", Value::Null),
        (
            "input",
            vec![object! {"name" => "x", "type" => "unknown"}].into(),
        ),
        (
            "input",
            vec![object! {"name" => "x", "type" => "number"}; 2].into(),
        ),
        (
            "output",
            vec![object! {"name" => "", "type" => "object"}].into(),
        ),
        (
            "output",
            vec![
                object! {"name" => "x", "type" => object! {"list" => "value", "optional" => true}},
            ]
            .into(),
        ),
        (
            "output",
            vec![object! {"name" => "x", "type" => "value", "default" => 1.0}].into(),
        ),
    ] {
        let mut bad = root.clone();
        bad.insert(key.into(), value);
        let bytes = codec::encode(&bad.into(), Limits::default()).unwrap();
        assert!(
            ScriptContract::decode(&bytes, Limits::default()).is_err(),
            "{key}"
        );
    }
    root.insert("unknown".into(), true.into());
    assert!(ScriptContract::decode(
        &codec::encode(&root.into(), Limits::default()).unwrap(),
        Limits::default()
    )
    .is_err());
    let small = Limits {
        bytes: 8,
        ..Limits::default()
    };
    assert!(ScriptContract::decode(&bytes, small).is_err());
    assert!(ScriptContract::for_script::<Root, Root>()
        .encode(small)
        .is_err());
}
