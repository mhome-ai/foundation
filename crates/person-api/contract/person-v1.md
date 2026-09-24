# Person v1

Person belongs to the authenticated tenant and Space of one Core. IDs never grant
access across scopes. All Space members can manage profiles and evidence. A camera
producer can describe its model, synchronize galleries, acknowledge revisions and
submit bounded evidence. The one-time legacy importer closes after migration and
cannot reopen after a reset. It is not a second profile-management interface.

Person IDs are independent of embedding spaces. Model contracts include weight
SHA-256, preprocessing, dimensions, normalization and metric. Registration binds an
explicit validated matching policy to the embedding space. A conflicting contract
or policy is rejected; a changed model/policy requires a new embedding-space ID in
v1. There is no automatic cross-model vector comparison. Multiple clusters from
different models can be assigned to the same person by a member.

Only Core decides cluster membership. Concurrent submissions are serialized before
matching; producers submit source evidence, never centroid/count replacements.
The shared matcher is pure, deterministic and has no IO or inference dependency.
Known-person additions are pending until reviewed. Unknown evidence forms candidates;
only stable candidates (four active samples from at least two producer tracks) enter
the recognition gallery. Review rejection deletes the vector and crop but keeps a
bounded receipt to suppress retries. Matching score is not a calibrated probability.

Gallery synchronization returns immutable pages of one epoch/revision, or deltas
including removals. A consumer atomically installs a complete snapshot. Caches must
be memory-only, bound to the authenticated Node instance, and expire within five
minutes of download initiation even offline. Missing/expired/incompatible galleries
mean recognition unavailable, never evidence that somebody is a stranger. Reset,
restore and person deletion rotate the epoch; writes captured before that boundary
are rejected. An offline camera may retain its leased gallery until expiry.

Delete/reset affects profiles, clusters, sample vectors, crops and Person caches.
Historical camera events, notifications and recordings are separate owners and are
not rewritten or deleted. Reset may retain non-biometric legacy migration fences so
old producer databases cannot silently repopulate a cleared library. New live
observations can create unnamed candidates again; reset is not a blocklist.

Media lives transactionally with Person samples in a bounded Core-owned store.
Unnamed evidence expires after 30 days without sightings; named evidence remains
until deleted. There are explicit per-track, cluster and Space capacity limits.
Archives are explicit passphrase-encrypted, authenticated, same-Space exports with
bounded chunked transfers. They do not participate in automatic cloud data sync.
Restore replaces only Person data and rotates the epoch. Archive format and encryption
version are independent from embedding-model contracts. All API errors fail closed.
