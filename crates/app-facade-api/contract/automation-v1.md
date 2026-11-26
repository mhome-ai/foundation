# Automation controls and display names

These Space-scoped public operations accept user programs, including temporary Task Flows.
Generated Twin and Group programs remain internal resources.

- `/app/automation/start` accepts `automationId` and optional `timezone` and
  `locationSymbol` for execution text. Authenticated actor information is server-owned.
  Recurring programs are enabled; Task Flows are activated and triggered.
- `/app/automation/stop` accepts `automationId`, disables the program and cancels
  all its instances, including durable waits. Completed actions are not undone.
- Both return Full `data: {automationId, enabled, running}` with observed state.
  A short Task Flow may already be idle when start returns. Essential renders the
  same fields as YAML. Failures use the canonical error envelope, never `success:false`
  inside successful business data. State pushes may supersede a request result.
- `/app/automation/rename` accepts only Standard Automations (`AutomationUtil.is_std`),
  excluding Group entity IDs. Twin, Group, Temporary and other prefixed internal
  programs cannot be renamed here. Rename a Group through its Group operation.
  It accepts `automationId` and a nonblank `name`, and returns
  `{automationId, name}`. It changes display metadata without parsing, compiling,
  replacing script source, creating a snapshot or restarting a generation. Public
  list/detail and UI labels use this display name. Ordinary source edits preserve it;
  snapshot rollback restores the snapshot's configuration.
- `/app/automation/description/update` accepts `automationId`, `expectedScriptHash`
  and at least one of `description` or nonempty `varDescription`. It validates the
  exact source hash and parses the rewritten description. `name` is rejected; use
  rename. The script's authored root name remains available in the script itself.

Inputs are closed objects. The schema is `schema/automation.v1.schema.json`.

For `/app/automation/save` and `/app/automation/snapshot/rollback`, the definition
is committed before the optional attempt to restore the enabled state. If that
attempt is rejected or unconfirmed, the Facade returns `partial` with the
Automation ID and `restoreIssue` (`failed` or `unconfirmed`). Rollback also
includes the snapshot ID. Its `enabled` field is omitted when the post-write
observation is unavailable. A newer user stop or definition change supersedes a
pending restoration without changing the successful definition save. Read
current state before deciding whether to enable.

`/app/automation/delete` returns `success` when all targets complete and `partial`
when `deletedIds` and `failedIds` both contain IDs. A batch with no completed
targets returns `error` with its per-target details. These outcomes apply to direct
calls and prepared commits, in Full and Essential presentation.

A partially deleted definition remains available for deletion with the same ID.
Existing cleanup steps tolerate resources already removed by an earlier attempt.
An already absent and cleaned definition is a completed delete target.

## Single execution control and evidence

- `/app/automation/instance/list` takes `{automationId}` and returns
  `{instances: [{threadId, instanceId, revision, phase, startedAt}]}`. Only active
  runs are listed, oldest start first. `phase` is `EXECUTING`, `WAITING`, `QUEUEING`
  or `STOPPING`. Full times are epoch milliseconds; Essential uses Home time.
- `/app/automation/instance/stop` takes `{automationId, threadId, instanceId}`.
  It fences that instance, captures its execution facts durably, and closes it.
  It leaves other instances and future recurring triggers enabled. The response
  is `{automationId, threadId, instanceId, state, outcome}`. `STOPPING` means the
  executing worker has not completed capture; repeat the same request to observe
  completion. `FINISHED` reports the authoritative terminal outcome. A run that
  completed before the stop request keeps its actual outcome; ordinary success
  may already have discarded its details. Already performed effects are not undone.
  A started external call without a result is uncertain, not a confirmed failure.
- `/app/automation/diagnostic/get` takes `{automationId, cursor?, limit?}` and returns
  `{diagnostics, nextCursor}`. The default page size is 50, with a maximum of 100.
  Follow `nextCursor` until absent.
  Cursors are opaque and bound to the authorized Space and Automation. Metadata
  is ordered by frozen completion time (capture time while active), then thread and
  raw instance identity, all newest first. Each thread exposes its latest 10 retained
  terminal instances across revisions. Failed/interrupted, DEBUG and explicitly
  stopped/captured executions share this quota; ordinary successful
  runs create no run-history event and retain no completed execution detail; their
  checkpoint objects are reclaimed asynchronously. Last-run state remains separate.
  Active captures are additional. `availableAt` records first terminal publication;
  there is no `expiresAt` or age expiry. Reads/retries and richer updates never refresh
  terminal order. Internal storage may buffer about 20 records before batch trimming
  to 10; periodic cleanup handles lower volume and failed object deletion. Buffered
  older terminal records are unavailable through both list and direct-read APIs.
  Active and pending-publication bytes are protected until settlement/acknowledgement.
- `/app/automation/diagnostic/read` takes the execution identity and optional
  `cursor`, and returns `{logs, nextCursor}`. Follow the opaque cursor for older
  events. Pages contain up to 100 events with a target size of 256 KiB; a single
  larger event is returned intact. Pagination never removes stored content.
  Missing or damaged bodies produce an error; they are never represented as an
  empty successful page.

Events contain `sequence`, `timestamp`, `kind`, `operationId`, `level`, `message`
and `details`. Ordinary events describe actual action/read requests, their results,
and scheduled waits. Paired events share an operation ID. Schema-declared sensitive
request fields are excluded. DEBUG adds internal compiler/interpreter flow. JSONL
storage uses epoch milliseconds; Essential renders timestamps in Home time without
changing stored instants. Agent fetches source through the standard source interface;
execution records never duplicate the original Plan or source.

Management events and last-run state remain separate; ordinary completed runs do not
append activity history. Checkpoints persist only new immutable JSONL segments and a
constant-size head reference, plus runtime memory. An empty segment is not written.
The Plan is fetched by automation/thread and must match the run's revision; a missing
or changed Plan ends the old run. Generation and ownership checks still apply.
A process crash can lose events since its last checkpoint. Explicit stop confirms
completion only after available evidence has been saved, before checkpoint cleanup.
