use mhome_script_sdk::{
    codec::{self, Limits},
    *,
};
fn roundtrip(v: Value) {
    let bytes = codec::encode(&v, Limits::default()).unwrap();
    assert_eq!(codec::decode(&bytes, Limits::default()).unwrap(), v);
}
#[test]
fn scalar_and_nested_roundtrips() {
    for v in [
        Value::Null,
        true.into(),
        false.into(),
        (-12.5).into(),
        "猫🐈".into(),
        Timestamp {
            epoch_millis: i64::MAX,
        }
        .into(),
        Value::Array(vec![]),
        object! {"a" => vec![1.0, 2.0], "child" => object!{"x" => "text"}}.into(),
    ] {
        roundtrip(v);
    }
}
#[test]
fn fixed_wire_fixtures_and_precision() {
    assert_eq!(
        codec::encode(&42.0.into(), Limits::default()).unwrap(),
        [4, 0, 0, 0, 0, 0, 0, 69, 64]
    );
    let v = Value::Duration(Duration {
        millis: 1500,
        display_unit: 0x22,
    });
    assert_eq!(
        codec::encode(&v, Limits::default()).unwrap(),
        [10, 1, 0x22, 0xdc, 5, 0, 0, 0, 0, 0, 0]
    );
    let t = Value::from(Temperature::from_celsius(23.123456789));
    let decoded = codec::decode(
        &codec::encode(&t, Limits::default()).unwrap(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        Temperature::try_from(decoded).unwrap().celsius,
        23.123456789_f64 as f32 as f64
    );
    // Display units do not reinterpret a base-unit value.
    let q = Value::Quantity(Quantity {
        kind: 2,
        unit: 1,
        value: 20.0,
    });
    roundtrip(q);
}
#[test]
fn all_quantity_and_calendar_tags() {
    roundtrip(Duration::from_millis(i64::MAX).into());
    roundtrip(Duration::from_millis(i64::MIN).into());
    for (kind, unit) in [(2, 2), (3, 5), (5, 0x10), (6, 0x30), (7, 0x40)] {
        roundtrip(Value::Quantity(Quantity {
            kind,
            unit,
            value: -12.0,
        }));
    }
    for v in [
        Value::LocalDate(chrono::NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()),
        Value::LocalTime(chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap()),
        Value::RelativeDate(51),
        Value::SpecialDate {
            year: 0,
            month: 0,
            day: 0,
            weekday: Some(1),
        },
        Value::SpecialTime {
            event: 0,
            unit: 0x20,
            offset_millis: -1000,
        },
    ] {
        roundtrip(v);
    }
}
#[test]
fn rejects_truncation_at_every_position() {
    let bytes = codec::encode(
        &object! {"x" => vec!["abc", "def"], "v" => Timestamp{epoch_millis:123}}.into(),
        Limits::default(),
    )
    .unwrap();
    for end in 0..bytes.len() {
        assert!(
            codec::decode(&bytes[..end], Limits::default()).is_err(),
            "offset {end}"
        );
    }
}
#[test]
fn rejects_invalid_wire_forms() {
    for bytes in [
        vec![],
        vec![63],
        vec![0, 0],
        vec![0x40],
        vec![0xc1, 0],
        vec![1, 1, 255],
        vec![0x93, 255, 255, 255, 255],
        vec![10, 2, 0x20, 0, 0, 0, 0],
        vec![10, 2, 3, 0, 0, 0, 0],
        vec![10, 3, 6, 0, 0, 0, 0],
        vec![13, 24, 0, 0],
        vec![12, 2, 30, 0xe8, 7],
        vec![12, 0x7f],
        vec![16, 2, 1, b'x', 0, 1, b'x', 0],
    ] {
        assert!(
            codec::decode(&bytes, Limits::default()).is_err(),
            "{bytes:?}"
        );
    }
}
#[test]
fn limits_apply_on_both_sides() {
    for limits in [
        Limits {
            bytes: 2,
            ..Limits::default()
        },
        Limits {
            values: 1,
            ..Limits::default()
        },
        Limits {
            depth: 0,
            ..Limits::default()
        },
    ] {
        let v = Value::Array(vec![Value::Null, Value::Null]);
        assert!(codec::encode(&v, limits).is_err());
        assert!(codec::decode(&[19, 2, 0, 0], limits).is_err());
    }
    let mut object = Object::new();
    object.insert("猫".repeat(86), Value::Null);
    assert!(codec::encode(&Value::Object(object), Limits::default()).is_err());
    let deep = [[19, 1].repeat(65), vec![0]].concat();
    assert!(codec::decode(&deep, Limits::default()).is_err());
}
#[test]
fn sized_values_cross_length_widths() {
    for size in [0, 255, 256, 65535, 65536] {
        roundtrip(Value::String("x".repeat(size)));
    }
}
#[test]
fn result_protocol_and_input_contract() {
    for result in [
        Ok(Value::Null),
        Err(ScriptError::new("EMPTY", "empty samples")),
    ] {
        assert_eq!(
            codec::decode_result(
                &codec::encode_result(&result, Limits::default()).unwrap(),
                Limits::default()
            )
            .unwrap(),
            result
        );
    }
    for bytes in [&[2, 0, 0][..], &[1, 2, 0], &[1, 1, 0], &[1, 0, 0, 0]] {
        assert!(codec::decode_result(bytes, Limits::default()).is_err());
    }
    let mut args =
        Arguments::new(object! {"x" => vec![1.0,2.0], "obj" => object!{"a" => true}}.into())
            .unwrap();
    assert_eq!(args.take::<Vec<f64>>("x").unwrap(), vec![1.0, 2.0]);
    assert!(args.finish().is_err());
    let mut args = Arguments::new(object! {"x" => true}.into()).unwrap();
    assert!(args.take::<f64>("x").is_err());
    assert!(args.take_value("missing").is_err());
}
#[test]
fn arbitrary_short_bytes_never_panic() {
    // Deterministic malformed corpus, including random lengths, tags and UTF-8.
    let mut seed = 1234_u64;
    for n in 0..10000 {
        let len = n % 96;
        let mut bytes = vec![];
        for _ in 0..len {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            bytes.push(seed as u8);
        }
        let _ = codec::decode(&bytes, Limits::default());
    }
}
#[test]
fn image_reference_and_guest_invalid_input() {
    roundtrip(
        Image {
            uri: "artifact://space/image".into(),
        }
        .into(),
    );
    let out = guest::dispatch(&[63], |_| panic!("must not call user source"));
    assert!(codec::decode_result(&out, Limits::default())
        .unwrap()
        .is_err());
}
