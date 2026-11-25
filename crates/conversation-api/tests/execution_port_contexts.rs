//! Existing adapters may omit the type argument; background adapters remain distinct.
use async_trait::async_trait;
use conversation_api::execution::*;

struct ChatAdapter;

fn unavailable() -> ExternalError {
    ExternalError {
        kind: ExternalErrorKind::Unavailable,
        message: "test adapter".into(),
        retry_after_ms: None,
    }
}

#[async_trait]
impl Llm for ChatAdapter {
    async fn model_profile(
        &self,
        _: &InvocationContext,
        _: &UseCase,
    ) -> Result<ModelProfile, ExternalError> {
        Err(unavailable())
    }
    async fn complete(
        &self,
        _: &InvocationContext,
        _: CompletionRequest,
    ) -> Result<Completion, LlmFailure> {
        Err(unavailable().into())
    }
}

#[async_trait]
impl ContextSource for ChatAdapter {
    async fn load(&self, _: &InvocationContext) -> Result<OperationalContext, ExternalError> {
        Err(unavailable())
    }
}

#[async_trait]
impl AppFacade for ChatAdapter {
    async fn invoke(
        &self,
        _: &InvocationContext,
        _: FacadeRequest,
    ) -> Result<FacadeResult, ExternalError> {
        Err(unavailable())
    }
    async fn prepare(
        &self,
        _: &InvocationContext,
        _: FacadeRequest,
    ) -> Result<PreparedAction, ExternalError> {
        Err(unavailable())
    }
    async fn commit(
        &self,
        _: &InvocationContext,
        _: PreparedAction,
    ) -> Result<FacadeResult, ExternalError> {
        Err(unavailable())
    }
    async fn reject(&self, _: &InvocationContext, _: PreparedAction) -> Result<(), ExternalError> {
        Err(unavailable())
    }
}

struct BackgroundAdapter;

#[async_trait]
impl Llm<SuggestionInvocationContext> for BackgroundAdapter {
    async fn model_profile(
        &self,
        _: &SuggestionInvocationContext,
        _: &UseCase,
    ) -> Result<ModelProfile, ExternalError> {
        Err(unavailable())
    }
    async fn complete(
        &self,
        _: &SuggestionInvocationContext,
        _: CompletionRequest,
    ) -> Result<Completion, LlmFailure> {
        Err(unavailable().into())
    }
}

#[test]
fn ports_keep_default_chat_implementations_and_object_safety() {
    let model: &dyn Llm = &ChatAdapter;
    let facade: &dyn AppFacade = &ChatAdapter;
    let source: &dyn ContextSource = &ChatAdapter;
    // Existing object users are exactly InvocationContext specializations.
    let _: &dyn Llm<InvocationContext> = model;
    let _: &dyn AppFacade<InvocationContext> = facade;
    let _: &dyn ContextSource<InvocationContext> = source;
    let _: &dyn Llm<SuggestionInvocationContext> = &BackgroundAdapter;
}
