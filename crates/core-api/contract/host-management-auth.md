# Host management authorization

This protocol is separate from Client–Core business credentials. A User cloud
login identifies a person. A Host access token authorizes that person to manage
one installation of one Host. Space Clients never receive Host management tokens.

## Installation policy

The installed package fixes one of three modes; onboarding cannot change it:

- `admin` stores User administrators and may run multiple Spaces.
- `spaceSelf` stores its own immutable `{tenantId, scopeId}` claim.
- `spaceDep` derives its claim from a colocated Space Client's canonical binding;
  it stores no separate Space claim or administrator ACL.

Both Space modes are managed by the current cloud Space owner. No owner account
is persisted as a permanent local administrator. Reset rotates keys and binding
generation. Removing the final Admin is rejected; reset explicitly unclaims it.
Android uses `admin`; iOS is a management Client, not a background Host.

## Daily access

1. The Client obtains a signed public Host challenge, directly or through its
   current Space Core. Its nonce, Host identity, key, environment, mode, generation
   and claim are covered by the Host signature and the TLS key.
2. Authenticated User HTTPS requests `/api/v1/host/access/issue`; Web uses the
   equivalent cloud-owned `/app/host/access/issue`. Lion checks its trusted Host
   directory and current admin/Space owner authority.
3. Lion returns an RS256 JWT with type `meow-host-access+jwt`, issuer
   `meow-host-authority`, audience `meow-host-management`, unique `jti`, `sub`,
   `iat`, `exp`, `binding`, and explicit permissions. Lifetime is at most 300s.
   Binding contains Host ID, Host key ID, environment, reset generation, User ID
   and either the exact Space or null for Admin mode.
4. The Client sends `Authorization: Bearer ...` over pinned Host HTTPS. Core may
   forward this unchanged, after checking it belongs to the original User and
   current Space. Core is not a management principal or a credential issuer.
5. Host verifies the signature against its trusted cloud signing-key directory,
   purpose, issuer/audience, lifetime, identity/generation, mode, current local
   ACL and requested permission on every protected request. Signing keys are
   cached with bounded refresh and unknown-key retry; request verification does
   not perform a cloud lookup per operation.

Clients cache access tokens in memory until expiry. Issuance requires cloud
connectivity; an already-issued token remains usable until expiry after a network
loss. Expiry affects management only, not running Core/Node services. There is no
refresh token or offline authorization renewal. Logout fences in-flight requests
and clears the Client's Host state. Identity mismatch requires explicit recovery;
never substitute another route or key after a possibly executed mutation.

## Public and protected Host routes

Bounded status/challenge/first-claim bootstrap routes and the applicant endpoint
are outside management-token middleware. The latter accepts only a signed cloud
applicant credential for submit/read-own/cancel-own operations. Hardware
information, metrics, runtime/service installation and restart, administrator
changes and reset require access tokens. Permission declarations are `read`,
`manageRuntime`, `manageAdministrators`, `reset`. Space Host does not grant
`manageAdministrators`. Local OS administrative control remains a distinct
physical/local trust path.

## First claim

1. Native User Client discovers the Host on its LAN. Ethernet skips BLE; BLE
   provisioning can add a one-time claim token. Core has no first-claim proxy.
2. Client generates a canonical base64url 32-byte exchange secret and sends its
   SHA-256 digest in a fresh `/v1/auth/challenge` request. The secret stays local.
3. Client sends the signed challenge, explicit reset recovery flag and optional
   selected `scopeId` to Lion `/host/authorizations/claim`. Space mode requires
   the current owner; Lion derives the tenant from the User login. Admin mode
   has no Space selection. A Cloud Space can bind before its first Core exists.
4. Lion signs a five-minute `meow-host-claim+jwt`, bound to that exact signed
   challenge and exchange digest. Client durably saves the trusted Host pin and
   an unfinished-confirmation marker before submitting it plus the secret and optional BLE claim token directly
   to `/v1/auth/claim`.
5. Host verifies and atomically persists the claim, then signs a receipt with type
   `meow-host-claim-receipt+jwt`. Client verifies User, Host, key, environment,
   generation and exact resulting claim before forwarding the receipt.
6. Client submits the receipt to `/host/authorizations/commit-claim`; Lion commits
   the trusted directory with a compare-and-set against the original version.
   Only after confirmation does Client discard the unfinished-confirmation marker.

The transaction has a fixed ten-minute deadline from creation. An explicit retry
always uses a new challenge, secret and grant; no saved proof or receipt is
required. Host can confirm its existing binding for a current administrator or
the same Space, with current ownership checked by Lion. This never adds an
administrator or changes the Space. Exact receipt retries are idempotent only
within the original deadline. Expired transactions fail even if their database
TTL deletion is delayed. A replaced Host key or generation requires explicit
factory-reset recovery. Stale receipts cannot overwrite newer directory trust.

## Additional administrators

A User requests access from Hosts. Lion `/host/administrators/prepare` verifies a
signed challenge for an already claimed Admin Host and signs a ten-minute
`meow-host-administrator-applicant+jwt`. It binds the authenticated User, their
server-owned profile, Host identity/reset generation and random request ID. It
has no management permissions. The User submits it directly or through the
current Space Core to `/v1/auth/administrators` and can read/cancel only their own
request. Host keeps one live request per User; repeated submissions do not extend
its deadline. Rejected, cancelled or expired requests can be explicitly retried.

