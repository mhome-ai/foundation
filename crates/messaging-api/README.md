# mhome-messaging-api

Transport-neutral request and response DTOs for `/messaging/*` management APIs. The crate owns
provider discovery, placement, connection, binding, and setup wire shapes. It intentionally
contains no provider SDK, transport, runtime lifecycle, persistence, or Agent integration.
