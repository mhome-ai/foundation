# mhome-machine-identity

Persistent local machine identity derivation and stable host naming shared by mHome runtimes.

The crate owns no network, database, cloud, or product-service behavior. Callers choose the identity
file location and remain responsible for its lifecycle.

The standard runtime workdirs share a physical machine ID but have distinct Host IDs:
production `<runtime-root>` keeps `H`, `<runtime-root>/dev` uses `H-dev`, and
`<runtime-root>/e2e` uses `H-e2e`. Here `H` is the existing machine ID hash and the
identity file is `system/device_identity.json`. Runtime root is resolved by
`mhome-runtime-paths` (`~/.meow` on Unix). Unrelated/custom workdirs and direct raw
machine ID derivation retain the existing behavior. Build mode and cloud URL do not
select an environment.

Reads and atomic writes share a file lock across processes. Existing Dev/E2E machine
records receive the suffix once; their old Host credentials and product data are
**not** migrated. The Host rejects an old authorization record with a different ID;
that development/test installation must be reset and configured again. Production
records are left unchanged. Corrupt or unreadable identities fail instead of being
replaced by a new identity.
