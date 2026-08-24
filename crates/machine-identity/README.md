# mhome-machine-identity

Persistent local machine identity derivation and stable host naming shared by mHome runtimes.

The crate owns no network, database, cloud, or product-service behavior. Callers choose the identity
file location and remain responsible for its lifecycle.
