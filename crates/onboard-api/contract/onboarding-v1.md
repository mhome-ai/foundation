# Product onboarding v1

Status: new source contract. The initial SDK implements the bounded device-owned
operation core; product transports, real grant issuers and installed product
adapters must be delivered before advertising an operational entrypoint. This
document is not a list of registered App/CLI routes. There is no legacy wire or
stored-state fallback.

[Operation authority](authority-v1.md) defines the signed grant, possession and
renewal profile. [Product lifecycle](product-lifecycle-v1.md) defines the unique
runtime owner and whole-product management/removal boundaries.

## Ownership and layers

The physical product owns its operation, shared Space binding and recovery. The
commissioning App collects input, performs physical pairing, obtains limited
authorization and submits the product request. It never sequences Host, Pod and
Wave writes. Multiple pairing, network handoff and status requests are one
protocol interaction, not multiple product additions.

Cloud and the selected Hub retain authorization and admission authority. The
product cannot authorize itself merely by declaring its roles. The issuer checks
the requesting user's actual permissions. A product operation grant cannot become
a general Space Client, Host administrator or package-installation credential.
The ordinary Host/Pod/Wave operating credentials remain separate.

No Integration ID, package choice, Entity description or installed business model
is required by this protocol. Wave grant confirmation and authenticated protocol
access complete its onboarding portion. Integration lookup, cloud acquisition,
contract compatibility and capability projection happen afterwards in Core.
Unknown business definitions are distinct from temporary acquisition failures.
Neither invalidates an existing binding. OTA capability changes do not change
physical identity or require a new onboarding operation.

## Product declaration

`Product` declares wire version 1, `ownership:space`, environment, root identity,
128-bit lowercase-hex installation generation, one to three unique components,
network capabilities and `canHostHub`. Only `host`, `pod`, `wave` roles exist.
Every nonempty subset is valid. `canHostHub` requires a Host and must reflect the
installed product's actual ability to run Core. It is not inferred from an OS.

Identity IDs and Space/Tenant/Hub IDs use 1..64 ASCII bytes from
`A-Z a-z 0-9 _ - . :`. Key IDs are `p256:` plus 64 lowercase hexadecimal digits,
the SHA-256 fingerprint of the verified uncompressed P-256 public point. Physical
pairing authenticates that point. A descriptor alone does not authenticate it.
Component declarations must be checked against the product's trusted installation
or firmware; neither a broadcast nor an App request can append another component.
The first increment supports same-machine/same-firmware products only.

`network.owner` is `external`, `product`, or `none`. External networking follows
the OS/container and cannot be changed or erased by onboarding. Product-owned
Wi-Fi has explicit trial/commit/rollback hooks. IP products expose LAN. Only a
pure-BLE Wave product uses `none` with BLE and no LAN. A product with neither
transport is invalid. No unsupported adapter may report a successful empty scan.

The descriptor digest is SHA-256 of a sequence of UTF-8 strings. Each string is
prefixed with its byte length as an unsigned 32-bit big-endian integer:

1. `meow.onboard.product.v1`;
2. environment (`prod`, `dev`, `e2e`), root ID, root key ID, generation;
3. each component's role, ID and key ID, sorted Host, Pod, Wave;
4. network owner, LAN bit, BLE bit, Hub-capability bit (bits are strings `0`/`1`).

Ownership and version are fixed by the domain separator. JSON property order and
component declaration order do not affect the digest. Secret input is never part
of this digest. Models/product images and Integration metadata are not identity.

## Requests and authority

The logical protocol offers describe, network preparation, start, status and
continuation. Actual routes and transport framing belong to their real adapters;
this contract does not register placeholder routes.

`Start` contains `plan` and `authorizationProof`. `Plan` contains wire version,
128-bit lowercase-hex operation ID, installation generation, descriptor digest,
`space:{tenantId,spaceId}`, and `hub:{intent:join,hubId,keyId}` or
`hub:{intent:first}`. First Hub intent requires `canHostHub` and explicit user
approval. It never replaces another Hub or treats a pending removal as an empty
Space. The cloud rechecks its current Hub generation and exclusive setup intent.

The authorization adapter must verify signature, trusted issuer, bounded lifetime,
onboarding purpose, possession binding, exact Plan and root key before any durable
acceptance or side effect. The issuer must bind the approved product descriptor
and Space to this operation. Structurally valid JSON is not authorization. Proofs
use a nonempty printable ASCII encoding, at most 8192 bytes, and never appear in Debug, status or persisted
public operation state. The SDK must not accept a caller-supplied `verified` flag.
Expired authority cannot start new effects. Reconciliation of an already committed
fact is distinct from starting another action. Renewing authority can only continue
the exact original operation and cannot change its target or component set.

