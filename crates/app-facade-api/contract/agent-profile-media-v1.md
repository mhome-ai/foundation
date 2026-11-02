# Agent Profile media

Memory and Identity stay Space-shared Markdown strings. Public get responses use
`{ content }`; save keeps its completion-only success envelope. The same Facades
serve UI, CLI, MCP and native Agent workflows; only the native update tool merges
new facts before calling save. There is no separate image-profile DTO.

An actual CommonMark image node `![description](meow-artifact://...)` is a retained
reference. Reference-style images are supported and canonicalized inline. Prose,
plain links and fenced/inline code are not image ownership declarations. Image
labels are descriptive; adjacent prose records the approved identity or fact.
No XML tag, signed delivery URL, local path or embedded binary is persisted.

The authoritative Space owner validates each reference's tenant, Space and Image
kind, reads its source bytes and creates an independent manual/standard Artifact
with a random Profile creation identity. Each document keeps its own reference
set. Existing references owned by that same document are reused. New references
from another document are copied, so deleting one document's reference cannot
invalidate another's. Source expiration/deletion does not affect committed copies.

Text, owned URIs, pending retired URIs and a new internal revision publish in one
conditional write. A stale save fails and requires a new read/reconciliation. A
known rejection cleans newly created copies only; an uncertain publication error
never deletes potentially committed content. This revision is not a public merge
protocol and does not prevent a caller from submitting already-stale text.

After publication, context is invalidated and removed owned images are deleted.
Failed retired deletions remain recorded and are retried on the next save. The
save worker completes publication/cleanup after caller cancellation. Process
termination before publication or failed cleanup of unpublished copies can leave
unreferenced Artifacts; there is no global reference collector or cross-store
transaction. Storage failures never justify deleting possibly committed content.

Callers re-read after saving to obtain canonical rewritten URIs. Full reads return
structured `{content}` and essential reads serialize the same complete text,
including multiline Markdown. Neither mode embeds image bytes. Native `media`
Skill `artifact_resolve` attaches actual Image content to the next model request;
CLI hosts must resolve and view an output file or supported expiring image URL;
MCP returns its existing image content block. Resolving is scoped, not public access.
