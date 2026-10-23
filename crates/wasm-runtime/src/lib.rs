//! Process-wide Wasmtime engine. ABI adapters own their Linkers and Store state.
use anyhow::{bail, ensure, Context, Result};
use mhome_script_sdk::{codec, ScriptResult, Value};
use std::{
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;
pub use tokio_util::sync::CancellationToken;
use wasmtime::{
    Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, UpdateDeadline,
};

fn engine_config() -> Config {
    let mut config = Config::new();
    config.epoch_interruption(true);
    config.consume_fuel(false);
    #[cfg(target_os = "ios")]
    configure_pulley(&mut config);
    config
}
/// Also used by host-side tests to exercise the iOS interpreter configuration.
#[cfg(any(target_os = "ios", test))]
fn configure_pulley(config: &mut Config) {
    config.target("pulley64").expect("valid Pulley target");
    config.signals_based_traps(false);
    config.memory_reservation(0);
    config.memory_reservation_for_growth(1024 * 1024);
    config.memory_guard_size(0);
}

/// One engine and one independent ticker per process. Clones share the engine.
pub fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let engine = Engine::new(&engine_config()).expect("failed to create Wasm engine");
        let ticker = engine.clone();
        std::thread::Builder::new()
            .name("wasm-epoch".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_millis(10));
                ticker.increment_epoch();
            })
            .expect("failed to start Wasm epoch ticker");
        engine
    })
}
pub fn compilation_slots() -> Arc<Semaphore> {
    static SLOTS: OnceLock<Arc<Semaphore>> = OnceLock::new();
    SLOTS.get_or_init(|| Arc::new(Semaphore::new(2))).clone()
}
/// Must run BEFORE instantiation: a module start function can loop too.
/// Async callers yield at every tick, so dropping a pending call can cancel it.
pub fn configure_store<T: 'static>(
    store: &mut Store<T>,
    timeout: Option<Duration>,
    cancel: CancellationToken,
) {
    let started = Instant::now();
    store.set_epoch_deadline(1);
    store.epoch_deadline_callback(move |_| {
        if cancel.is_cancelled() {
            return Err(wasmtime::Error::msg("Wasm invocation cancelled"));
        }
        if timeout.is_some_and(|timeout| started.elapsed() >= timeout) {
            return Err(wasmtime::Error::msg("Wasm invocation timed out"));
        }
        Ok(UpdateDeadline::Yield(1))
    });
}

