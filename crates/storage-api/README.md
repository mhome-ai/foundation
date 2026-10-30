# mhome-storage-api

Canonical Storage capability types shared by Core, plugins and App Facades.
Core/Node bindings and Host transport grants remain in `mhome-core-api`.
App entrypoints re-export these types from `mhome-app-facade-api`.
Database layout, physical IO, repository selection and recovery are implementation details.
See [the contract](contract/storage-v5.md).
