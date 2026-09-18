# mhome-llm-api

Canonical Rust model contract: messages, ordered content parts, model tool schemas, completion
requests/responses, finish reasons, usage and private opaque continuation. `service` defines the
`/llm/complete` invocation wrapper; node installation/management contracts remain in Core API.

A continuation records a versioned format, an opaque compatibility key and a losslessly serialized
provider payload. Its Debug implementation redacts the payload. Adapters own codecs and key
construction; callers must retain it with its original assistant message and must never show it as
user-visible content. A changed route starts a new model context instead of forging reasoning.

`schema/message.v1.schema.json` is the offline message contract. Conversation's self-contained
execution schema embeds the same definitions and has a drift test. Only Rust consumers currently
use this package, so it does not introduce an unused npm package.

## Invocation policy

Agent callers own `ModelConstraints`. `input` is a closed `image|video|audio|file`
list; `text` is implied and rejected if listed. True tool/structured flags require
confirmed capability; empty `input` and false flags impose no requirement.
Deployments validate `payload ⊆ constraints.input ⊆ capabilities.input` without
inferring requirements from payloads. Callers must declare their needs correctly.
`structured_output` remains a capability requirement, not an output format; callers
not using schema-constrained output leave it false.

`GenerationParameters` belongs to deployment model configuration. Resolve and freeze
it with the selected route; Agent cannot override reasoning or temperature. Explicit
settings are desired preferences. Adapters clamp reasoning effort onto the model's
supported list (nearest neighbor, ties pick the lower effort) and omit it only when
that list is empty. Saved preferences are not rewritten. Model-specific temperature
interactions remain adapter responsibilities.

Continuation payloads remain opaque, attached to the original assistant message,
and must survive persistence and subsequent tool/user turns without modification.
Adapters own encoding and route compatibility checks.

Version 2 removes the unused `ModelConstraints.reasoning` field. Move such settings
to deployment generation configuration. Unknown constraint fields are rejected.

Version 3 replaces `vision: bool` on `ModelConstraints` and `ModelCapabilities`
with `input: list<string>`. There is no `vision` alias. Artifact kind and MIME
must agree; adapters must not downgrade undeclared modalities to text.
