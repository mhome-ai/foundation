# Native Plugin Preview preflight

`/app/plugin/preview/check` is a read-only App Facade on the selected Local
Space's Hub. It accepts the empty business input `{}` and requires Space Manage
access. Its business result is `{ "spaceId": "<selected-space>" }`, inside the
standard Facade response envelope. Unknown input fields are rejected.

The desktop CLI must complete this check before asking its local Host to install
or restart a native Preview. Cloud Spaces cannot route this Hub-only operation.
The check creates no session, token, reservation or persistent data. It does not
grant Host installation permission: the Host's existing local authorization still
applies. Binding rechecks Space authority after installation; this preflight does
not guarantee that permissions cannot change during the operation.

Integration Preview status remains a Space Read operation. Native Preview must
not use that status read as evidence of management permission.
