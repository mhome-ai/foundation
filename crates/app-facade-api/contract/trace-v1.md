# Linear execution attribution

Trace is an ordered array of flat items, each with an uppercase type and Unix-millisecond timestamp. Source kinds reuse Facade source kinds. The array records user-visible attribution boundaries; Automation instructions, conditions, branches and payloads are not trace items.

Ignite explicitly starts an Automation, trigger starts an ordinary run, and resume continues an existing run. Resume preserves the saved run chain. Internal asynchronous instruction completions do not append resume items. Trace uses run-level attribution, without per-branch causal graphs.

Command history stores an optional `r` reference. Snapshots are immutable and scoped by tenant and Space; IDs are positive signed 64-bit integers, represented as decimal strings in JSON. The trace and referencing command use the existing command storage owner. Recording is best effort: missing attribution cannot fail an operation or suppress its command history.

`/app/trace/get` accepts `input.traceIds`, at most 100 decimal IDs in the current Space. The result contains `traces` (ID to array) and `missingTraceIds`. Normal history reads do not implicitly resolve references. Entity and device history timelines return the command’s optional `r` unchanged as a decimal string; state and event entries omit it. Their `traceSteps` contains only legacy stored string sources, never a downgraded projection of a structured snapshot. Callers that need attribution batch the selected references through `/app/trace/get`; missing or expired snapshots leave the command visible. Full returns the original structured model with Unix-millisecond timestamps; Essential preserves its attribution fields and renders item times in the Space Home timezone. Authorization is Space read and routing follows Space mode. Legacy stored string sources remain readable without inventing per-item times.

Snapshots have a seven-day best-effort retention window. Local cleanup is opportunistic on writes; cloud providers use native TTL. Reuse is a small in-memory optimization, without a global deduplication guarantee.

Attribution is bounded to 256 items and 32 KiB encoded JSON. Oversized attribution is omitted. Consecutive resumes of the same Automation keep the latest resume timestamp in the new snapshot, preserving the original chain and previously persisted snapshots.
