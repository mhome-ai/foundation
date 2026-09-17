//! Persistence contracts expressed in Agent consistency semantics.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::execution::{
    ConversationSurface, DurableEvent, InvocationContext, Message, QueuedRun, RunId, Scope,
    ThreadId,
};

/// Monotonic revision used for optimistic concurrency control.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Revision(pub u64);

impl Revision {
    /// Revision expected when creating a new thread record.
    pub const INITIAL: Self = Self(0);

    /// Returns the next revision, if representable.
    #[must_use]
    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// Fully scoped key for one durable Agent thread.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ThreadKey {
    /// Authorization and persistence scope.
    pub scope: Scope,
    /// Canonical Conversation-space identity and delivery route.
    pub surface_id: ConversationSurface,
    /// Thread identifier within the scope.
    pub thread_id: ThreadId,
}

/// Opaque control-state checkpoint bytes owned and versioned by Runtime.
///
/// Prompt-visible history is intentionally stored separately as an append-only journal so a
/// control transition does not rewrite every preceding message.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Runtime checkpoint schema version.
    pub format_version: u32,
    /// Serialized checkpoint payload.
    pub bytes: Vec<u8>,
}

/// Materialized Runtime state used when seeding or exporting one thread generation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeSnapshot {
    /// Small mutable execution-control checkpoint.
    pub checkpoint: Checkpoint,
    /// Prompt-visible history materialized from the append-only journal.
    pub history: Vec<Message>,
}

/// Current durable state of one thread.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThreadRecord {
    /// Current optimistic concurrency revision.
    pub revision: Revision,
    /// Latest Runtime checkpoint.
    pub checkpoint: Checkpoint,
    /// Prompt-visible history materialized from the append-only journal.
    pub history: Vec<Message>,
    /// Event committed with this generation but not yet acknowledged by the downstream consumer.
    pub pending_event: Option<DurableEvent>,
}

/// Prompt-history mutation committed atomically with Runtime control state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "messages", rename_all = "snake_case")]
pub enum HistoryMutation {
    /// Adds rows to the currently materialized history.
    Append(Vec<Message>),
    /// Replaces the materialized history after deterministic window truncation.
    Replace(Vec<Message>),
}

/// One atomic state transition requested by Runtime.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThreadCommit {
    /// Lease fencing token required by distributed implementations.
    pub fencing_token: Option<u64>,
    /// Replacement Runtime checkpoint.
    pub checkpoint: Checkpoint,
    /// Atomic prompt-history append or replacement.
    pub history: HistoryMutation,
    /// Exact pending event durably acknowledged by the downstream consumer.
    pub acknowledge_event: Option<crate::execution::EventId>,
    /// Event that becomes durable with the checkpoint revision.
    pub event: Option<DurableEvent>,
    /// Context indexed by distributed deployments while this checkpoint or its outbox needs recovery.
    /// `None` removes the thread from the runnable recovery index.
    pub recovery_context: Option<InvocationContext>,
}

/// Outcome of an optimistic thread commit.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommitOutcome {
    /// The transition was committed at the returned revision.
    Applied {
        /// Newly committed revision.
        revision: Revision,
    },
    /// Another writer advanced the thread before this commit.
    Conflict {
        /// Current durable revision.
        actual: Revision,
    },
    /// The writer no longer owns the distributed execution lease.
    StaleFence {
        /// Current fencing token observed by the storage implementation.
        actual: u64,
    },
}

/// Stable persistence failure independent of File, `SQLite`, S3, or `DynamoDB` details.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum StoreError {
    /// The requested state transition violates the storage contract.
    #[error("invalid durable state transition: {message}")]
    InvalidInput {
        /// Safe diagnostic message.
        message: String,
    },
    /// Durable state cannot be decoded or fails integrity validation.
    #[error("corrupt durable state: {message}")]
    Corrupt {
        /// Safe diagnostic message.
        message: String,
    },
    /// The authenticated scope cannot access the requested state.
    #[error("durable state access denied")]
    PermissionDenied,
    /// The storage dependency is temporarily unavailable.
    #[error("durable state unavailable: {message}")]
    Unavailable {
        /// Safe diagnostic message.
        message: String,
    },
    /// The adapter failed without a more specific stable classification.
    #[error("durable state failure: {message}")]
    Internal {
        /// Safe diagnostic message.
        message: String,
    },
}

/// Atomic thread state repository.
#[async_trait]
pub trait ThreadStore: Send + Sync {
    /// Loads the current thread record.
    async fn load(&self, key: &ThreadKey) -> Result<Option<ThreadRecord>, StoreError>;

