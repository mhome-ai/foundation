# Automation UI presentation reads

`/app/presentation/automation/names/get` accepts `{ids}` (1–100 selected IDs).
IDs are trimmed and deduplicated in request order. It returns `{automations: {id: label}}`.
Missing IDs are omitted. Labels use the saved preferred name, then the script name,
then the existing unknown-label fallback. Only selected definitions are loaded;
there is no status read, script parsing, history or relation expansion.
Generated-program labels may be read within the authorized Space. This does not
make their definitions or controls available through user-program business Facades.

Automation detail adds `runnable` from its already-loaded definition. Group entries
omit it. Automation status adds `corrupted` from its already-loaded status, and
`expireAt` for temporary programs from existing creation/last-run timestamps.
Full and Essential retain these fields; response detail does not alter read scope.
