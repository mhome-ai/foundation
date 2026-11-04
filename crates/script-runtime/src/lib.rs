//! Native QuickJS execution, independent of Core/cloud storage and host services.
//! Each call owns its runtime. The local future must run on a dedicated JS worker,
//! not a request executor thread: JavaScript runs synchronously between awaits.
mod value;
mod worker;
pub use worker::{Admission, ScriptWorkers, WorkerConfig};

use mhome_script_api::{ScriptError, ScriptValue, ValueLimits, MAX_ARTIFACT_BYTES};
use rquickjs::{
    function::Args,
    loader::{ImportAttributes, Loader, Resolver},
    module::{Declarations, Exports, ModuleDef},
    promise::MaybePromise,
    AsyncContext, AsyncRuntime, Ctx, Exception, Function, Module, Promise, Value,
};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};
pub use tokio_util::sync::CancellationToken;

pub type HostFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ScriptValue, ScriptError>> + Send + 'a>>;

/// Host implementations own trusted tenant/Space/connection context. They must
/// allowlist operations, authorize each call, and bound IO before allocating it.
/// Dropping these futures releases temporary resources; it is not compensation.
pub trait ScriptHost: Send + Sync + 'static {
    fn call<'a>(
        &'a self,
        operation: &'a str,
        args: Vec<ScriptValue>,
        control: InvocationControl,
    ) -> HostFuture<'a>;
}

pub struct NoHost;
impl ScriptHost for NoHost {
    fn call<'a>(&'a self, _: &'a str, _: Vec<ScriptValue>, _: InvocationControl) -> HostFuture<'a> {
        Box::pin(async {
            Err(ScriptError::new(
                "HOST_UNAVAILABLE",
                "This script has no host capabilities",
            ))
        })
    }
}

#[derive(Clone, Debug)]
pub struct InvocationControl {
    pub deadline: Instant,
    pub cancel: CancellationToken,
    shutdown: CancellationToken,
}

impl InvocationControl {
    pub fn new(deadline: Instant, cancel: CancellationToken) -> Self {
        Self {
            deadline,
            cancel,
            shutdown: CancellationToken::new(),
        }
    }

    pub async fn cancelled(&self) {
        tokio::select! {
            _ = self.cancel.cancelled() => {},
            _ = self.shutdown.cancelled() => {},
        }
    }

