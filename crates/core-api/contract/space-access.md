# Space business access

`space_access` defines the ordered levels READ < WRITE < MANAGE for one
Space. READ covers ordinary business observation. WRITE includes ordinary
business edits and effects, such as editing a dashboard, saving an automation,
controlling a device and running an automation. MANAGE covers identity, grants,
credentials, onboarding and infrastructure administration. It is not a global
account role or Host administrator grant.

An accepted user membership grants WRITE; an OWNER membership grants MANAGE.
An authenticated Space Client grants WRITE only in its exact tenant and Space.
Its enrolling user is never consulted. Passing an OWNER membership cannot
promote that Space Client. An absent membership does not grant a User access.
There is no additional per-Client permission setting in this model.

Credential validation precedes this pure decision. The ingress must validate
the authenticated enrollment and target binding, including the required Space
Client credential ID, before supplying a `ClientPrincipal`. Membership facts
must belong to the authenticated User and the exact target Space. Neither the
request body nor model output may supply a role or principal.
MCP grant/resource restrictions and Host authorization still apply separately.

Agent and relay adapters must retain the original principal. Tool confirmation
does not increase its business authority. A billing account is not an acting
principal and must never supply permissions to a device.

## Implementation boundary

This module is an additive, I/O-free decision foundation. It does not install
API classifications, authenticate Space credentials, change live admission,
change caches or replace User-only Agent/session contracts. Cloud and Core
adapters must consume the same reviewed operation policy before enabling the
new principal end to end. The policy of platform-paid calls from a Space Client
is not decided by READ/WRITE/MANAGE.

`tests/space_access.rs` covers the complete role/required-level matrix,
Space/tenant isolation, the Space Client's WRITE ceiling, missing membership,
invalid identities and rejection of unknown roles or the retired EXECUTE name.
