# NFC management v1

The six shared App Facades use the canonical `control`/`input` request and
`status`/`data`/`error`/`warnings` result. Identity and Space authorization come
from transport context, never input. Source directory is cloud-owned; all other
operations use the Space's authoritative Core or cloud executor. The schema is
`schema/nfc.v1.schema.json`; Rust DTOs and the Lion Java binding are checked against
`fixtures/nfc.conformance.json`. Schema validates structural shape; normalization
also checks whitespace, source identity uniqueness and merged policy constraints.

| Target | Input | Full success data | Mode |
| --- | --- | --- | --- |
| `/app/nfc/list` | optional `createState`: active (default), pending, all | `{nfcTags:[{tagId,spaceId,name,description,policy,trustedSources,createState,createdAt,lastUsedAt}]}` | direct, Space read |
| `/app/nfc/sources/list` | `{}` | `{sources:[{sourceType,sourceId,sourceName}]}` | direct, Space read, cloud |
| `/app/nfc/create` | required name, policy; optional description, trustedSources | `{tagId,createState:"pending",writePayload}` | direct, Space write |
| `/app/nfc/activate` | `{tagId}` | no data; `{status:"success"}` | direct, Space write |
| `/app/nfc/update` | tagId and at least one of name, description, policy, trustedSources | no data | direct, Space write |
| `/app/nfc/delete` | `{tagIds:[...]}`, 1–100 unique IDs | no data | direct or prepared, Space write |

Name is trimmed, nonempty and unique case-insensitively within the tenant/Space,
including pending records. Duplicate create never removes or replaces pending.
Description defaults to empty; update omission preserves, empty clears, null fails.
Unknown fields and client-supplied lifecycle/timestamps fail.
Policy is ALL_PHONE or CUSTOM_PHONE. Sources have type phone and a nonempty unique
sourceId, plus optional sourceName (default empty). ALL_PHONE clears sources;
CUSTOM_PHONE requires at least one after patching. Source directory only includes
enabled registered phones advertising event.nfc. It is a discovery snapshot,
not an online status or runtime permission guarantee.

Create only creates a record. `writePayload` is `mhomeai://nfc/{spaceId}:{tagId}`.
A phone writes that exact content; activate is called only on physical success.
Only activation changes pending to active and sets createdAt. Repeating activation
preserves createdAt and lastUsedAt. Cancelled/failed physical writes explicitly
delete pending; read/list/sync do not clean it up. No TTL or background cleanup.
Update patches configuration without activating or resetting metadata.
Delete treats missing IDs as complete and returns an acknowledgement. SQLite and
DynamoDB delete a request atomically. Tablestore batch deletion can partially
apply before returning an error. Physical tags retain their written content.
Prepared preview includes requested IDs, found state, name and creation state;
last-use observations do not invalidate a preview.

Full list timestamps are Unix milliseconds. Essential retains all business fields
in YAML, localizes positive timestamps, and preserves zero as unknown. Mutations
with no business result return only canonical status in either detail mode.

Physical tap (`/nfc/tap` on Core, `/api/v1/nfc/tap` on cloud/mobile) remains an
authenticated native ingress, not a management Facade or CLI/MCP tool. It checks
Space, active state and source policy. Successful acceptance observes lastUsedAt
best effort; persistence failure does not reject the tap. List and sync are read-only.
