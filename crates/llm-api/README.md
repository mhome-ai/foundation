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

Agent callers own `ModelConstraints`: true requires confirmed capability, false
imposes no requirement. Deployments validate these declarations without inferring
requirements from payloads. Callers must declare their needs correctly.
`structured_output` remains a capability requirement, not an output format; callers
not using schema-constrained output leave it false.

`GenerationParameters` belongs to deployment model configuration. Resolve and freeze
it with the selected route; Agent cannot override reasoning or temperature. Explicit
settings are desired preferences; adapters may omit unsupported settings without changing saved preferences. Model-specific support
and parameter interactions are adapter responsibilities.

Continuation payloads remain opaque, attached to the original assistant message,
and must survive persistence and subsequent tool/user turns without modification.
Adapters own encoding and route compatibility checks.

Version 2 removes the unused `ModelConstraints.reasoning` field. Move such settings
to deployment generation configuration. Unknown constraint fields are rejected.
