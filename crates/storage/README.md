# mhome-storage

Shared storage engine used by MeowCore and the Storage Plugin. `Catalog` owns logical Space Folders, immutable object identities and placements, per-Folder quotas and durable operation receipts. `Disk` owns an exclusive physical repository with a persistent UUID, staged streaming writes, SHA-256 validation and durable physical deletion. Product authentication, discovery, TLS, HTTP routes and Camera retention remain in their respective consumers.

New objects prefer an authorized, available external repository; otherwise they use the internal repository. Confirmed external preallocation failure may fall back before consuming input, after retiring the unused physical object ID. Once input transfer has an uncertain result, the same logical key remains pinned to its recorded repository and physical ID until recovery resolves it. Reads and deletes always use the recorded repository UUID. Existing bytes never migrate automatically.

Ordinary Folders have positive byte quotas and `reject` or `evictOldest`. There is no repository or Space aggregate quota. Reservations and exclusive victim claims coordinate concurrent uploads without serializing their byte transfers. Eviction starts only after the incoming object is fully uploaded and verified. No staging space fails the write without pre-eviction. If one victim deletion fails, the incoming object is not published; earlier successful deletions remain applied. Deletion failures retain object metadata and usage until physical deletion is confirmed.

Each Space has one reserved `artifact` system Folder. Its quota is null, it never auto-evicts, and ordinary management cannot edit it. Core blocks ordinary Folder sessions for it; scoped Artifact APIs retain control over its objects. `temporary` provides process-local memory entries with idle expiry. Logical unlimited capacity does not bypass actual disk space or safety headroom.

Writes and deletes run in owned tasks so dropping a caller does not cancel an in-progress filesystem mutation. Durable journals resolve lost replies and crashes. A rename without its ready receipt is recovered by validating its complete bytes. Scope removal fences new access and retains the catalog until every physical deletion succeeds, including offline repositories. Backend access during cleanup is restricted by the embedding Core's authenticated cleanup policy.

Both SQLite databases use complete `0001_baseline.sql` definitions and SQLx checksums. This release supports new databases only. Preserve existing databases and media as backups; import is a separate explicit operation. Never convert, erase, or repair old databases automatically at startup.

Core internal disk keeps 256 MiB of physical headroom. External Disk's default headroom is 5% of its filesystem, bounded to 2–50 GiB and capped at half the volume. An individual object must fit on one repository; filesystem capacities are never added together to claim a larger writable object.
