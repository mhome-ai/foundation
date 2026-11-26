# Suggestion contract v1

This package declares the shared Suggestion data contract and proposed Facade
operations. The declarations alone do not register routes, enable scheduling,
execute models, or expose a UI. Runtime and product integrations are delivered
separately. Existing conversation and Now Attention protocols are unchanged.

## Responsibilities

- Foundation owns these DTOs, strict schemas and shared conformance fixtures.
- Agent owns SuggestionGenerator: prompts, allowed read/validation tools,
  analysis, type-specific generation and validation. Automation is the first
  production content type, not a restriction of the common framework.
- The backend responsible for a Space (Core or Lion) owns admission, budgets,
  authorization, persistence, run completion, feedback and business handlers.
- Tiger adapts shared Agent execution, model routing and billing for cloud
  Spaces. It does not own a second Suggestion database.
- Assistant, UI and future clients consume the same business Facades. They do
  not compose Automation saves, enablement and feedback writes themselves.

A local Space retains its run and Suggestion data locally even when its model
inference uses the cloud. Data follows the existing Space migration/deletion
boundary. Runtime integration must verify that boundary before enabling runs.

## Triggers, authority and payment

Only periodic generation and an authenticated user's explicit request start a
run. No event wakeups, per-Suggestion rechecks, observation states or observation
tasks are introduced. Expiration changes applicability, not scheduling.

The trusted `SuggestionInvocationContext` in conversation-api is independent of
chat surfaces and threads. Its trigger selects payment attribution:

| Trigger | Payer | Result owner |
| --- | --- | --- |
| `EXPLICIT`, verified requesting `userId` | That user | The Space |
| `SCHEDULED` | Current Space owner, resolved by the billing adapter | The Space |

For example, if owner Alice and member Bob share a Space, a periodic run bills
Alice; Bob's explicit run bills Bob. Transferring Space ownership changes the
owner resolved for future scheduled billing admission. The billing adapter must
freeze its resolved payer consistently with existing model-accounting rules;
the public input and model output cannot select a payer. Billing failure must
not silently select another member or account.

This is a payment rule, not an authorization grant. A scheduled service is not
an enrolled `ClientPrincipal::Space`, nor a user impersonation. Core/Lion must
authenticate the internal admission, bind its Space and current execution
ownership, check enabled settings/model/budget, and restrict reads and validation
to that Space. Explicit generation additionally checks the requesting user's
Space access. Shape validation cannot substitute for those checks.

## Generic content and validation

`SuggestionDraft<C>` contains title, summary, reasons, evidence, optional expiry,
and flattened content selected by `type`. `Suggestion<C>` adds backend-owned
identity, Space, run, version, lifecycle, millisecond timestamps and resolution.
The generator cannot assign those backend fields. A suggested expiry is checked
against admission/publication time before persistence.

Type names use `UPPER_CASE`. A registered type declares its strict content model,
validator and supported business actions. The content must serialize to an
object with the matching `type`, without shadowing common fields. Production
writers must reject unknown types and undeclared fields. The generic JSON Schema
leaves room for extension fields; passing it alone is insufficient for admission.
Validate the concrete registered type as well. Test-only `INFORMATION` fixtures
prove heterogeneous output without registering another production feature.

Extension needs are independent: a new input requires an authorized reader/tool;
a new output requires a content model, validator and appropriate consumer/handler.
Neither requires an independent scheduler or Suggestion table. Prompt changes
cannot authorize unknown executable types or arbitrary Facade targets.

Evidence preserves a source type, source identifier, optional observation time
and a concrete excerpt. The producer checks references against actual authorized
reads or already validated evidence. Missing, expired, truncated or unavailable
history must not be treated as proof that an event did not happen. Excerpts stay
with the Suggestion when raw history expires. They do not override current facts.

The Rust `validate()` methods enforce cross-field invariants as well as shape.
JSON Schema enforces portable shape/conditional rules; time ordering, same-Space
resolution actors, evidence provenance, business permissions and actual script
validity require host checks. Deserialization alone is not semantic validation. Nonblank text and identity
padding use Unicode White_Space, consistently with Rust `str::trim`; schemas
spell out that set because JavaScript `\s` treats NEXT LINE and BOM differently.

## Automation content

`AUTOMATION` adds `operation`, `automationId`, `baseScriptHash` and `script`.

- `CREATE` has no existing Automation ID or base hash.
- `UPDATE` requires both the existing ID and the hash of its source at generation.
- `script` preserves the complete existing AutomationScript object: main `script`,
  auxiliary `scripts`, optional `sdkVersion` and optional `parseResults`.
- The final source bundle must pass the existing Automation verifier before the
  Suggestion is published. Nonempty source or model-supplied parse results do not
  constitute verification. No Automation is saved/enabled during generation.
- Applying an update checks the base hash within the existing editing boundary.
  It must not overwrite an intervening user change after a separate stale check.

The first Automation handler declares `APPLY`: a newly created Automation is
saved and enabled; an update preserves the existing Automation's enabled or
stopped state within the editing boundary. A deliberately stopped Automation must
not be re-enabled by applying an update. UI labels state the resulting action.
Handling is one-shot. After an admitted action ends, its Suggestion is `RESOLVED`
regardless of success, partial success, failure or an unknown write outcome. For
example, saving the new Automation and failing to enable it records a `partial`
result with its Automation ID and the enable failure. The UI says what succeeded
and failed and offers the ordinary Automation entrypoint. It does not leave a
pending Suggestion, retry button, recovery task or compensation workflow.
Enabling a new task-flow Automation does not mean executing its task immediately.