Any current administrator can list requests/members and approve or decline.
Approval atomically updates the Host ACL and persists a signed receipt before
Lion `/host/administrators/commit` updates the trusted directory. Either Client can
retry that confirmation within the fixed deadline without another ACL write.
Administrators are equal. Removal rechecks current authority and the last-admin
constraint under the same Host mutation lock; it invalidates that member's pending
approval receipt. A removed member must apply again and receive new approval.
There are no invitations or pairing-code administrator approvals. The local CLI may approve or remove administrators as an OS LocalOperator; it never turns an OS account into a cloud administrator.
These actions never reopen first TOFU claim remotely. Daily access tokens retain
the five-minute lifetime.

Native LAN identity recovery on an already claimed Admin Host submits an
administrator application, not another first claim. It returns `claimed: false`
and the Host application result until an approved receipt is confirmed. The
Client retains the explicitly accepted pin and an unfinished-confirmation marker;
a matching pinned challenge can resume that applicant transaction after restart.
Another key change requires another explicit confirmation. Core-routed discovery
cannot authorize identity recovery. Successful ordinary access clears the marker.

## Service business authorization

Host management controls service installation, update/removal and machine
administration. Running services enforce their own business permissions. A
Space owner can initialize its Hub on an existing Core without being a Host
administrator or presenting a Host management token. Admin Hosts may serve many
Spaces and Users without granting their administrator ownership of those Spaces.
Single-Space policy limits service admission to the bound Space independently
of Host management authorization. Installing a missing Core requires Host
management permission; using an installed Core does not.

## Single-Space service execution

Host derives the immutable `RuntimeSpacePolicy`: Admin → `multi`; claimed Space
Host → `single` with the exact tenant/Space; unclaimed Space Host → no service
launch. Every managed process receives required `--runtime-space-policy` JSON.
Packages cannot override it. Old binaries fail closed on the required protocol.
Core and Nodes check the policy at commissioning, credential persistence,
stored identity recovery, refresh/reconnect and request admission. Topology
`NodeInstancePolicy` remains independent.

Existing mDNS service names remain unchanged. Candidate records include
`space_mode=multi` or `space_mode=single`, `space_tenant`, `space_id`; missing or
malformed policy is not interpreted as multi. Discovery hides other-Space
candidates, but signed commissioning challenges and local admission remain the
security boundary. Core TLS commission proofs bind environment and runtime policy
with the `meow-hub-tls-v2` framing. Host discovery advertises `host_mode`, `claimed`
and the claimed Space where applicable; signed status establishes authenticity.

## Local OS operator

The local CLI uses a private Unix socket (service owner or root) or local Windows
pipe (owner, SYSTEM or Administrators). Its trusted transport creates an OS actor;
request JSON cannot select that actor. It does not require Client login or a cloud
token. App/remote HTTPS, including loopback, always uses cloud User tokens. Both
entrypoints use the same operations and last-administrator/runtime Space checks.

## Dependent product commissioning and reset

`<workdir>/security/device-binding.json` is a private, atomic, versioned record:
`env`, Client installation ID/key ID, Host installation ID/key ID, `generation`,
and optional `space`. The Client is its only writer. It contains no bearer token,
refresh token, password or owner ID. Host reads and validates it on authorization
and service reconciliation. Missing, malformed or mismatched records fail closed;
only a genuinely new Client installation may create the initial empty binding.

The App pairs directly with the product over its LAN initialization channel. BLE
only provisions Wi-Fi; Ethernet skips it. The owner selects a Space. The product
exposes a Host-signed challenge including `deviceBinding`, bound to the paired
Client and Host identities and generation. Lion uses the standard claim issuance
and owner check. The product's Client asks its local Host to verify the grant via
OS IPC, atomically writes the shared Space binding, then obtains the Host receipt
and sends it to the App for normal cloud confirmation. Independent `/v1/auth/claim`
cannot mutate a dependent Host. Only a paired Client can complete this product path.

After binding, Host can run preinstalled services with the single-Space policy.
If the selected Space has no local Hub, App initializes the device's Core using
the existing Hub setup state machine and the already verified Host TLS key. It
then requests the normal Lion-issued Space Client credential, delivered to the
product over the same secure LAN session. An existing Local Space uses its Hub.
Only the owner may initialize any Space Client, including a standalone one.
Same-Space retries reuse the binding, and changing Space requires product reset.

Either dependent Host reset or Client reset records one private
`security/device-reset.json` intent. Pending reset immediately disables Host
authorization and service execution. Client coordinates Host/service shutdown
and reset, waits for the new Host key, clears its credentials (and optionally
Wi-Fi), rotates its Client key and shared generation, and clears the intent last.
An interrupted step resumes on process restart. Missing/corrupt binding recovery
is allowed only under this explicit reset intent. Ordinary startup never repairs
or overwrites a lost binding. Both entrypoints reset the complete product.
