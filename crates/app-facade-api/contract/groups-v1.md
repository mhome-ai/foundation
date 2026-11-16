# Group operation state

A Group owns its definition, derived Twin, generated execution plans and related
cards. Deleting a Group preserves member resources. Member type is immutable.

`applyState` is persisted before changing derived resources. Validation and
preparation failures preserve the preceding definition. After a definition is
accepted, interrupted or failed application remains `applying` or `failed` until
another application succeeds. Controls require `ready` and a matching runtime
projection revision. `applying` alone does not establish that a worker is active.

A saved incomplete creation has a stable Group ID and complete member definition.
A user may reapply those IDs through `/app/group/members/update` with the current
revision, or delete the Group. Reusing the original `creationId` and payload also
preserves identity; it never overwrites later edits of a completed Group.

Deletion persists `deleting`, rejects rename/member updates and removes the Group
record last. Repeating deletion at the retained revision continues cleanup;
an absent Group succeeds. These operations do not require a background recovery
worker or a per-step task log. Normal reconciliation after member-source changes
continues independently.

Rename preserves runtime and does not require available members. Its saved name
and internal `presentationPending` marker are committed together before updating
Twin/card titles. The marker clears after all title writes succeed. An interrupted
rename therefore remains visible while ready controls remain usable. Saving the
current name again repairs titles; a successful full application also settles it.

List summaries, details and mutation results expose optional `applicationProblem`.
An incomplete persisted state yields a problem even when no exception was recorded.
A ready Group may carry a title problem. Mutation results with a problem are
`partial`, retaining Group identity, current revision and diagnostics in both
Full and Essential presentation. Read success means the definition was read;
inspect its state and problem separately. Member-resolution read failures retain
the existing `GROUP_MEMBERS_UNAVAILABLE` partial-read contract.

Notifications are best effort. A notification failure does not undo successful
writes. Rename invalidation follows the persisted final outcome. Clients may
refresh reads and expose user-initiated write retries. Existing member-source
reconciliation can reapply affected incomplete Groups under its existing rules;
this contract adds no separate recovery worker.

The cloud Group record gains one optional boolean attribute. Primary keys and
indexes do not change. Core SQLite includes `presentation_pending` in its complete
baseline; an existing database requires an explicit rebuild under the workspace
baseline policy. No startup conversion or automatic deletion is performed.
