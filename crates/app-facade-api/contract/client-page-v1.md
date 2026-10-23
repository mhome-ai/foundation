# Client page navigation

`/app/client/page/report` forwards the authenticated Client's observed navigation
snapshot to `basic.app.state.page` subscriptions in its focused Space. It uses
the FacadeCall envelope and spaceRead authorization, accepts no caller-selected
Client identity, and stores no second navigation state. The canonical payload
is `schema/client-page.v1.schema.json`; success returns `{}`.

`current` describes the topmost registered navigation view. A drawer requires
`base`, the underlying full page. Page-specific navigation identifiers belong
in Dynamic `payload`; drafts, page contents, transient menus and dialogs are
excluded. Unregistered views use `page: unknown`.

`basic.app.state.page` and `basic.app.command.openPage` use client-dependent
page discovery. Native functions request `/app/invoke/discovery.page`,
`/app/invoke/state.page` and `/app/invoke/command.openPage`. Commands reply only
when the target is observed active; timeout does not prove navigation was
cancelled. State reporting converges to the latest snapshot after reconnect;
it is not a durable log of every intermediate transition.

Persistent subscriptions bind `a:current` to a concrete Client before endpoint
hashing. Deploy the manifest, both runtimes and Client page host together.
