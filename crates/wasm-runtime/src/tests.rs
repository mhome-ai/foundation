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
