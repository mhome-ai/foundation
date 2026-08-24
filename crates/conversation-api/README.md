# mhome-conversation-api

Typed, transport-neutral DTOs for the client-facing mHome conversation API.

This crate owns wire targets, request and response bodies, user-visible message content, and
conversation events. It deliberately contains no Agent runtime, persistence, transport, surface
routing, or messaging-provider implementation.
