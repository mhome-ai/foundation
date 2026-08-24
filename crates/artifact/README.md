# mhome-artifact-api

Storage-independent artifact references shared by mHome runtimes.

The crate defines and validates the `meow-artifact://v1/` URI format. It does not
contain local filesystem, network, database, or cloud storage implementations.
