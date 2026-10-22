# Client appearance

`/app/client/appearance/report` uses the canonical FacadeCall envelope and the
normal Space-mode, direct-only route. Its input is the six-field snapshot in
`schema/client-appearance.v1.schema.json`; success returns `{}`.

Only the authenticated, live UI Client may report its own snapshot in the Space
it is displaying. The installation and transport identity come from ingress,
never from input. Both User and Space Clients use the existing Space admission.
The report forwards to `basic.app.state.appearance` subscribers; it does not
create backend preference storage. Clients report after local settings or
capability changes, system appearance changes, and connection/focus restoration.

The public capabilities are `basic.app.state.appearance` and
`basic.app.command.setAppearance`. Discovery is part of those schemas. Backend
native functions invoke `/app/invoke/discovery.appearance`,
`/app/invoke/state.appearance`, or `/app/invoke/command.setAppearance` with a
Space-scoped server request. The Client replies once over the original connection
with the original target, signature, Space, and source peer. An error payload is
an invocation failure, including for commands without response data.

Changes validate the entire patch before applying it to the Client's persisted
Space preferences. Omitted settings are unchanged; no reset or separate options
operation is provided. `background` is the public name; opacity is retained for
Classic backgrounds while `cardOpacityApplied` is false. Presets come from the
target Client's discovery rather than a second backend catalog.

A persistent appearance subscription resolves `a:current` before calculating its
endpoint and storing its request. The first resolution is retained in condition
memory, so both re-evaluation and recovery reuse the concrete Client identity
without requiring the creator's transient connection context.
