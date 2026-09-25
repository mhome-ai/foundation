# Person producer v2

Person belongs to the authenticated tenant and Space of one Core. All Space members
can manage profiles and evidence. Producers have exactly two operations: describe
an embedding contract and submit observations for authoritative identity decisions.
There is no legacy import, gallery download, acknowledgement, lease or separate
sample submission interface. This is a hard protocol cut; old requests are rejected.

Embedding contracts include the weight SHA-256, preprocessing, dimensions,
normalization and metric. A changed embedding contract requires a new ID. Matching
thresholds and algorithms belong to Core and are not negotiated with producers.
There is no cross-model vector comparison. Multiple model-specific clusters can
belong to the same Person.

Core recognizes an entire batch against one pre-learning snapshot, then computes its
learning plan. Snapshot computation holds no database transaction. A short write
transaction validates the Space epoch/revision before atomically applying samples,
crops, projections and idempotent decision receipts. Concurrent changes cause bounded
recomputation; old epochs are rejected. Retried evidence preserves its original identity
decision unless the referenced authority was removed or reassigned.

Known-person additions are pending review. Unknown evidence forms candidates; a
candidate needs four active samples from two producer tracks to become stable.
Provisional candidates participate in ambiguity checks but cannot establish identity.
Matching scores are not calibrated probabilities. Network errors and unavailable
recognition are never evidence that a person is a stranger.

Sample retention is bounded by track, cluster and Space. Better evidence can replace
weaker evidence from the same track without increasing its two-sample limit. Automatic
learning cannot replace a named person's approved samples. Rejecting evidence removes
its vector and crop while keeping a bounded receipt to suppress retries.

Reset/deletion/restore rotates the epoch. Reset clears only Person-owned profiles,
samples, vectors and crops. Camera events, recordings and notifications are separate
owners and remain unchanged. New live sightings may create unnamed candidates.
Unnamed evidence expires after 30 days without sightings. Media and samples share a
transaction. Explicit encrypted archives are scoped to the same Space and do not
participate in automatic cloud synchronization. No legacy archive policy conversion
is performed.
