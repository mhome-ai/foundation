# Person v2

Core owns the Space-wide library and all identity decisions. Producers describe an
immutable embedding contract and submit live observations. There is no gallery download,
review workflow, separate sample submission, or observation replay.

An observation expires after five minutes. Core returns its current recognition decision
and a transient retained/skipped learning result. No decision receipts, rejected evidence,
or request history are persisted. Retained samples alone carry their source ID for deduplication.
Capture timestamps and source device IDs are not retained in the library or archive.

Core reads a consistent snapshot, releases the database transaction, and computes matching
and learning in memory. Matching projections are derived, never stored separately.
A short transaction validates epoch/revision and applies useful changes atomically.
Reliable identification of an unknown Person refreshes one last-seen timestamp,
coalesced within one minute. There is no sighting history. Touches do not change
matching/card revisions, retain new photos, or refresh known people.
Card revisions describe the representative photo, name and identity assignment;
hidden sample changes only invalidate the internal Space snapshot revision.
All decisions in a batch use its pre-learning snapshot. Unavailable or ambiguous recognition
is not evidence of a stranger.

Each group retains at most ten vectors with corresponding metadata-free face crops of
at most 256 by 256 pixels. Track quotas and similarity prevent redundant evidence from
filling the group. Learning uses stricter thresholds and ambiguity checks than recognition.
Naming anchors the representative the user saw; automatic learning must also match this
protected sample and cannot replace it. Only a materially better photo changes the card.

Each retained stranger group is created atomically with a Person. IDs use `h:<uuid>`
and survive naming. `kind` is `unknown` until a member names the person, then `known`;
recognition certainty is separate. A four-digit uppercase hexadecimal code is unique
among current People in the Space and supplies `Stranger AC2F`. The code remains
reserved while the Person exists, even after naming. IDs are not recycled. Unknown
People have one cluster; known People may have multiple model/pose clusters.

Members see one representative card per group, and can name, rename or remove it. Names
are labels, never identity keys. Equal names do not merge identities. Vectors from different
models are never compared. No raw samples or review operations are exposed by management.

Unknown People and all their bounded evidence expire together after thirty days
without reliable identification, initially counted from creation. A one-minute grace
covers coalesced last-seen writes, so retention is never shortened. Old photos may
remain while the Person is active. Uncertain/provisional matches do not refresh the
clock. Named People remain until removed. Deleting the final group erases its Person;
new evidence after deletion creates a new identity. Reset only clears Person data.

Both known and confidently matched unknown people return `identified` with the
canonical Person ID. `stranger` means no matching identity; `uncertain` includes
ambiguous/provisional matches. Creating a Person during learning cannot rewrite the
same batch's pre-learning decision. Base Person values have `id == uid`, `displayName`
and `kind`; matching evidence stays private to Core.

Explicit encrypted backup contains only the current library, including naming anchors.
Transfers are temporary, scoped to the member and Space, and closed or expired. The archive
format is v2 and includes last-seen clocks and reserved short codes. Restore preserves
those clocks; older archive versions are rejected before replacing any data. Person data is never registered with general cloud data synchronization.
