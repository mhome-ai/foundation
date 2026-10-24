# Host registration directory

`/app/host/list` takes empty input and reads the caller's Admin Host registrations
and the current Space's Host registrations (Space member required). The result is
`{hosts, observedAtMs}`. Each row contains `hostId`, `hostName`, `env`, `mode`,
`registered: true`, `reachability: "unknown"`, `authorizationState: "unknown"`.

These are historical successful registrations, including offline Hosts. They do
not grant current access or attest that an administrator remains in the Host ACL.
Names are registration snapshots. No keys, tokens, signed challenges or ACLs leave
this directory. Native list composition adds current observations using existing
Host authentication; only authenticated live results establish current access.

A signed-in personal Client may omit the Space envelope to read only its Admin
registrations, including before its first Space exists. With a Space envelope,
Space membership is required before that Space's directory is read.
Commissioning, claims, permission prompts and runtime installation are unchanged
by this directory contract.
