# Personal home navigation

The signed-in User owns two independent profiles, `compact` and `wide`, shared
across Web, desktop and phone Clients and independent of the focused Space.
`/app/user/navigation/catalog` takes an empty object and returns the canonical
`manifest/navigation-cards.v1.json` catalog. `get` takes `{layout}`. `update`
takes `{layout, cardIds, menuCardIds?}`. Both return
`{layout, cardIds, menuCardIds, revision}`. The authenticated caller selects
the owner; requests cannot select a User, Client, Space, URL or arbitrary page.

`cardIds` is the ordered Nav list. Dashboard, Now and Menu each occur exactly
once; Menu is always projected and stored last. Requests and stored profiles
from older Clients with Menu elsewhere are normalized, preserving all other
relative order. Additional entries are Menu card IDs from the catalog.
Membership makes the card's root route a home page; child editors and object
details retain their ordinary navigation lifecycle.

`menuCardIds` is the ordered list of remaining Menu cards. A complete edit supplies
both lists and replaces them in one document write. They must be disjoint and
together contain every catalog ID exactly once. Required Nav entries cannot move
to Menu. Unavailable cards remain in their saved lists/order when a Client filters
its UI, so a different Space/platform does not delete personal preferences.
New catalog cards absent from an older request or stored profile are appended in
default order. Unknown IDs, duplicates and overlap are rejected.

For backward compatibility, omitting `menuCardIds` changes only Nav membership
and order: retained Menu entries keep their saved relative order, newly unpinned
entries are appended, and newly pinned entries leave Menu. Older readers may
ignore the additive response field. The default Nav list is
`[dashboard, now, menu]`; the default Menu order is Devices, Automations, Widgets,
Integrations, Messaging Apps, NFC, Matter, Camera, Floor Plan, People, Storage,
Phone Setting. No startup data rewrite or separate Menu document is needed.

UI edits are drafts until the user clicks Done. Normal browsing does not reorder
or show per-card editing menus. Editing blocks page navigation, supports transfers
between Menu and Nav and sorting within both, and leaves Menu fixed last with a
thin separator. Dropping outside both lists changes nothing; touch scrolling
stays native until a held drag activates. Cancel discards the draft; a failed
save retains it for retry. Screen profiles remain independent.

Writes are last-write-wins per profile, without a caller revision precondition.
An opaque revision identifies a saved snapshot, not a Client clock. Each write
replaces one profile only. `/user/navigation/changed` invalidates that authenticated
user's profile across their connected Clients. Consumers re-read authoritative
state and also resync on reconnection/focus; notifications are not a second durable
store. Saving a profile does not require an App Client online and never performs
a page's business action.
