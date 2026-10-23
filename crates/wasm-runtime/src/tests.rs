use super::*;
use mhome_script_sdk::{object, Duration as ScriptDuration, Temperature, Timestamp};

fn wasm_text(invoke: &str, extra: &str) -> String {
    format!(
        r#"(module
      (memory (export "memory") 1)
      (data (i32.const 1024) "\01\00\00")
      (func (export "script_abi_version") (result i32) i32.const 1)
      (func (export "script_alloc") (param i32) (result i32) i32.const 64)
      (func (export "script_dealloc") (param i32 i32))
      (func (export "script_invoke") (param i32 i32) (result i64) {invoke})
      {extra})"#
    )
}
fn wasm(invoke: &str, extra: &str) -> Vec<u8> {
    wat::parse_str(wasm_text(invoke, extra)).unwrap()
}
fn success() -> &'static str {
    "i64.const 4398046511107"
} // (1024 << 32) | 3
fn limits() -> ScriptLimits {
    ScriptLimits {
        timeout: Duration::from_millis(100),
        ..Default::default()
    }
}
async fn call(bytes: Vec<u8>) -> Result<ScriptResult> {
    ScriptModule::compile(bytes)
        .await?
        .invoke(&object! {}.into(), limits(), CancellationToken::new())
        .await
}
#[tokio::test]
async fn success_uses_shared_engine() {
    let module = ScriptModule::compile(wasm(success(), "")).await.unwrap();
    assert!(Engine::same(module.0.engine(), engine()));
    assert_eq!(
        module
            .invoke(&object! {}.into(), limits(), CancellationToken::new())
            .await
            .unwrap()
            .unwrap(),
        Value::Null
    );
}
#[tokio::test]
async fn infinite_loop_and_start_function_time_out() {
    for bytes in [
        wasm("(loop $spin br $spin) unreachable", ""),
        wasm(
            success(),
            "(func $start (loop $spin br $spin)) (start $start)",
        ),
    ] {
        let start = Instant::now();
        assert!(call(bytes).await.is_err());
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}
#[tokio::test]
async fn cancellation_is_per_store_and_does_not_starve_executor() {
    let looping = ScriptModule::compile(wasm("(loop $spin br $spin) unreachable", ""))
        .await
        .unwrap();
    let working = ScriptModule::compile(wasm(success(), "")).await.unwrap();
    let cancel = CancellationToken::new();
    let other = cancel.clone();
    let args = Value::Object(Default::default());
    let slow = looping.invoke(&args, ScriptLimits::default(), cancel);
    let concurrent = async {
        tokio::time::sleep(Duration::from_millis(30)).await;
        other.cancel();
        working
            .invoke(&args, ScriptLimits::default(), CancellationToken::new())
            .await
    };
    let (a, b) = tokio::join!(slow, concurrent);
    assert!(a.unwrap_err().to_string().contains("cancel"));
    assert_eq!(b.unwrap().unwrap(), Value::Null);
}
#[tokio::test]
async fn dropping_call_releases_store_and_later_call_runs() {
    let looping = ScriptModule::compile(wasm("(loop $spin br $spin) unreachable", ""))
        .await
        .unwrap();
    assert!(tokio::time::timeout(
        Duration::from_millis(40),
        looping.invoke(
            &Value::Null,
            ScriptLimits::default(),
            CancellationToken::new()
        )
    )
    .await
    .is_err());
    assert!(call(wasm(success(), "")).await.unwrap().is_ok());
}
#[tokio::test]
async fn rejects_bad_buffers_memory_growth_and_traps() {
    for body in [
        "i64.const -1",
        "i64.const 0",
        "i64.const 274877906947",    // input alias
        "i64.const 281470681743363", // out of bounds
        "i32.const 2048 memory.grow drop i64.const 4398046511107",
        "unreachable",
    ] {
        assert!(call(wasm(body, "")).await.is_err(), "{body}");
    }
    let mut small = limits();
    small.memory_bytes = 100;
    let m = ScriptModule::compile(wasm(success(), "")).await.unwrap();
    assert!(m
        .invoke(&Value::Null, small, CancellationToken::new())
        .await
        .is_err());
}
#[tokio::test]
async fn rejects_imports_wrong_signatures_and_versions() {
    let imported = wat::parse_str(
        "(module (import \"host\" \"http_request_async\" (func)) (memory (export \"memory\") 1))",
    )
    .unwrap();
    assert!(ScriptModule::compile(imported).await.is_err());
    assert!(ScriptModule::compile(vec![0, 1, 2]).await.is_err());
    for module in [
        "(module (memory (export \"memory\") 1) (func (export \"script_abi_version\") (result i64) i64.const 1))",
        "(module (memory (export \"memory\") 1))",
    ] { assert!(call(wat::parse_str(module).unwrap()).await.is_err()); }
}
#[tokio::test]
async fn pre_cancelled_invocation_never_runs_guest() {
    let module = ScriptModule::compile(wasm("unreachable", ""))
        .await
        .unwrap();
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(module
        .invoke(&Value::Null, limits(), cancel)
        .await
        .unwrap_err()
        .to_string()
        .contains("cancel"));
}
#[tokio::test]
async fn integration_style_store_initialization_with_epoch_and_start() {
    let mut store = Store::new(engine(), ());
    configure_store(&mut store, None, CancellationToken::new());
    let module = Module::new(engine(), "(module (func $start nop) (start $start) (func (export \"invoke\") (result i32) i32.const 42))").unwrap();
    let instance = Linker::new(engine())
        .instantiate_async(&mut store, &module)
        .await
        .unwrap();
    assert_eq!(
        instance
            .get_typed_func::<(), i32>(&mut store, "invoke")
            .unwrap()
            .call_async(&mut store, ())
            .await
            .unwrap(),
        42
    );
}
#[tokio::test]
async fn pulley_epoch_interrupts_loop() {
    let mut config = engine_config();
    configure_pulley(&mut config);
    let engine = Engine::new(&config).unwrap();
    let module = Module::new(&engine, "(module (func (export \"spin\") (loop $l br $l)))").unwrap();
    let mut store = Store::new(&engine, ());
    configure_store(
        &mut store,
        Some(Duration::from_millis(40)),
        CancellationToken::new(),
    );
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &module)
        .await
        .unwrap();
    let tick_engine = engine.clone();
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ticker_stop = stop.clone();
    let ticker = std::thread::spawn(move || {
        while !ticker_stop.load(std::sync::atomic::Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(5));
            tick_engine.increment_epoch();
        }
    });
    let result = instance
        .get_typed_func::<(), ()>(&mut store, "spin")
        .unwrap()
        .call_async(&mut store, ())
        .await;
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    ticker.join().unwrap();
    assert!(result.is_err());
}
#[tokio::test]
#[ignore = "requires the Rust wasm32 fixture; run scripts/test-script-wasm.sh"]
async fn rust_sdk_fixture_runs_real_wrapper_and_errors() {
    let path = std::env::var("SCRIPT_SDK_WASM").expect("SCRIPT_SDK_WASM fixture path");
    let module = ScriptModule::compile(std::fs::read(path).unwrap())
        .await
        .unwrap();
    let input = object! {
        "samples" => vec![100.0, 200.0], "targetLux" => 300.0,
        "temperature" => Temperature::from_celsius(23.5), "delay" => ScriptDuration::from_millis(1500),
        "at" => Timestamp {epoch_millis: 1722650400000}, "settings" => object! {"mode" => "auto"},
    };
    let result = module
        .invoke(
            &input.clone().into(),
            ScriptLimits::default(),
            CancellationToken::new(),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.get("brightness").unwrap().as_number().unwrap(), 50.0);
    for field in ["temperature", "delay", "at", "settings"] {
        assert_eq!(result.get(field).unwrap(), &input[field]);
    }
    let mut bad = input;
    bad.insert("samples".into(), Value::Array(vec![]));
    assert_eq!(
        module
            .invoke(
                &bad.into(),
                ScriptLimits::default(),
                CancellationToken::new()
            )
            .await
            .unwrap()
            .unwrap_err()
            .code,
        "INVALID_INPUT"
    );
    let bad = object! {"samples" => "wrong type"};
    assert_eq!(
        module
            .invoke(
                &bad.into(),
                ScriptLimits::default(),
                CancellationToken::new()
            )
            .await
            .unwrap()
            .unwrap_err()
            .code,
        "INVALID_ARGUMENT"
    );
}
#[tokio::test]
#[ignore = "requires the Rust wasm32 fixture; run scripts/test-script-wasm.sh"]
async fn rust_sdk_fixture_typed_contract_and_invocation() {
    let path = std::env::var("SCRIPT_SDK_TYPED_WASM").expect("SCRIPT_SDK_TYPED_WASM fixture path");
    let module = ScriptModule::compile(std::fs::read(path).unwrap())
        .await
        .unwrap();
    let input = object! {
        "samples" => vec![100.0, 200.0], "target_lux" => 300.0,
        "temperature" => Temperature::from_celsius(23.5), "delay" => ScriptDuration::from_millis(1500),
        "at" => Timestamp {epoch_millis: 1722650400000}, "settings" => object! {"mode" => "auto"},
        "image" => mhome_script_sdk::Image { uri: "artifact://space/image".into() },
        "items" => vec![object! {"x" => 1.0}, object! {"y" => "dynamic"}],
        "data" => vec![Value::Null, object! {"nested" => vec![true, false]}.into()],
        "matrix" => vec![vec![1.0, 2.0], vec![], vec![3.0]],
    };
    use mhome_script_sdk::{Field, FieldType as T};
    let contract = module
        .contract(ScriptLimits::default(), CancellationToken::new())
        .await
        .unwrap();
    let fields = |entries: Vec<(&str, T)>| {
        entries
            .into_iter()
            .map(|(name, ty)| Field {
                name: name.into(),
                ty,
            })
            .collect::<Vec<_>>()
    };
    let common = fields(vec![
        ("temperature", T::Temperature),
        ("delay", T::Duration),
        ("at", T::Timestamp),
        ("settings", T::Object),
        ("image", T::Image),
        ("items", T::List(Box::new(T::Object))),
        ("data", T::Value),
        ("matrix", T::List(Box::new(T::List(Box::new(T::Number))))),
    ]);
    let mut expected_input = fields(vec![
        ("samples", T::List(Box::new(T::Number))),
        ("target_lux", T::Number),
    ]);
    expected_input.extend(common.clone());
    let mut expected_output = fields(vec![
        ("brightness", T::Number),
        ("levels", T::List(Box::new(T::Number))),
    ]);
    expected_output.extend(common);
    assert_eq!(contract.input, expected_input);
    assert_eq!(contract.output, expected_output);
    assert_eq!(contract.sdk_version, mhome_script_sdk::SDK_VERSION);
    let result = module
        .invoke(
            &input.clone().into(),
            ScriptLimits::default(),
            CancellationToken::new(),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.get("brightness").unwrap().as_number().unwrap(), 50.0);
    for field in [
        "temperature",
        "delay",
        "at",
        "settings",
        "image",
        "items",
        "data",
        "matrix",
    ] {
        assert_eq!(result.get(field).unwrap(), &input[field]);
    }
    assert_eq!(result.get("levels").unwrap(), &input["samples"]);
    for (key, value, message) in [
        ("target_lux", Some("wrong".into()), "argument target_lux"),
        ("samples", None, "missing argument samples"),
        ("extra", Some(Value::Null), "unexpected arguments: extra"),
        (
            "matrix",
            Some(vec![vec!["wrong"]].into()),
            "argument matrix: [0]: [0]",
        ),
        ("items", Some(vec![1.0].into()), "argument items: [0]"),
    ] {
        let mut bad = input.clone();
        match value {
            Some(value) => {
                bad.insert(key.into(), value);
            }
            None => {
                bad.remove(key);
            }
        }
        let error = module
            .invoke(
                &bad.into(),
                ScriptLimits::default(),
                CancellationToken::new(),
            )
            .await
            .unwrap()
            .unwrap_err();
        assert_eq!(error.code, "INVALID_ARGUMENT");
        assert!(error.message.contains(message), "{error}");
    }
    let mut bad = input;
    bad.insert("samples".into(), Value::Array(vec![]));
    assert_eq!(
        module
            .invoke(
                &bad.into(),
                ScriptLimits::default(),
                CancellationToken::new()
            )
            .await
            .unwrap()
            .unwrap_err()
            .code,
        "INVALID_INPUT"
    );
    let bad = object! {"samples" => "wrong type"};
    assert_eq!(
        module
            .invoke(
                &bad.into(),
                ScriptLimits::default(),
                CancellationToken::new()
            )
            .await
            .unwrap()
            .unwrap_err()
            .code,
        "INVALID_ARGUMENT"
    );
}

