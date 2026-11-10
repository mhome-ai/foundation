# Reference picker presentation

UI-only Space reads use the standard FacadeCall and success/data/error envelope.
Both targets require spaceRead and follow spaceMode routing. They are not native
Agent, CLI or MCP tools. Full and Essential preserve the same business fields;
Essential represents them as YAML.

| Target | Input | Business output |
| --- | --- | --- |
| `/app/presentation/reference/type/list` | `{}` | `{types:[{entityType,name,category}]}` |
| `/app/presentation/reference/item/list` | `{entityType}` | `{items:[{id,name,entityType,category,capabilities,group}]}` |

Inputs reject unknown fields. entityType is a nonblank string, trimmed at the
boundary. category is device, widget, automation or entity, derived by the server.
Candidate names do not replace IDs. Empty capabilities and false group values are
preserved. No business input selects another Space.

Types come from installed and auto-installed Integration standalone declarations,
deduplicated and sorted by entityType. Empty types are not hidden by counting their
instances. Item reads select one type only. Widget reads preserve the shared global
catalog (including global Automation-owned Widgets), not the narrower Widget-page
visibility rules. Device and Entity groups retain their existing capability
resolution. Automation references include regular and temporary programs and
Automation groups, with the existing internal-object exclusions and display names.
Location and AppClient retain their dedicated discovery rules.

Reference reads omit room/enablement enrichment, group runtime state, Automation
status and script hydration. They do not load Widget values, histories, owner
names or full configurations. Metadata reads may still require underlying stored
rows and group capability resolution; this contract does not promise one DB query.

The UI lazily caches types and items by Space/type and merges in-flight reads.
Search remains client-side. Failed requests are retryable and are not successful
empty catalogs. Space changes discard late results. Message reference identity and
serialization remain unchanged. Legacy `/reference/*` request routes are retired.
