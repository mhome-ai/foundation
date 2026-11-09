# General Entity reads

All reads use authenticated tenant and Space. Device, Automation and Widget types
belong to their dedicated domains. Location and concrete AppClient endpoints are
built-in Entity types; other types are Integration-defined.

- `/app/entity/list`: `{entityType}` returns `{entities: [EntityMetadata]}` for the
  exact type, including same-type Groups. There is one list, with no mode or refresh.
- `/app/entity/detail/get`: `{entityType, ids}` returns ordered deduplicated
  `{items: [{requestedId, found, reason?, entity?}]}`. `entity` is exactly the same
  metadata model as a list row; callers with known IDs need not list first.
- `/app/entity/state/get`: `{entityType, ids}` (1–100 nonblank IDs before
  deduplication) returns ordered `{items: [{requestedId, found, reason?, resource?}]}`.
  `resource` is `{entityId, latest: [{capabilityId, state?, event?, log?}]}`.
  These are latest persisted observations, without refreshing a Provider or deriving
  online status. Groups read their own observations without expanding members.
  Existing Location/AppClient endpoints return `latest: []`; missing resources use
  `found: false, reason: "not_found"` and omit resource. No observations is not missing.
- `/app/entity/history/list` remains independent and returns at most 100 records.

Metadata contains id, name, entityType, integration?, active, capabilities, data,
tags (Blueprint IDs), availabilityStatus?, issue, group and entityIds (Group members).
It contains no current capability values. Full retains the complete metadata.
Essential omits Blueprint tags, ordinary entityIds, absent optional values and
redundant built-in data identity fields. It preserves arbitrary Integration data,
including Provider-local data.id, falsy values and Group entityIds. State payloads
are not trimmed; only declared observation timestamps are localized by the shared
presentation-time rules. No arbitrary business-data timestamp inference is used.

List, detail and state respect source installation visibility. Built-ins are
scoped to the current Space. `/app/integration/entity/list` and its callers are a
separate source-catalog contract and are not changed by these reads.