Other type handlers declare their own actions. The common framework does not
equate acceptance with enabling or running a resource. UI opens the already
generated source without a new model call; handling may revalidate against
current resources and supported SDK rules.

## Run and result semantics

One Space has at most one active run. A duplicate admission returns that run with
`accepted: false`; its focus is not incorporated and its payer is not substituted.
The host owns `SuggestionRun` status and measured model usage. The generator
returns drafts and an optional bounded context summary, never a success claim or
its own accounting. Analysis, script generation and bounded repair share one
run budget: model calls, prompt/output size, tool results and deadline.

| Status | Meaning |
| --- | --- |
| `RUNNING` | Admitted; no published result IDs or completion time yet |
| `SUCCEEDED` | Completed; zero results is normal |
| `PARTIAL` | Completed with recorded error and any independently valid results |
| `FAILED` | No published results; recorded error |
| `CANCELLED` | Stopped with no published results |

The host settles publication and run state, so a run that publishes valid results
alongside a failure is `PARTIAL`, not `FAILED`. Failure preserves existing
Suggestions and the last successful context. A process restart does not replay
model reasoning or blindly rerun writes. This contract does not require an
unbounded run log, a retry service or catch-up runs for every missed period.

## Suggestion lifecycle and handling

`PENDING` means available for review, `RESOLVED` means one-shot handling has ended,
`DISMISSED` records rejection, and `EXPIRED` means no longer applicable.
Resolved/dismissed records require a backend resolution. Pending/expired records
have none. Resolution records the action, existing operation ID, time, canonical
actor, resulting resource IDs/feedback, and the full canonical `FacadeResponse`
in `result`. Handling lifecycle and business outcome are independent: `RESOLVED`
is not a success claim or proof that a physical device reached its target.
An unknown outcome uses the existing error/problem representation, not a new
status or recovery state. New action handlers always write `result`; older receipts
without it remain readable and must not be inferred to have succeeded. Summary
`outcome` projects only the recorded result status, without copying its details.

Handling and feedback bind `suggestionId` plus `expectedVersion` to what the
user reviewed. Business authorization and existing prepare/commit or idempotency
protocols remain authoritative. The backend checks expiry, current resources and
type-specific preconditions before performing the declared action. An invalid or
unauthorized request is rejected before admission; it does not consume the
Suggestion. Once admitted, handling may run at most once. Repeated requests return
the persisted receipt instead of repeating the business action; an unknown outcome
does not justify write replay. A host must not claim exactly-once execution across
an unconfirmed storage failure. The model cannot create its own handler or action.

Next-run context includes current facts, pending Suggestions and recent decisions
and actual results, plus a bounded summary. Do not append all old prompts forever
or replace fresh facts with an old model summary. Existing history remains in its
own business stores. Now is a presentation consumer, not a Suggestion content type;
its deterministic Attention selection remains a separate responsibility.

## Facade declarations

All operations use existing `FacadeCall` and `FacadeResponse`, including caller
authorization, operation identity, Full/Essential presentation and errors. Space
is supplied by authorized invocation context, not an input `spaceId` field.
The following are contract declarations, not a current support inventory.

| Target | Input | Business output |
| --- | --- | --- |
| `/app/suggestion/generate` | `GenerateInput`: optional nonblank `focus` | `GenerateOutput`: `accepted` and run |
| `/app/suggestion/generation/get` | `GenerationGetInput`: optional `runId` | `GenerationGetOutput`: current/latest run, or null before any run |
| `/app/suggestion/list` | `ListInput`: type/status filters, cursor, limit (default 50, maximum 100) | `ListOutput`: summaries and optional cursor |
| `/app/suggestion/detail/get` | `DetailInput`: `suggestionId` | Complete typed Suggestion |
| `/app/suggestion/handle` | `HandleInput`: ID, expected version, declared action | Canonical business result, also stored in the resolution |
| `/app/suggestion/feedback` | `FeedbackInput`: ID, expected version, `DISMISS`, optional comment | Updated Suggestion summary on completed success |
| `/app/suggestion/settings/get` | Empty input | `SuggestionSettings` |
| `/app/suggestion/settings/save` | `SuggestionSettings`: explicit `enabled` boolean | Saved `SuggestionSettings` |

An explicit unknown or no-longer-retained run ID returns `NOT_FOUND`; it must not
silently return an unrelated run. Lists are bounded and omit script bundles,
evidence and resolutions; detail reads return the saved complete content.
Business errors/partial or unknown outcomes use the canonical Facade result;
marking handling finished must not replace that result with `success`.

Settings belong to the authorized Space. Absent storage defaults to `enabled: true`.
The switch controls periodic generation only; explicit authorized generation remains
available. The UI places it in the existing Agent Settings page. Turning it off
prevents future periodic admissions; it does not delete results or replay/cancel
an admitted run. Settings follow the Space migration/deletion boundary. This adds
no per-Suggestion timer, follow-up task or separate scheduler per output type.

## Conformance

`fixtures/suggestion.conformance.json` is shared by Rust and strict AJV tests.
The execution identity has a separate shared corpus in conversation-api.
These tests cover shape, multiple types, complete bundles, backend-field rejection,
status invariants and payment selection. They do not prove runtime authorization,
script compilation, billing settlement, persistence or local/cloud integration.
