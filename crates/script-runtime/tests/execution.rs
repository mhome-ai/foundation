use mhome_script_api::{ScriptError, ScriptValue as V, ValueLimits};
use mhome_script_runtime::{
    invoke, CancellationToken, HostFuture, InvocationControl, NoHost, ScriptHost, ScriptLimits,
};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

fn control(ms: u64) -> InvocationControl {
    InvocationControl::new(
        Instant::now() + Duration::from_millis(ms),
        CancellationToken::new(),
    )
}
async fn run(source: &str) -> Result<V, ScriptError> {
    invoke(
        source,
        vec![],
        Arc::new(NoHost),
        control(1000),
        ScriptLimits::default(),
    )
    .await
}
fn object(fields: impl IntoIterator<Item = (&'static str, V)>) -> V {
    V::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

#[tokio::test(flavor = "current_thread")]
async fn sync_async_and_module_initialization_return_raw_business_data() {
    for source in [
        "export default () => ({ok:false,n:42})",
        "const n = await Promise.resolve(42); export default async () => ({ok:false,n})",
    ] {
        assert_eq!(
            run(source).await.unwrap(),
            object([("ok", V::Bool(false)), ("n", V::Number(42.0))])
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn direct_conversion_preserves_integers_domain_precision_and_proto_keys() {
    let input = object([
        ("min", V::I64(i64::MIN)),
        ("max", V::U64(u64::MAX)),
        (
            "temp",
            object([
                ("_type", V::String("Temperature".into())),
                ("unit", V::String("F".into())),
                ("value", V::Number(72.123456789)),
            ]),
        ),
        (
            "duration",
            object([
                ("_type", V::String("Duration".into())),
                ("unit", V::String("ms".into())),
                ("value", V::Number(0.125)),
            ]),
        ),
        ("__proto__", object([("polluted", V::Bool(true))])),
    ]);
    let actual = invoke(
        "export default x => { if ({}.polluted) throw Error('pollution'); return x; }",
        vec![input.clone()],
        Arc::new(NoHost),
        control(1000),
        ScriptLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(actual, input);
    assert_eq!(
        run("export default () => 18446744073709551615n")
            .await
            .unwrap(),
        V::U64(u64::MAX)
    );
    assert!(run("export default () => 18446744073709551616n")
        .await
        .is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn rejects_lossy_unsupported_and_cyclic_results() {
    for body in [
        "undefined",
        "NaN",
        "Infinity",
        "Symbol()",
        "new Map()",
        "new Set()",
        "new Date()",
        "new Uint16Array(2)",
        "new (class Thing {})()",
        "(()=>{})",
        "[,,]",
        "new Proxy({}, {})",
        "(()=>{let x={};x.self=x;return x})()",
    ] {
        assert!(
            run(&format!("export default () => ({body})"))
                .await
                .is_err(),
            "{body}"
        );
    }
    // Reusing a value in two branches is not a cycle.
    assert!(run("export default () => {let x={v:1};return [x,x]}")
        .await
        .is_ok());
}

#[tokio::test(flavor = "current_thread")]
async fn binary_data_uses_uint8array_and_respects_view_offsets() {
    let result = invoke(
        "export default b => b.subarray(1,3)",
        vec![V::Bytes(vec![1, 2, 3, 4])],
        Arc::new(NoHost),
        control(1000),
        ScriptLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(result, V::Bytes(vec![2, 3]));
    assert_eq!(
        run("export default () => new Uint8Array([255,0,128])")
            .await
            .unwrap(),
        V::Bytes(vec![255, 0, 128])
    );
}

#[tokio::test(flavor = "current_thread")]
async fn errors_have_bounded_codes_and_messages() {
    let result = run(
        "export default () => { throw Object.assign(new Error('denied'), {code:'NO_ACCESS'}); }",
    )
    .await
    .unwrap_err();
    assert_eq!(result, ScriptError::new("NO_ACCESS", "denied"));
    let error = run("export default () => {throw Error('猫'.repeat(10000));}")
        .await
        .unwrap_err();
    assert!(error.message.len() <= 4096);
}

#[tokio::test(flavor = "current_thread")]
async fn file_node_network_imports_are_not_available() {
    assert!(
        run("import {call} from '@meowlink/script/host'; export default () => typeof call")
            .await
            .is_err()
    );
    for name in [
        "fs",
        "node:fs",
        "./other.js",
        "/etc/passwd",
        "https://example.com/code.js",
    ] {
        assert!(
            run(&format!("import x from '{name}'; export default () => x"))
                .await
                .is_err()
        );
        assert!(run(&format!("export default async () => import('{name}')"))
            .await
            .is_err());
    }
    assert_eq!(
        run("export default () => [typeof process,typeof require,typeof fetch]")
            .await
            .unwrap(),
        V::List(vec![V::String("undefined".into()); 3])
    );
}

#[tokio::test(flavor = "current_thread")]
async fn heap_and_conversion_limits_fail_without_poisoning_next_invocation() {
    let mut limits = ScriptLimits {
        memory_bytes: 1024 * 1024,
        ..ScriptLimits::default()
    };
    assert!(invoke(
        "export default () => Array.from({length:1000000},(_,i)=>({i}))",
        vec![],
        Arc::new(NoHost),
        control(1000),
        limits
    )
    .await
    .is_err());
    limits.memory_bytes = 32 * 1024 * 1024;
    limits.values = ValueLimits {
        nodes: 4,
        ..ValueLimits::default()
    };
    assert!(invoke(
        "export default () => [1,2,3,4,5]",
        vec![],
        Arc::new(NoHost),
        control(1000),
        limits
    )
    .await
    .is_err());
    assert_eq!(
        run("export default () => 42").await.unwrap(),
        V::Number(42.0)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn deadlines_cover_initialization_cpu_promises_and_result_getters() {
    for source in [
        "while(true){}; export default () => 1",
        "export default () => {while(true){try{while(true){}}catch(e){}}}",
        "await new Promise(()=>{}); export default () => 1",
        "export default () => new Promise(()=>{})",
        "export default () => ({get value(){while(true){}}})",
        "export default () => {let loop=()=>Promise.resolve().then(loop);loop();return 1}",
        "export default async () => {let loop=()=>Promise.resolve().then(loop);loop();await new Promise(()=>{})}",
    ] {
        let start = Instant::now();
        let error = invoke(
            source,
            vec![],
            Arc::new(NoHost),
            control(30),
            ScriptLimits::default(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "DEADLINE_EXCEEDED", "{source}: {error}");
        assert!(start.elapsed() < Duration::from_secs(2));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn cancellation_interrupts_cpu_even_when_local_executor_cannot_poll() {
    let control = control(2000);
    let token = control.cancel.clone();
    let thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        token.cancel();
    });
    let error = invoke(
        "export default () => {while(true){}}",
        vec![],
        Arc::new(NoHost),
        control,
        ScriptLimits::default(),
    )
    .await
    .unwrap_err();
    thread.join().unwrap();
    assert_eq!(error.code, "CANCELLED");
}

struct TestHost {
    calls: AtomicUsize,
    dropped: Arc<AtomicUsize>,
    children: AtomicUsize,
}
struct DropCount(Arc<AtomicUsize>);
impl Drop for DropCount {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
impl ScriptHost for TestHost {
    fn call<'a>(
        &'a self,
        operation: &'a str,
        args: Vec<V>,
        control: InvocationControl,
    ) -> HostFuture<'a> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            assert!(control.deadline > Instant::now());
            match operation {
                "echo" => Ok(V::List(args)),
                "wait" => {
                    let _guard = DropCount(self.dropped.clone());
                    std::future::pending().await
                }
                "fail" => Err(ScriptError::new("HOST_DENIED", "not authorized")),
                "channel" => {
                    self.children.fetch_add(1, Ordering::SeqCst);
                    // HostFuture is Send. A real adapter dispatches to the JS worker;
                    // tests of that scheduling boundary live with the worker adapter.
                    tokio::time::sleep(Duration::from_millis(2)).await;
                    Ok(V::Number(41.0))
                }
                _ => Err(ScriptError::new(
                    "UNKNOWN_OPERATION",
                    "Operation is not allowlisted",
                )),
            }
        })
    }
}
fn host() -> Arc<TestHost> {
    Arc::new(TestHost {
        calls: AtomicUsize::new(0),
        dropped: Arc::new(AtomicUsize::new(0)),
        children: AtomicUsize::new(0),
    })
}

#[tokio::test(flavor = "current_thread")]
async fn native_host_arguments_and_errors_use_direct_values() {
    let host = host();
    let result = invoke("import {call} from '@mhome/script/host'; export default async () => ({echo:await call('echo',[42,18446744073709551615n]),failure:await call('fail',[]).catch(e=>e.code)})", vec![], host.clone(), control(1000), ScriptLimits::default()).await.unwrap();
    assert_eq!(
        result,
        object([
            ("echo", V::List(vec![V::Number(42.0), V::U64(u64::MAX)])),
            ("failure", V::String("HOST_DENIED".into()))
        ])
    );
    assert_eq!(host.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn deadline_drops_pending_host_future() {
    let host = host();
    let error = invoke(
        "import {call} from '@mhome/script/host'; export default async () => await call('wait',[])",
        vec![],
        host.clone(),
        control(30),
        ScriptLimits::default(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, "DEADLINE_EXCEEDED");
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.dropped.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn dropping_caller_future_releases_host_work_without_cancelling_parent() {
    let host = host();
    let control = control(2000);
    let parent = control.cancel.clone();
    let future = invoke(
        "import {call} from '@mhome/script/host'; export default async () => await call('wait',[])",
        vec![],
        host.clone(),
        control,
        ScriptLimits::default(),
    );
    assert!(tokio::time::timeout(Duration::from_millis(30), future)
        .await
        .is_err());
    assert_eq!(host.dropped.load(Ordering::SeqCst), 1);
    assert!(!parent.is_cancelled());
}

#[tokio::test(flavor = "current_thread")]
async fn detached_host_calls_and_unhandled_rejections_are_not_success() {
    for source in ["import {call} from '@mhome/script/host'; export default () => {call('wait',[]);return 1}",
        "import {call} from '@mhome/script/host'; export default () => {Promise.resolve().then(()=>call('wait',[]));return 1}"] {
        let error = invoke(source, vec![], host(), control(1000), ScriptLimits::default()).await.unwrap_err();
        assert_eq!(error.code, "UNAWAITED_HOST_CALL");
    }
    assert_eq!(
        run("export default () => {Promise.reject(Error('lost'));return 1}")
            .await
            .unwrap_err()
            .code,
        "UNHANDLED_REJECTION"
    );
    assert_eq!(
        run("export default () => {Promise.reject(Error('handled')).catch(()=>{});return 1}")
            .await
            .unwrap(),
        V::Number(1.0)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn host_calls_are_bounded_before_spawning_futures() {
    let host = host();
    let limits = ScriptLimits {
        concurrent_host_calls: 1,
        ..ScriptLimits::default()
    };
    assert!(invoke("import {call} from '@mhome/script/host'; export default async () => await Promise.all([call('wait',[]),call('wait',[])])", vec![], host.clone(), control(1000), limits).await.is_err());
    assert!(host.calls.load(Ordering::SeqCst) <= 1);
    let limits = ScriptLimits {
        host_calls: 1,
        ..ScriptLimits::default()
    };
    assert!(invoke("import {call} from '@mhome/script/host'; export default async () => {await call('echo',[]);return await call('echo',[])}", vec![], host, control(1000), limits).await.is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn globals_do_not_cross_invocations() {
    assert_eq!(
        run("globalThis.secret=42;export default () => secret")
            .await
            .unwrap(),
        V::Number(42.0)
    );
    assert_eq!(
        run("export default () => typeof secret").await.unwrap(),
        V::String("undefined".into())
    );
    assert_eq!(
        run("export default () => ({})").await.unwrap(),
        V::Object(BTreeMap::new())
    );
}

#[tokio::test(flavor = "current_thread")]
async fn atomics_wait_cannot_block_the_worker() {
    assert!(
        run("export default () => Atomics.wait(new Int32Array(new SharedArrayBuffer(4)),0,0)")
            .await
            .is_err()
    );
}
