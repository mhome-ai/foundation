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
