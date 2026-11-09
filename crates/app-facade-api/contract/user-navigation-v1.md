# Personal home navigation

The signed-in User owns two independent profiles, `compact` and `wide`, shared
across Web, desktop and phone Clients and independent of the focused Space.
`/app/user/navigation/catalog` takes an empty object and returns the canonical
`manifest/navigation-cards.v1.json` catalog. `get` takes `{layout}`. `update`
takes `{layout, cardIds}` and atomically replaces that profile's complete ordered
list. Both return `{layout, cardIds, revision}`. The authenticated caller selects
the owner; requests cannot select a User, Client, Space, URL or arbitrary page.

Dashboard, Now and Menu must each occur exactly once. Additional entries are Menu
card IDs from the catalog. Membership makes the card's root route a home page;
child editors and object details retain their ordinary navigation lifecycle.
Unavailable cards remain in the saved order when a Client filters its UI.

Profiles default to `[dashboard, now, menu]`. Writes are last-write-wins per
profile, without a caller revision precondition. An opaque revision identifies a
saved snapshot, not a Client clock. Each write replaces one profile only.
`/user/navigation/changed` invalidates that authenticated user's profile across
their connected Clients. Consumers re-read authoritative state and also resync on
reconnection/focus; notifications are not a second durable store. Saving a profile
does not require an App Client online and never performs a page's business action.
