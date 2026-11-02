# Now attention

`/app/now/attention/list` is a Space-read Facade routed by Space execution mode.
Input is an empty object inside `FacadeCall.input`. Identity comes from the
verified caller, never an input user ID. Existing Dialog, Twin and reconnect events invalidate the view. Time boundaries
also refresh it while the page is visible.

All eligible items use one score:
`100 * relation * time * (0.625 * impact + 0.375 * necessity)`.
There are no source or severity tiers. Personal owner: 1; household: 0.8;
other personal owner: 0. Round scores to four decimal places, then sort descending
and by stable ID ascending for ties. The display threshold is 20.

Recipients control visibility and are not ownership. Without explicit business
ownership, shared household facts remain household facts, even if delivered to
one user. Personal ownership must be declared by the business source.

Decay uses the original occurrence time and a configured half-life; retries,
reads and redelivery must not reset it. Window relevance rises from zero at
start to one at peak, remains one until deadline, and is zero at deadline.
Persistent relevance stays one until the underlying condition is cleared.
Unknown observations do not clear a previously active condition. Expiration of a
request invalidates that request, but does not resolve a separate device issue.

The policy manifest and conformance fixtures are canonical across Rust and Java.
Unknown reasons and invalid time models have zero score. Policy updates do not
change business lifecycle state. Ordinary running, recent activity, scheduled
execution and notification priority alone do not establish attention eligibility.

The output contains items, evaluatedAt, nextRefreshAt, policyVersion and
sourceErrors. Source failures must not be represented as an authoritative empty
view. Actions continue through their business Facades with fresh permission and
state checks. Only confirmed completion removes an item; accepting a device
command is not confirmation that the condition was resolved.

Current adapters read pending Dialogs from their authoritative store, applying the
current audience, expiry and status on each read. Optional `attentionContext`
can declare `dialog.pending`, `task.userDecision` or `schedule.decisionRequired`.
Without it, pending Dialogs use household association and a 15-minute half-life.
Task failure, notification priority and schedule trigger times are not inferred
as user-decision facts.

Standard device facts currently cover `state.water` (`wet` / `dry`) and
`state.smoke` (`detected` / `clear`). Other readings, including unknown/testing,
do not create an incident and do not resolve an existing incident. The small
retained device projection preserves the episode across restarts and Space
execution-mode migration. Its revision is owned by Attention, independent of
rebuildable Twin cache counters. Each reconciliation reads current canonical
Twin state, including retained-history recovery, rather than replaying event
payloads. Device metadata and retained episodes cover cold/missing caches.

All timestamps are epoch milliseconds. `sourceUpdatedAt` is the underlying
state-change timestamp, not a heartbeat or proof of current connectivity.
`verificationRequired` marks a retained issue whose current reading is unknown.
Only one execution owner serves a Space; local and cloud candidates are not
merged into a second queue. Source errors remain visible alongside usable items.
