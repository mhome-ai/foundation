# Create Widgets and add them to a Canvas

`/app/dashboard/card/widget/create` is a direct, Space-write operation executed by
the backend that owns the Space. Dashboard clients call it directly. It takes a destination Canvas, a title, and one
or more ordered Data or Button Widget definitions. A definition uses the
corresponding standalone create fields, with no existing Widget ID. Dry runs
are unsupported.

```json
{
  "canvasId": "canvas-home",
  "title": "Evening",
  "widgets": [
    {"type": "button", "definition": {"displayName": "Evening", "label": "Run"}},
    {"type": "data", "definition": {"displayName": "Target", "dataType": "Number", "repeated": false}}
  ]
}
```

Validate the definitions, destination, and prospective card before writing.
Create the Widgets in order, stopping at the first failed or unconfirmed write.
Publish a single Card in the Canvas only after all Widgets and Card references
are ready. Any Widget whose creation is confirmed remains available as an ordinary
Space resource. To place existing Widgets, use `/app/dashboard/card/add` with
`source.kind: "widget"` and their existing IDs.

Success returns `data: {spaceId, canvasId, createdWidgetIds, cardId}`.
If at least one Widget was created and the entire operation did not complete,
return `partial` with `data: {spaceId, canvasId, createdWidgetIds, issue}`.
The issue identifies `stage` (`widget` or `card`), `outcome` (`notCommitted` or
`unconfirmed`), and the zero-based `widgetIndex` for a Widget write. A rejected
operation before any confirmed creation is an error. If the first write fails or is
unconfirmed, the error uses `TEMPORARY_UNAVAILABLE` with the issue in `error.details`.
Full and Essential retain the same facts. Essential renders the receipt as YAML.

Notification failures do not undo a confirmed write. After an unconfirmed
response, read Dashboard and Widget state before creating anything again.
