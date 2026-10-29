# Integration UI presentation

`/app/integration/presentation/get` is a read-only App Facade for UI consumers.
Input is `{integrationId}` for one root Integration. Tenant, Space and caller
come from authenticated context; Space read membership is required. Empty IDs,
unknown fields and Provider IDs are rejected; missing Integrations are not found.
This operation is not registered in Agent, MCP or CLI catalogs.

The Space execution authority performs the complete read. Integration names and
icons include the root and every Provider in its visible catalog, regardless of
installation. `typeNames` labels entity types; `entityStandalone` indicates
independent entity presentation; `groupSupported` indicates type-level group
support. Neither boolean map describes an instance's active state. `twinNames`
labels the root's applicable production Twin templates, not entity instances.
Developer draft overrides belong to developer tooling, not ordinary App display.
`capabilities` uses the shared CapabilityDescriptor, including full IDs and
visibility. No scripts, entity instances, live states or connection data are read.

Collections are always present and empty when the model has no corresponding
metadata. Optional empty names/icons are omitted. Upstream failures propagate;
existing metadata cache refresh policies remain in effect. No installation,
entity synchronization or business writes are performed.

UI sends `control: {mode: "direct", responseDetail: "full"}` and receives
`{status: "success", data: ...}`. Essential retains the same business fields in
YAML `data`; this interface has no separate field reduction. Mutating control
modes are not supported. SDK indexes the capability array by full ID for UI use.
