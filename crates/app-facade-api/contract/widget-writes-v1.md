# Widget writes

Widget create and configuration Facades are shared by UI, Agent, CLI and MCP.
Creation requires displayName; description may be omitted. A supplied description
must remain nonblank. Existing text value inputs remain supported.

## Data runtime values

`/app/widget/data/update` accepts widgetId and exactly one non-null input:

- value: a text-encoded scalar, for example "false" or "25C".
- values: an array of text-encoded values, including an empty array.
- typedValue: typed JSON, for example false, 0, [], or
  {"_type":"Temperature","value":25,"unit":"C"}.

Explicit typedValue:null is rejected; clearing is a separate Automation capability.
The stored definition determines type, scalar/array shape, options and bounds.
Object selections are restricted to stored Automation candidates. Only their type
and UID identify a candidate; stored business fields replace caller-supplied fields.
Both text and typed inputs use the same manager, write constraints, history and events.
The request never changes configuration, visibility or ownership.

## Circadian runtime controls

`/app/widget/circadian/runtime/update` accepts widgetId and at least one of curve,
offset, multiplier. Unknown fields and explicit null controls are rejected.
Curve must identify an existing schedule; offset must match the stored data type;
multiplier must be finite. Updates retain existing manual adjustment timing and
recompute the current and next values. Automation-owned parameters are operable.
Visibility, owner, name, location and schedule definitions remain unchanged.
This is a direct operation, independent of prepared definition edits.

Data value updates, Button triggers, Circadian runtime controls and delete return
an acknowledgement: {"status":"success"}, without business data.
Create/configuration responses retain action, widgetId, type and displayName.

## Deletion

`/app/widget/delete` retains the required explicit safe flag. The UI passes false
to preserve its previous deletion behavior. This migration does not change the
shared safe=true protection for Widgets referenced by Automations.

All legacy public /widget/* request Controllers are retired. /widget/reportUpdate
is a push event and internal /dispatch/widget/* operations retain their own roles.
