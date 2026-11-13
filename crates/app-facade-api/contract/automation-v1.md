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
