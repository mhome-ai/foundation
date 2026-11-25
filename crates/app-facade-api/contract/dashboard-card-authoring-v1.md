# Dashboard Card authoring read

`/app/dashboard/card/authoring/get` accepts `{cardId}` in the selected Space.
It requires Space read access and follows the normal Space execution mode.
A missing, internal, ambiguous or unplaced Card returns `NOT_FOUND`.

The Facade owns the complete response: `spaceId`, `cardId`, `canvasId`, `title`,
`primaryViewId` and `views: [{id, name, patternId}]`. Nullable fields are explicit.
Views preserve order and identity. Weather views use `default.basic.weather`;
automation-only Cards expose a synthetic `automation` pattern without a view ID.
Unknown patterns remain null; consumers must not infer them from device names.
Device values, resource references, layouts and renderer implementation data are
excluded. Full and Essential preserve the same business fields.

This read does not install a skin, change the selected view or save a Preview.
The public developer entrypoint is `meow make ui card preview inspect`.
The CLI forwards this DTO without reconstructing Card semantics.