#[tokio::test]
async fn abi_version_and_every_guest_entrypoint_are_bounded() {
    let wrong_version =
        wasm_text(success(), "").replace("(result i32) i32.const 1)", "(result i32) i32.const 2)");
    assert!(call(wat::parse_str(wrong_version).unwrap())
        .await
        .unwrap_err()
        .to_string()
        .contains("ABI version"));
    for (before, after) in [
        (
            "(result i32) i32.const 1)",
            "(result i32) (loop $l br $l) unreachable)",
        ),
        (
            "(result i32) i32.const 64)",
            "(result i32) (loop $l br $l) unreachable)",
        ),
        (
            "(func (export \"script_dealloc\") (param i32 i32))",
            "(func (export \"script_dealloc\") (param i32 i32) (loop $l br $l))",
        ),
    ] {
        let bytes = wat::parse_str(wasm_text(success(), "").replace(before, after)).unwrap();
        assert!(call(bytes).await.is_err());
    }
}
#[tokio::test]
async fn tables_and_malformed_envelopes_are_rejected() {
    assert!(call(wasm(success(), "(table 10001 funcref)"))
        .await
        .is_err());
    assert!(
        call(wasm(success(), r#"(data (i32.const 1024) "\02\00\00")"#))
            .await
            .is_err()
    );
}
#[tokio::test]
async fn host_frees_input_then_output_once() {
    let text = wasm_text(success(), "(global $frees (mut i32) (i32.const 0))").replace(
        "(func (export \"script_dealloc\") (param i32 i32))",
        r#"(func (export "script_dealloc") (param $ptr i32) (param $len i32)
          global.get $frees i32.eqz
          if local.get $ptr i32.const 64 i32.ne if unreachable end
          else local.get $ptr i32.const 1024 i32.ne if unreachable end end
          global.get $frees i32.const 1 i32.add global.set $frees)"#,
    );
    assert!(call(wat::parse_str(text).unwrap()).await.unwrap().is_ok());
}

fn contract_module(bytes: &[u8], contract_body: Option<&str>) -> Vec<u8> {
    let data = bytes
        .iter()
        .map(|byte| format!("\\{byte:02x}"))
        .collect::<String>();
    let packed = (2048u64 << 32) | bytes.len() as u64;
    let body = contract_body
        .map(str::to_owned)
        .unwrap_or_else(|| format!("i64.const {packed}"));
    // Both allocation and run trap if metadata extraction accidentally calls them.
    let text = wasm_text(
        "unreachable",
        &format!(
            r#"
        (data (i32.const 2048) "{data}")
        (func (export "script_contract") (result i64) {body})"#
        ),
    )
    .replace("(result i32) i32.const 64)", "(result i32) unreachable)");
    wat::parse_str(text).unwrap()
}
fn contract_fixture() -> Vec<u8> {
    mhome_script_sdk::ScriptContract {
        sdk_version: mhome_script_sdk::SDK_VERSION.into(),
        input: vec![],
        output: vec![],
    }
    .encode(codec::Limits::default())
    .unwrap()
}
#[tokio::test]
async fn metadata_does_not_run_business_code_and_frees_exact_buffer() {
    let bytes = contract_fixture();
    let module = ScriptModule::compile(contract_module(&bytes, None))
        .await
        .unwrap();
    assert!(Engine::same(module.0.engine(), engine()));
    assert_eq!(
        module
            .contract(limits(), CancellationToken::new())
            .await
            .unwrap()
            .input,
        vec![]
    );
    let packed = (2048u64 << 32) | bytes.len() as u64;
    let data = bytes
        .iter()
        .map(|byte| format!("\\{byte:02x}"))
        .collect::<String>();
    let text = wasm_text(
        "unreachable",
        &format!(
            r#"
        (data (i32.const 2048) "{data}")
        (global $freed (mut i32) (i32.const 0))
        (func (export "script_contract") (result i64) i64.const {packed})"#
        ),
    )
    .replace(
        r#"(func (export "script_dealloc") (param i32 i32))"#,
        &format!(
            r#"(func (export "script_dealloc") (param i32 i32)
                global.get $freed if unreachable end
                local.get 0 i32.const 2048 i32.ne if unreachable end
                local.get 1 i32.const {} i32.ne if unreachable end
                i32.const 1 global.set $freed)"#,
            bytes.len()
        ),
    );
    ScriptModule::compile(wat::parse_str(text).unwrap())
        .await
        .unwrap()
        .contract(limits(), CancellationToken::new())
        .await
        .unwrap();
    let trapping = wasm_text(
        "unreachable",
        &format!(
            r#"
        (data (i32.const 2048) "{data}")
        (func (export "script_contract") (result i64) i64.const {packed})"#
        ),
    )
    .replace(
        r#"(func (export "script_dealloc") (param i32 i32))"#,
        r#"(func (export "script_dealloc") (param i32 i32) unreachable)"#,
    );
    let error = ScriptModule::compile(wat::parse_str(trapping).unwrap())
        .await
        .unwrap()
        .contract(limits(), CancellationToken::new())
        .await
        .unwrap_err();
    assert!(format!("{error:#}").contains("unreachable"));
}
#[tokio::test]
async fn metadata_rejects_missing_invalid_and_oversized_exports() {
    let fixture = contract_fixture();
    for bytes in [
        wasm(success(), ""), // legacy dynamic-only module has no contract
        contract_module(&[63], None),
        contract_module(&fixture, Some("i64.const 0")),
        contract_module(&fixture, Some("i64.const -1")),
        contract_module(&fixture, Some("i64.const 281470681743363")),
        contract_module(&fixture, Some("unreachable")),
        contract_module(
            &fixture,
            Some("i32.const 2048 memory.grow drop i64.const 0"),
        ),
    ] {
        assert!(ScriptModule::compile(bytes)
            .await
            .unwrap()
            .contract(limits(), CancellationToken::new())
            .await
            .is_err());
    }
    for replacement in ["i32.const 2", "(loop $spin br $spin) unreachable"] {
        let text = wasm_text(
            "unreachable",
            r#"(func (export "script_contract") (result i64) i64.const 0)"#,
        )
        .replace(
            "(result i32) i32.const 1)",
            &format!("(result i32) {replacement})"),
        );
        let error = ScriptModule::compile(wat::parse_str(text).unwrap())
            .await
            .unwrap()
            .contract(limits(), CancellationToken::new())
            .await
            .unwrap_err();
        if replacement == "i32.const 2" {
            assert!(error.to_string().contains("ABI version"));
        } else {
            assert!(format!("{error:#}").contains("timed out"));
        }
    }
    let module = ScriptModule::compile(contract_module(&fixture, None))
        .await
        .unwrap();
    let mut tiny = limits();
    tiny.wire.bytes = 8;
    assert!(module
        .contract(tiny, CancellationToken::new())
        .await
        .is_err());
    let mut tiny = limits();
    tiny.memory_bytes = 100;
    assert!(module
        .contract(tiny, CancellationToken::new())
        .await
        .is_err());
    let wrong = wasm_text(
        "unreachable",
        r#"(func (export "script_contract") (result i32) i32.const 0)"#,
    );
    assert!(ScriptModule::compile(wat::parse_str(wrong).unwrap())
        .await
        .unwrap()
        .contract(limits(), CancellationToken::new())
        .await
        .is_err());
}
#[tokio::test]
async fn metadata_entrypoints_and_start_are_bounded_and_cancellable() {
    let bytes = contract_fixture();
    let slow = ScriptModule::compile(contract_module(
        &bytes,
        Some("(loop $spin br $spin) unreachable"),
    ))
    .await
    .unwrap();
    let start = Instant::now();
    assert!(slow
        .contract(limits(), CancellationToken::new())
        .await
        .is_err());
    assert!(start.elapsed() < Duration::from_secs(5));
    for extra in ["(func $start (loop $spin br $spin)) (start $start)", ""] {
        let text = wasm_text(
            "unreachable",
            &format!(
                r#"{extra} (func (export "script_contract") (result i64) i64.const 4398046511107)"#
            ),
        );
        let text = if extra.is_empty() {
            text.replace(
                r#"(func (export "script_dealloc") (param i32 i32))"#,
                r#"(func (export "script_dealloc") (param i32 i32) (loop $spin br $spin))"#,
            )
        } else {
            text
        };
        let error = ScriptModule::compile(wat::parse_str(text).unwrap())
            .await
            .unwrap()
            .contract(limits(), CancellationToken::new())
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains("timed out"));
    }
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(slow
        .contract(limits(), cancel)
        .await
        .unwrap_err()
        .to_string()
        .contains("cancel"));
    let cancel = CancellationToken::new();
    let other = cancel.clone();
    let (result, _) = tokio::join!(slow.contract(ScriptLimits::default(), cancel), async move {
        tokio::time::sleep(Duration::from_millis(30)).await;
        other.cancel();
    });
    assert!(result.unwrap_err().to_string().contains("cancel"));
}