    pub fn check(&self) -> Result<(), ScriptError> {
        if self.cancel.is_cancelled() || self.shutdown.is_cancelled() {
            Err(ScriptError::new("CANCELLED", "Script invocation cancelled"))
        } else if Instant::now() >= self.deadline {
            Err(ScriptError::new(
                "DEADLINE_EXCEEDED",
                "Script invocation timed out",
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ScriptLimits {
    pub memory_bytes: usize,
    pub stack_bytes: usize,
    pub values: ValueLimits,
    pub host_calls: usize,
    pub concurrent_host_calls: usize,
}

impl Default for ScriptLimits {
    fn default() -> Self {
        Self {
            memory_bytes: 32 * 1024 * 1024,
            stack_bytes: 512 * 1024,
            values: ValueLimits::default(),
            host_calls: 1024,
            concurrent_host_calls: 32,
        }
    }
}

struct CancelOnDrop(CancellationToken);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

/// Source must already be resolved against trusted ownership, manifest and hash.
/// This leaf executor does not grant resource access or fetch artifacts.
pub async fn invoke(
    source: &str,
    args: Vec<ScriptValue>,
    host: Arc<dyn ScriptHost>,
    control: InvocationControl,
    limits: ScriptLimits,
) -> Result<ScriptValue, ScriptError> {
    control.check()?;
    if source.is_empty() || source.len() > MAX_ARTIFACT_BYTES {
        return Err(ScriptError::new(
            "INVALID_SCRIPT",
            "JavaScript source size is out of bounds",
        ));
    }
    let args = ScriptValue::List(args);
    args.validate(limits.values)
        .map_err(|e| ScriptError::new("INVALID_ARGUMENT", e.to_string()))?;
    let control = InvocationControl {
        cancel: control.cancel.child_token(),
        ..control
    };
    let _cancel_on_drop = CancelOnDrop(control.cancel.clone());
    let runtime = AsyncRuntime::new().map_err(engine_error)?;
    runtime.set_memory_limit(limits.memory_bytes).await;
    runtime.set_max_stack_size(limits.stack_bytes).await;
    let interrupt = control.clone();
    runtime
        .set_interrupt_handler(Some(Box::new(move || interrupt.check().is_err())))
        .await;
    runtime.set_loader(HostResolver, HostLoader).await;
    let rejected = Arc::new(AtomicUsize::new(0));
    let rejection_count = rejected.clone();
    runtime
        .set_host_promise_rejection_tracker(Some(Box::new(move |_, _, _, handled| {
            if handled {
                let _ = rejection_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                    Some(n.saturating_sub(1))
                });
            } else {
                rejection_count.fetch_add(1, Ordering::Relaxed);
            }
        })))
        .await;
    let context = AsyncContext::full(&runtime).await.map_err(engine_error)?;
    let active = Arc::new(AtomicUsize::new(0));
    let call = context.async_with(async |ctx| {
        let result = async {
            install_host(&ctx, host, control.clone(), limits, active.clone())?;
            let (module, ready) = Module::declare(ctx.clone(), "entry.mjs", source)?.eval()?;
            ready.into_future::<()>().await?;
            let entry: Function = module.get("default")?;
            let ScriptValue::List(values) = &args else {
                unreachable!()
            };
            let mut arguments = Args::new(ctx.clone(), values.len());
            for arg in values {
                arguments.push_arg(value::to_js(&ctx, arg)?)?;
            }
            let result: MaybePromise = arguments.apply(&entry)?;
            let output = result.into_future::<Value>().await?;
            // Finish queued JS microtasks, but never drive newly detached host IO.
            // Interrupts and this check also bound a chain of tiny Promise jobs.
            while ctx.execute_pending_job() {
                control.check().map_err(|e| throw_host(&ctx, e))?;
            }
            value::from_js(output, limits.values)
        }
        .await;
        result.map_err(|error| exception_error(&ctx, error))
    });
    let result = tokio::select! {
        biased;
        _ = control.cancelled() => Err(ScriptError::new("CANCELLED", "Script invocation cancelled")),
        _ = tokio::time::sleep_until(control.deadline.into()) => Err(ScriptError::new("DEADLINE_EXCEEDED", "Script invocation timed out")),
        result = call => result,
    };
    // Interrupts report JS exceptions; expose the actual request cancellation reason.
    control.check()?;
    if result.is_ok() && active.load(Ordering::Acquire) != 0 {
        return Err(ScriptError::new(
            "UNAWAITED_HOST_CALL",
            "Script returned before its host calls completed",
        ));
    }
    if result.is_ok() && rejected.load(Ordering::Acquire) != 0 {
        return Err(ScriptError::new(
            "UNHANDLED_REJECTION",
            "Script left an unhandled Promise rejection",
        ));
    }
    result
    // Dropping context/runtime destroys their pending Promise/host futures. Never
    // re-enter failed guest code for cleanup or tear down host-owned subscriptions.
}

fn engine_error(error: rquickjs::Error) -> ScriptError {
    ScriptError::new("SCRIPT_ERROR", bounded(&error.to_string(), 4096))
}

fn bounded(value: &str, max: usize) -> String {
    let mut end = value.len().min(max);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn exception_error(ctx: &Ctx<'_>, error: rquickjs::Error) -> ScriptError {
    if !error.is_exception() {
        return engine_error(error);
    }
    let exception = ctx.catch();
    if let Some(object) = exception.as_object() {
        let code = object
            .get::<_, String>("code")
            .unwrap_or_else(|_| "SCRIPT_ERROR".into());
        let message = object
            .get::<_, String>("message")
            .unwrap_or_else(|_| "Script failed".into());
        ScriptError::new(bounded(&code, 128), bounded(&message, 4096))
    } else if let Some(message) = exception.as_string().and_then(|v| v.to_string().ok()) {
        ScriptError::new("SCRIPT_ERROR", bounded(&message, 4096))
    } else {
        ScriptError::new("SCRIPT_ERROR", "Script threw a non-error value")
    }
}

fn throw_host(ctx: &Ctx<'_>, error: ScriptError) -> rquickjs::Error {
    match Exception::from_message(ctx.clone(), &bounded(&error.message, 4096)) {
        Ok(exception) => {
            if let Err(error) = exception.as_object().set("code", bounded(&error.code, 128)) {
                return error;
            }
            exception.throw()
        }
        Err(error) => error,
    }
}

struct ActiveCall(Arc<AtomicUsize>);
impl Drop for ActiveCall {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

fn install_host<'js>(
    ctx: &Ctx<'js>,
    host: Arc<dyn ScriptHost>,
    control: InvocationControl,
    limits: ScriptLimits,
    active: Arc<AtomicUsize>,
) -> rquickjs::Result<()> {
    let calls = Arc::new(AtomicUsize::new(0));
    let function = Function::new(
        ctx.clone(),
        move |ctx: Ctx<'js>, operation: String, args: Value<'js>| {
            control.check().map_err(|e| throw_host(&ctx, e))?;
            if operation.is_empty() || operation.len() > 128 {
                return Err(Exception::throw_type(&ctx, "invalid host operation"));
            }
            let ScriptValue::List(args) = value::from_js(args, limits.values)? else {
                return Err(Exception::throw_type(
                    &ctx,
                    "host arguments must be an array",
                ));
            };
            if calls.fetch_add(1, Ordering::Relaxed) >= limits.host_calls {
                return Err(Exception::throw_range(
                    &ctx,
                    "script host call limit exceeded",
                ));
            }
            if active
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                    (n < limits.concurrent_host_calls).then_some(n + 1)
                })
                .is_err()
            {
                return Err(Exception::throw_range(
                    &ctx,
                    "script concurrent host call limit exceeded",
                ));
            }
            let guard = ActiveCall(active.clone());
            let host = host.clone();
            let control = control.clone();
            let future_ctx = ctx.clone();
            Promise::wrap_future(&ctx, async move {
                let _guard = guard;
                control.check().map_err(|e| throw_host(&future_ctx, e))?;
                let result = tokio::select! {
                biased;
                _ = control.cancelled() => Err(ScriptError::new("CANCELLED", "Host call cancelled")),
                _ = tokio::time::sleep_until(control.deadline.into()) => Err(ScriptError::new("DEADLINE_EXCEEDED", "Host call timed out")),
                result = host.call(&operation, args, control.clone()) => result,
            }.map_err(|e| throw_host(&future_ctx, e))?;
                control.check().map_err(|e| throw_host(&future_ctx, e))?;
                result
                    .validate(limits.values)
                    .map_err(|e| Exception::throw_range(&future_ctx, &e.to_string()))?;
                value::to_js(&future_ctx, &result)
            })
        },
    )?;
    // Function has a safe JsLifetime implementation. The native module consumes
    // this private value during evaluation; it is never a guest-writable global.
    ctx.store_userdata(function)
        .map_err(|_| Exception::throw_internal(ctx, "host bindings already borrowed"))?;
    Ok(())
}

const HOST_MODULE: &str = "@mhome/script/host";
struct HostResolver;
impl Resolver for HostResolver {
    fn resolve<'js>(
        &mut self,
        _: &Ctx<'js>,
        base: &str,
        name: &str,
        attributes: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<String> {
        if name == HOST_MODULE && attributes.is_none() {
            Ok(name.into())
        } else {
            Err(rquickjs::Error::new_resolving_message(
                base,
                name,
                "only bundled code and platform host imports are supported",
            ))
        }
    }
}
struct HostLoader;
impl Loader for HostLoader {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        _: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<Module<'js>> {
        if name == HOST_MODULE {
            Module::declare_def::<HostModule, _>(ctx.clone(), name)
        } else {
            Err(rquickjs::Error::new_loading(name))
        }
    }
}
struct HostModule;
impl ModuleDef for HostModule {
    fn declare<'js>(declarations: &Declarations<'js>) -> rquickjs::Result<()> {
        declarations.declare("call")?;
        Ok(())
    }
    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> rquickjs::Result<()> {
        let function = ctx
            .remove_userdata::<Function<'js>>()
            .map_err(|_| Exception::throw_internal(ctx, "host bindings borrowed"))?
            .ok_or_else(|| Exception::throw_internal(ctx, "host bindings unavailable"))?;
        exports.export("call", *function)?;
        Ok(())
    }
}
