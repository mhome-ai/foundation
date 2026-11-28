# Product onboarding contract

`mhome-onboard-api` defines the new device-owned Space product onboarding
contract. The device SDK lives in the separate `meow-onboard` repository. This
crate contains types, validation, canonical fixtures and protocol documentation;
it implements no transport, issuer, device runtime or App Facade.

The seven nonempty combinations of Space Host, Space Pod and Wave share one
physical product identity, one Space binding and one device-owned operation.
User Pods and Admin Hosts cannot be components of these products.

Onboarding confirms identity, durable authorization and authenticated protocol
access. Integration selection, acquisition, Entity validation and capability
availability are independent business work. Missing or newly introduced
Integrations must never revoke a completed onboarding result.

See [onboarding v1](contract/onboarding-v1.md), the
[authority profile](contract/authority-v1.md) and
[product lifecycle](contract/product-lifecycle-v1.md). The claim types cover exact
operation binding, bounded UTC-millisecond lifetimes and fresh challenge binding.
`Product::operation_owner` selects the single installed runtime for all seven role sets.
Failures, restart and expired authority pause unfinished work for explicit user
retry of the same saved Plan. Necessary root possession proofs remain required;
there is no unattended device authority-renewal loop.
`validate()` checks structure
and relationships only; signature verification and operation authority remain
required at the device boundary. Fictional fixtures are not valid grants.

This is a new source contract, not a claim that existing product listeners accept
it. Existing controllers and products remain on their current protocol until the
device/issuer adapters and consumer dependency cutover are delivered together.
