# Person producer v3

Core owns the Space-wide library and all identity decisions. Producers describe an
immutable embedding contract and submit live observations. The protocol is a hard cut.
No gallery download, review workflow, separate sample submission or observation replay.

An observation expires after five minutes. Core returns its current recognition decision
and a transient retained/skipped learning result. No decision receipts, rejected evidence,
or request history are persisted. Retained samples alone carry their source ID for deduplication.

Core reads a consistent snapshot, releases the database transaction, and computes matching
and learning in memory. Matching projections are derived, never stored separately.
A short transaction validates epoch/revision and applies useful changes atomically.
Ordinary redundant recognition does not write the database. No per-sighting timestamps are stored.
All decisions in a batch use its pre-learning snapshot. Unavailable or ambiguous recognition
is not evidence of a stranger.

Each group retains at most ten vectors with corresponding metadata-free face crops of
at most 256 by 256 pixels. Track quotas and similarity prevent redundant evidence from
filling the group. Learning uses stricter thresholds and ambiguity checks than recognition.
Naming anchors the representative the user saw; automatic learning must also match this
protected sample and cannot replace it. Only a materially better photo changes the card.

Members see one representative card per group, and can name, rename or remove it. Names
are labels, never identity keys. Equal names do not merge identities. Vectors from different
models are never compared. No raw samples or review operations are exposed by management.

Unnamed samples expire individually seven days after receipt. New sightings cannot renew
old samples. Empty groups disappear. Removing a group erases its samples and media, cleans
an unreferenced profile and rotates the epoch to reject in-flight old work. There is no ignore
list; future live evidence can create a new unnamed group. Reset only clears Person data.
Camera events and recordings remain separate owners.

Explicit encrypted backup contains only the current library, including naming anchors.
Transfers are temporary, scoped to the member and Space, and closed or expired. The archive
format is v3 and does not import prior review/receipt formats. Person data is never registered
with general cloud data synchronization.
