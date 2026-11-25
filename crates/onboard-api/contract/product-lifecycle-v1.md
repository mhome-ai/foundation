# Product ownership and lifecycle v1

This document completes the shared lifecycle boundaries of
[product onboarding](onboarding-v1.md). It does not register management routes.
User Pods and Admin Hosts remain standalone products outside this Space engine.

## One device owner

An installation has exactly one product operation owner, derived from its trusted
roles. With a Host, the Host runtime owns the engine, endpoint and private journal.
Without a Host, a Pod runtime owns them when present; otherwise Wave firmware owns
them. A combined firmware still instantiates one engine, not one per role. Native
role processes communicate with that local owner through authenticated private
IPC. The commissioning App never takes over this responsibility after a crash.

| Roles | Operation owner |
| --- | --- |
| Host | Host runtime |
| Pod | Pod runtime |
| Wave | Wave firmware |
| Host + Pod | Host runtime |
| Host + Wave | Host runtime |
| Pod + Wave | Pod runtime |
| Host + Pod + Wave | Host runtime |

The protected SDK record is the product's authoritative Space binding and operation
journal, with one serialized writer. Accepting a Plan reserves its immutable target;
confirming Binding makes the shared binding durable. A component's cached Space
and operating credentials are validated against that binding, not another source
of product ownership. Do not keep the legacy Pod-owned Client/Host binding record
as a second writer. Cloud/Hub admission records remain external authorization facts,
reconciled by the corresponding role adapter; the journal cannot manufacture them.

The binding action checks the installation and role-store preconditions before
the engine commits the shared binding. It does not create another authoritative
binding file. An uncertain SDK save freezes effects until its actual record is
reloaded. Old credentials may not start workers while reset/recovery is unresolved.

## Management and feature availability

A combined product has one ownership-management entry. It cannot be split into
separately owned Host, Pod and Wave products. Changing its Space requires explicit
whole-product reset. Enforce this at device and Cloud/Core mutation boundaries,
including old component-specific removal APIs; hiding a UI button is insufficient.
Component records carry their product identity/generation so Core and Cloud can
reject a single-component unbind of a combined product.

Functions can be disabled and reenabled within the existing Space. Disabling a
function changes its business availability, not the installed role declaration,
root binding, operation generation or credentials. For example, disabling Wave
entities on a Pod + Wave screen leaves the screen in its Space and does not make
Wave claimable elsewhere. Reenabling uses its existing authority and reconciles
ordinary connection state; it does not rerun physical pairing. An unavailable or
disabled function must not change a previously completed onboarding result.

The approved standalone-Wave removal behavior remains distinct: confirmed removal
revokes its only Wave grant and removes the whole standalone product's ownership,
while preserving network configuration. The lifecycle adapter fences old work,
rotates the installation generation and clears the matching root operation before
allowing another addition. A lost removal reply stays pending until revocation is
confirmed. This is whole-product removal of a Wave-only product, never permission
to remove Wave alone from a combined product. It is an explicit lifecycle operation,
not `start` with another Space or automatic initialization after a missing journal.

## Reset, upgrade and recovery

Whole-product reset first persists a reset barrier, stops all declared role workers
and fences their pending replies. It then clears role credentials and owned data,
rotates the installation generation and installs the new empty protected record.
Interrupted reset resumes before any old worker is started. Network clearing is
an explicit product-owned reset consequence; external OS/container networking is
never erased. Installation roles survive reset and normal software upgrades.

Reset does not silently remove a registered cloud Hub, replace it with another Hub
or claim that cloud data was deleted. The Owner uses the established Hub removal
or recovery operation and sees its data consequences. Reestablishing an interrupted
first Hub still requires the matching online, cloud-approved setup intent.

Healthy completed roles can exist while a later role is pending. The public product
outcome remains incomplete and reports the blocked phase; retries reconcile the
same facts. Do not describe this as a distributed transaction, promise all-or-nothing
rollback, or clear healthy credentials just to hide partial progress.

Required product tests include all seven owner mappings, two local writers, old
component unbind attempts, disable/reenable without re-enrollment, uncertain remove,
interrupted reset at each durable boundary, upgrade preserving roles, first-Hub
cloud completion before children, and standalone User Pod/Admin Host regressions.
These are consumer acceptance requirements, not claims about existing adapters.