The operation ID is an idempotency key, not authorization. Repeating the same
Plan observes/continues its operation; reusing its ID for changed input is a
conflict. A different operation cannot replace unfinished device work. A new
initialization never changes a completed product's Space. Combined-product transfer
needs explicit product reset; standalone-Wave confirmed whole-product removal is
the lifecycle exception, preserving its network. Authorization-sensitive status
and continuation still authenticate the caller, even if it knows the operation ID.

## Networking and transport handoff

Discovery belongs to the commissioning Client: local LAN discovery plus its own
BLE adapter. Initial Wi-Fi setup never asks Core or a BLE relay Plugin to scan.
The Client reports unavailable hardware, disabled Bluetooth and denied permission
separately; none is an empty successful BLE scan. With an existing Hub, adding a
Space product requires the Client's authenticated direct-LAN connection to that
exact selected Hub, not a cloud/tunnel connection. A Space without a Hub must use
the explicit first-Hub flow. Physical pairing, a discovered name or knowing a
Space ID cannot bypass these admission checks. User Pod/Admin Host onboarding is
separate and does not inherit this Space-only restriction.

Wi-Fi devices use protected Client BLE only for initial network setup. The BLE
phase carries the selected public Hub identity or explicit first-Hub network
permit, never User refresh tokens or component business credentials. The device
owns the network trial and persists the network only after its actual owner
confirms success. Ordinary Space devices must discover and verify the exact Hub
through the trial interface within 30 seconds after receiving an IP. Failure
discards the trial and preserves the previously committed configuration.

First Hub networking uses its cloud-approved intent rather than trying to find a
Hub that does not exist. An existing own-Hub box uses explicit local network
recovery, without satisfying its LAN test by discovering itself. Those exceptions
must be derived from verified device/cloud facts, not a caller boolean.

After network commit the commissioner establishes a fresh paired LAN session and
checks the same root identity, generation and operation. It submits the complete
product authorization once; it does not request each role's credential separately.
Already-networked products skip the BLE prelude. Pure BLE products use their
protected operational carrier and do not invent an IP or Wi-Fi requirement.

No promise to finish without the App applies before necessary inputs/authority
have reached the device. Afterwards a reachable authorized device continues
without keeping the commissioning App alive. Temporary transport or cloud loss
never erases product ownership or silently opens another claim window.

## Device-owned operation

The fixed action order is network verification, shared binding, Host authorization
(when present), target Hub verification/first-Hub initialization, Pod activation
(when present), Wave authorization (when present), and completion. The SDK skips
only absent roles. A first Hub completes cloud data import and its durable cloud
handoff plus local protocol readiness before dependent role activation. It does
not wait for every business Integration to become usable.

An action is durably recorded as pending before being dispatched. After restart
or uncertain output, reconcile the exact pending action before attempting a new
write. A timeout is not confirmed absence. Adapters must reuse their saved
candidate/claim/attempt; they cannot allocate replacement credentials merely
because the original reply was lost. Recovery never replays business commands.
External action IDs include the operation, installation and action generation so
late replies cannot advance another product operation.

One complete product result requires confirmed Host/Pod/Wave results for all
declared roles. Host confirmation means actual saved authorization; Pod means a
matching authenticated identity from the pinned Hub; Wave means confirmed grant
and authenticated protocol access. Business inventory/Integration failure does
not block this result. Completion is recorded before its reply is sent. Devices
must retain enough protected facts to recover lost replies without re-pairing.

`Status` exposes the Plan, phase, state, declared component results, bounded reason
code and `updatedAtMs` (UTC epoch milliseconds, not seconds; maximum JavaScript
safe integer). It contains no passwords, PINs, tokens or private keys. An App
cache or a successful request acknowledgment cannot manufacture completion.

## Storage, reset and upgrades

The protected product binding and operation journal have one writer. Use atomic
replacement/native durable primitives and confirm writes before reporting them.
An uncertain write stops new effects until protected storage is reloaded. Missing
or corrupt state is not automatic first boot. Product-local explicit reset is
required when valid facts cannot be recovered. The journal contains only safe
Plan/action facts; credentials remain in their actual owners' private stores.

Installation roles survive upgrades and product reset. Whole-product reset stops
the relevant workers, clears all component credentials and owned product data,
coordinates identity-generation replacement, and only then reports a clean reset.
An interrupted reset resumes before old credentials are used. External OS network
configuration and unrelated product data are outside its deletion boundary.
Device reset does not implicitly remove the cloud Hub registration: an Owner uses
the established Hub removal/recovery operation and receives explicit data-loss
consequences. Reset must not silently replace a registered Hub with the same ID.

There is no distributed transaction or automatic rollback of durable grants.
State needed for uncertain-result reconciliation is retained; ordinary failures
do not clear healthy components to make results appear atomic.

## Initial implementation boundary

The protocol/type/schema corpus and portable action/recovery engine are the first
source delivery. They do not provide production grant signatures, BLE framing,
LAN listeners, Cloud endpoints, Core admission or an installed product. Those are
separate mandatory consumers, not optional permissive fallback implementations.
No capability table may advertise product support from this package alone.
