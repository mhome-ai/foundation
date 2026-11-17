# Messaging route update outcomes

The route is authoritative. Save the new Space/settings before changing derived
delivery registrations. A rejected save leaves the old registrations and actions
untouched. Local route changes and old action invalidation share the existing
SQLite transaction; construct the receipt before committing it.

After commit, attempt the current registration and best-effort old-Space action
cleanup independently. Cloud optional cleanup has a one-second total wait budget. Its
failure does not fail the update or prevent a later update. Cleanup must not
invalidate new-Space action sessions. Action handlers still check the current
route and authorization; provider message/button editing is not a prerequisite
for saving settings.

Retain obsolete AppClient registrations during route changes and cloud route
removal. They are disposable discovery records, not authorization. An unconditional
delete by reusable Space/client key could arrive after a later registration and
remove it. Actual delivery must resolve the authoritative route and check current
identity, membership or shared grant. Revocation and unlink still change those
authoritative records; retaining a registration never keeps a revoked route active.

Explicit, authorized selection of a revoked personal route creates a fresh active
binding using the existing binding identity field. Ordinary inbound messages reject
revoked routes and direct the user to select a Space. Route lists hide explicit
access denial but report authorization-query failures instead of returning a
successful empty list.

If the current delivery registration fails after commit, `/app/messaging/route/update`
returns `partial` with the committed `route` and optional `applicationProblem`
(`MESSAGING_ROUTE_REGISTRATION_FAILED`). Full and Essential retain this outcome and
warning. Private-chat `/switch` likewise says the Space was selected and explains
that delivery registration failed. A user may apply settings or switch again.

This receipt is transient. It introduces no persisted route state, cleanup task,
automatic write retry or pending-operation gate. Reads do not retain the failure
after refresh, and a later update need not clean every historical registration.

Clients preserve a known committed receipt even if a subsequent read fails. On an
unconfirmed/transport failure, reread the actual route without replaying the write.
Directory indexes discover route IDs; resolve those IDs from the authoritative
primary rows before displaying their current Space. A response from an abandoned
UI session must not change the new session or display its mutation result.
