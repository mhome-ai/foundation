# Business authorization

`manifest/authorization.v1.json` is the sole operation policy catalog consumed
by MeowCore and Lion. Rust uses `authorization::required`; Lion loads this same
file from the checksum-pinned protocol package into its runtime resources.
Both run `fixtures/authorization/decisions.json`. There is no permissive default.

The verified principal is either User or Space. User operations require a User
and retain their domain ownership checks. Space operations use READ, WRITE and
MANAGE: current members have WRITE, owners have MANAGE, and a Space principal
has WRITE only in its exact tenant and Space. A supplied owner role never
promotes a Space principal. Personal credentials, enrolling users and billing
accounts cannot substitute for the principal.

The `nodePrefix` is explicit delegation, not a wildcard execution grant. Static
operations take precedence. The authoritative Core must also enforce the exact
operation policy from the currently authenticated Node registration before
forwarding. MCP grants and Agent tool allowlists are additional restrictions.

User and Agent entrypoints use the same policies. Agent execution retains its
verified originating principal, including through cloud forwarding, prepared
actions and retries. Each execution rechecks current membership; confirmation
does not freeze or elevate permissions. Core durable actions store a typed
principal; cloud personal actions bind the verified User. Core records missing
their principal are rejected without legacy conversion.

`DelegatedCall` is private to the authenticated Lion-to-Core bridge. Its
`authority` is either `caller` with the original principal, or `service` for
the exact `serviceOperations` allowlist. Client input never accepts this
envelope, and Agent forwarding cannot select service authority. `MwsClientInfo`
is routing metadata, not the authority for these business calls.
Internal `/dispatch/app/*` calls accept direct domain execution only. Public
`/invoke/app/*` forwarding instead carries the complete `ForwardedCall` and enters
the same Core action orchestration as LAN calls. These are distinct private
operations, not compatibility formats. The latter preserves Client identity or
trusted MCP/Agent invocation binding and returns a tagged JSON response containing the complete Facade result.
User Client cloud ingress and MCP require a User principal. A verified Agent run
may carry its original Space principal only back to that bound Space; it does
not issue a cloud credential to a Space Client or invent an acting User.

Prepared state belongs to the operation's business executor. Changing Client
transport does not create a second action record. MCP's cloud continuation owns
its interaction/grant lifecycle, not a copy of Core business state. Agent adapters
carry the executor's prepared reference rather than querying a cloud action table.
Deploy this contract and its consumers together. Drain or expire pending
interactions before cutover; do not translate old references, migrate action rows,
or automatically replay operations after uncertain delivery.

Host administration retains its independent authentication and ownership checks;
the User policies on Host operations do not grant Host administration. Space Pod
enrollment requires the current Space owner, as defined by its commissioning
contract. User Pod approval uses the current personal login.

Old cloud dispatch envelopes and duplicated per-runtime policy tables are not
supported. Publish the Foundation package and synchronize consumers before a
runtime deployment. A Gradle source override is available only for explicit
joint source validation, never as an automatic production fallback.