    /// Commits a checkpoint and its optional durable event if `expected` and the fencing token are current.
    ///
    /// Distributed implementations must validate `fencing_token` against the authoritative lease,
    /// so a paused former owner cannot commit after a new owner acquires the thread.
    async fn commit(
        &self,
        key: &ThreadKey,
        expected: Revision,
        transition: ThreadCommit,
    ) -> Result<CommitOutcome, StoreError>;
}

/// Versioned process-local execution queue kept separate from both the durable
/// user waiting queue and the Runtime checkpoint.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct QueueSnapshot {
    /// Current optimistic concurrency revision.
    pub revision: Revision,
    /// Requests waiting to enter the Runtime loop.
    pub items: Vec<QueuedRun>,
}

/// Outcome of claiming the next queued request without destructively removing it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaimNextOutcome {
    /// The queue was empty at the expected revision.
    Empty,
    /// The head request was claimed and returned.
    Claimed {
        /// New queue revision after recording the claim.
        revision: Revision,
        /// Claimed request. It remains in process memory until `ack_started` succeeds.
        request: Box<QueuedRun>,
    },
    /// Another writer changed the queue first.
    Conflict {
        /// Current queue revision.
        actual: Revision,
    },
}

/// Process-local execution admission queue for one thread.
///
/// The deployment-owned user waiting queue is not this interface: `MeowCore` and
/// Lion persist it before dispatching one attempt into Runtime. Losing this
/// store loses the attempt and must never trigger cross-process replay.
#[async_trait]
pub trait QueueStore: Send + Sync {
    /// Loads the current queue snapshot.
    async fn load(&self, key: &ThreadKey) -> Result<QueueSnapshot, StoreError>;

    /// Enqueues a run idempotently by `operation_id` and `run_id`.
    ///
    /// Implementations preserve idempotency for the lifetime of the process.
    async fn enqueue(
        &self,
        key: &ThreadKey,
        request: QueuedRun,
    ) -> Result<QueueSnapshot, StoreError>;

    /// Claims the first unclaimed queue item at `expected` revision.
    ///
    /// `claimant` identifies the single process-local driver attempt. A lost
    /// attempt is failed by the deployment owner and is never reclaimed here.
    async fn claim_next(
        &self,
        key: &ThreadKey,
        expected: Revision,
        claimant: &str,
    ) -> Result<ClaimNextOutcome, StoreError>;

    /// Removes an item after the same run has been committed as the active thread checkpoint.
    ///
    /// Implementations must make repeated acknowledgements for the same run idempotent.
    async fn ack_started(
        &self,
        key: &ThreadKey,
        run_id: &RunId,
    ) -> Result<QueueSnapshot, StoreError>;

    /// Removes a queued run idempotently and returns the updated snapshot.
    async fn remove(&self, key: &ThreadKey, run_id: &RunId) -> Result<QueueSnapshot, StoreError>;
}

/// Process-local cooperative cancellation signal for a live execution attempt.
#[async_trait]
pub trait RunControl: Send + Sync {
    /// Requests cancellation idempotently.
    ///
    /// The full invocation context prevents collisions between users, scopes,
    /// threads, and runs inside one process.
    async fn request_cancel(
        &self,
        key: &ThreadKey,
        context: &InvocationContext,
    ) -> Result<(), StoreError>;

    /// Returns whether cancellation has been requested.
    async fn is_cancel_requested(
        &self,
        key: &ThreadKey,
        run_id: &RunId,
    ) -> Result<bool, StoreError>;

    /// Waits until cancellation is requested, allowing Runtime to drop an in-flight external
    /// operation and stop the execution attempt.
    async fn wait_for_cancel(&self, key: &ThreadKey, run_id: &RunId) -> Result<(), StoreError>;

    /// Clears the cancellation signal after a run has settled.
    async fn clear_cancel(&self, key: &ThreadKey, run_id: &RunId) -> Result<(), StoreError>;
}

/// Lease proving exclusive ownership of a thread run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunLease {
    /// Thread guarded by the lease.
    pub key: ThreadKey,
    /// Owning run.
    pub run_id: RunId,
    /// Opaque fencing token that increases across owners.
    pub fencing_token: u64,
    /// Absolute Unix expiration in milliseconds.
    pub expires_at_unix_ms: u64,
}

/// Coordinates exclusive execution of one thread inside one live process.
#[async_trait]
pub trait RunCoordinator: Send + Sync {
    /// Attempts to acquire a process-local lease for a run.
    async fn acquire(
        &self,
        key: &ThreadKey,
        run_id: &RunId,
        lease_ms: u64,
    ) -> Result<Option<RunLease>, StoreError>;

    /// Renews a lease if the caller still owns its fencing token.
    async fn renew(&self, lease: &RunLease, lease_ms: u64) -> Result<Option<RunLease>, StoreError>;

    /// Releases a lease if the caller still owns its fencing token.
    async fn release(&self, lease: &RunLease) -> Result<(), StoreError>;
}
