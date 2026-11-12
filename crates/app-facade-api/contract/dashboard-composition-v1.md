# Canvas layout and card composition

Canvas owns one versioned layout document for every Dashboard placement. Card
bodies own content, views, business references and size-only `defaultSizes`.
`cardIds`, absolute leaf positions and parent membership are read projections,
never independent stored placement authorities. Physical Device parentage does
not change this presentation model; arbitrary logical Cards can be composed.

The layout schema is `schema/canvas-layout.v1.schema.json`. Root coordinates are
Canvas-global; member coordinates are relative to their composition. Each item
has independent sm/md/lg layouts, limited to 2/4/6 columns respectively. Members
are leaf Card references, never nested compositions. IDs are unique and leaves
cannot be placed twice. Siblings cannot overlap; children must fit within their
outer rectangle. Coordinates use nonnegative signed-32-bit integers. The UTF-8
layout document is at most 256 KiB.

`/app/dashboard/canvas/content/get` returns all leaf definitions, `revision` and
`layoutDocument`. Observations remain independent and do not advance layout
revision. Opening the Dashboard or switching profiles never repacks a saved
layout. Default dimensions are used only when creating a placement.

## Commands

All three composition commands require current-Space write authorization and
`expectedRevision`. They return `{spaceId, canvasId, revision, layoutDocument}`;
creation also returns `createdItemId`. The backend owns validation and the whole
operation. A stale revision changes nothing. An optional `rootDrafts` array includes unsaved root placement changes for up to
three distinct profiles in that same commit. `rootDraft` remains supported for a
single profile; the two fields are mutually exclusive. Ordinary layout updates
may include `rootDrafts` for profiles other than their main `layoutSize`; organize
may include all edited profiles before arranging its selected profile.

- `/app/dashboard/canvas/composition/create`: `itemIds` selects two or more roots.
  Existing compositions flatten to leaves; leaf identities and current dimensions
  remain. Fixed-pass first-fit packing compares original, area, height and width
  order across available widths. Choose smaller bounding area, then greater
  width, with deterministic ties. No rotations, resizing or optimality guarantee.
  Place the resulting outer rectangle near the selection without moving others.
- `/app/dashboard/canvas/composition/ungroup`: `compositionId` expands members
  at outer-position plus local-position in every profile, preserving current size.
- `/app/dashboard/canvas/composition/layout/update`: `compositionId`, `layoutSize`
  and all `members` replace that profile's local layout. Alternatively `reset:true`
  repacks within current outer width and height. Omit `members` when resetting.

Ordinary drag, resize and organize also save Canvas layout and advance revision.
Organize treats a composition as one root, keeping its internal layout intact.
Group copy creates fresh container and Card IDs while retaining business refs;
move preserves member identity and internal layout. Cross-Canvas move commits
source and target Canvas documents together. Delete and resource cleanup prune
leaf references: one surviving member becomes an independent Card at its current
absolute position; zero members removes the container.

Existing data is converted explicitly with old writers stopped, a verified
backup and conflict checks. Runtime startup never converts legacy placement or
silently deletes a database. Cloud Canvas records stay in the existing `canvas`
table with unchanged primary keys. Existing records require an explicit in-place
conversion and a verified backup before cutover. Aliyun must have local
transactions enabled on that table; existing tables without the feature require
provider support. No replacement Canvas table is introduced.
