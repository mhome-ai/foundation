# Dashboard Card authoring read

`/app/dashboard/card/authoring/get` accepts `{cardId}` in the selected Space.
It requires Space read access and follows the normal Space execution mode.
A missing, internal, ambiguous or unplaced Card returns `NOT_FOUND`.

The Facade owns the complete response: `spaceId`, `cardId`, `canvasId`, `title`,
`primaryViewId` and `views: [{id, name, patternId}]`. Nullable fields are explicit.
Views preserve order and identity. `patternId` uses the same canonical identifier
as the Card DevKit and renderer binding. Blueprint IDs remain exact and case-sensitive
(for example `basic.device.onoff_brightness_color`); there is no UI-base prefix.
Weather views use `basic.weather`; automation-only Cards expose a synthetic
`automation` pattern without a view ID. Generic Widget data types use lowercase
intrinsic IDs (`Number` → `number`, `Button` → `button`, `Select[]` → `select[]`),
with terminal `_R`/`_W` presentation markers removed. `Circadian` maps to
`basic.circadian`, `TemperatureDifference` to `temperaturediff`, and `AutomationTitle`
to `automation`. These are Widget type projections, not Blueprint aliases.
Missing patterns remain null; consumers must not infer them from device names.
An unfamiliar Blueprint ID remains exact; this read does not invent its renderer contract.
Device values, resource references, layouts and renderer implementation data are
excluded. Full and Essential preserve the same business fields.

This read does not install a skin, change the selected view or save a Preview.
The public developer entrypoint is `meow make ui card preview inspect`.
The CLI forwards this DTO without reconstructing Card semantics.