#[derive(Clone, Debug)]
pub struct ScriptLimits {
    pub timeout: Duration,
    pub memory_bytes: usize,
    pub table_elements: usize,
    pub wire: codec::Limits,
}
impl Default for ScriptLimits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            memory_bytes: 64 * 1024 * 1024,
            table_elements: 10000,
            wire: codec::Limits::default(),
        }
    }
}
#[derive(Clone)]
pub struct ScriptModule(Module);
impl ScriptModule {
    /// Compiles portable Wasm, never deserializes native machine-code caches.
    pub async fn compile(bytes: Vec<u8>) -> Result<Self> {
        ensure!(
            bytes.len() <= 16 * 1024 * 1024,
            "script module exceeds size limit"
        );
        let slot = compilation_slots().acquire_owned().await?;
        tokio::task::spawn_blocking(move || {
            let _slot = slot;
            Self::validate(Module::from_binary(engine(), &bytes)?)
        })
        .await?
    }
    fn validate(module: Module) -> Result<Self> {
        ensure!(
            module.imports().next().is_none(),
            "script host imports are not supported by ABI v1"
        );
        let memory = module
            .get_export("memory")
            .and_then(|t| t.memory().cloned())
            .context("script memory export missing")?;
        ensure!(
            !memory.is_64() && !memory.is_shared(),
            "script requires unshared wasm32 memory"
        );
        // Function signatures are checked on the Store, before calling any export.
        Ok(Self(module))
    }
    pub async fn invoke(
        &self,
        args: &Value,
        limits: ScriptLimits,
        cancel: CancellationToken,
    ) -> Result<ScriptResult> {
        ensure!(!cancel.is_cancelled(), "Wasm invocation cancelled");
        ensure!(!limits.timeout.is_zero(), "Wasm invocation timed out");
        let input = codec::encode(args, limits.wire)?;
        let call = self.invoke_bytes(&input, &limits, cancel.clone());
        tokio::select! {
            biased;
            _ = cancel.cancelled() => bail!("Wasm invocation cancelled"),
            result = tokio::time::timeout(limits.timeout, call) => result.context("Wasm invocation timed out")?,
        }
    }
    async fn invoke_bytes(
        &self,
        input: &[u8],
        limits: &ScriptLimits,
        cancel: CancellationToken,
    ) -> Result<ScriptResult> {
        let state = StoreLimitsBuilder::new()
            .memory_size(limits.memory_bytes)
            .table_elements(limits.table_elements)
            .instances(1)
            .memories(1)
            .tables(1)
            .trap_on_grow_failure(true)
            .build();
        let mut store: Store<StoreLimits> = Store::new(engine(), state);
        store.limiter(|s| s);
        configure_store(&mut store, Some(limits.timeout), cancel);
        let linker = Linker::<StoreLimits>::new(engine());
        let instance = linker.instantiate_async(&mut store, &self.0).await?;
        let abi = instance.get_typed_func::<(), i32>(&mut store, "script_abi_version")?;
        let alloc = instance.get_typed_func::<i32, i32>(&mut store, "script_alloc")?;
        let dealloc = instance.get_typed_func::<(i32, i32), ()>(&mut store, "script_dealloc")?;
        let invoke = instance.get_typed_func::<(i32, i32), i64>(&mut store, "script_invoke")?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .context("script memory export missing")?;
        ensure!(
            abi.call_async(&mut store, ()).await? == mhome_script_sdk::ABI_VERSION,
            "unsupported script ABI version"
        );
        let in_len = i32::try_from(input.len()).context("script input length overflow")?;
        let in_ptr = alloc.call_async(&mut store, in_len).await? as u32 as usize;
        ensure!(in_ptr != 0, "script allocation failed");
        checked_range(
            in_ptr,
            input.len(),
            memory.data_size(&store),
            limits.wire.bytes,
        )?;
        memory.write(&mut store, in_ptr, input)?;
        let packed = invoke
            .call_async(&mut store, (in_ptr as i32, in_len))
            .await? as u64;
        let out_ptr = (packed >> 32) as usize;
        let out_len = (packed & 0xffff_ffff) as usize;
        ensure!(out_ptr != 0 && out_len != 0, "empty script response");
        checked_range(
            out_ptr,
            out_len,
            memory.data_size(&store),
            limits.wire.bytes,
        )?;
        ensure!(
            out_ptr + out_len <= in_ptr || in_ptr + input.len() <= out_ptr,
            "script output aliases input allocation"
        );
        // Check bounds BEFORE allocating a host buffer controlled by a guest length.
        let mut output = vec![0u8; out_len];
        memory.read(&store, out_ptr, &mut output)?;
        dealloc
            .call_async(&mut store, (in_ptr as i32, in_len))
            .await?;
        dealloc
            .call_async(&mut store, (out_ptr as i32, out_len as i32))
            .await?;
        Ok(codec::decode_result(&output, limits.wire)?)
        // Any error/trap drops the whole Store: never re-enter a failed guest for cleanup.
    }
}
fn checked_range(ptr: usize, len: usize, memory: usize, max: usize) -> Result<()> {
    ensure!(len <= max, "script buffer exceeds byte limit");
    ensure!(
        ptr.checked_add(len).is_some_and(|end| end <= memory),
        "script buffer outside guest memory"
    );
    Ok(())
}

#[cfg(test)]
mod tests;
