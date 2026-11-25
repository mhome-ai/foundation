# Space Agent conversation policy

`/app/agent/conversation/get` reads and `/app/agent/conversation/save` updates the
current Space's default handling of messages sent during a run. The storage
identity is tenant + Space, never user or conversation surface. The ordinary
App Facade envelope and Space authorization apply.

Save input is `{ "runningMessageBehavior": "steer" }` or `"queue"`. The system
default is `steer`. Full data contains `spaceId`, `runningMessageBehavior`,
monotonic `version`, and nullable `updatedAtMs`. Essential retains behavior and
version. A successful save returns the committed policy and publishes it on
`/app/agent/conversation/changed` to the Space's connected App clients. Consumers
apply only newer versions and reload after reconnecting.

Conversation reads the current policy for each new default-delivery admission.
Explicit `steer` or `queue` overrides it for that message. An accepted request
retains its resolved delivery and target run across retries and later setting
changes. Existing personal idle-timeout policy is independent.

Steering keeps the current run, identity, model plan and cumulative usage. It
adds all waiting user inputs in order after the entire model tool-call batch
settles, before the next model request. Pending, unexecuted interaction cards
are superseded; old answers are refused. Any admitted participant in a shared
conversation can steer. Running tools are allowed to finish.

Public queued inputs carry `runId` when they are awaiting steering into an
existing run. The UI renders them as separate user bubbles in that run and omits
queue editing controls. Applied transcript messages retain their independent
`requestId` and the original `runId`, author, timestamp, and public attachment
metadata. The backend projects that data from its durable ingress record.

Execution control sends an ordered `inputs` prefix, each with its own request ID,
authenticated actor and portable message. Repeated prefixes are idempotent; a
newer prefix includes older unconsumed inputs so concurrent delivery cannot
reverse them. Live steering neither reseeds thread state nor waits behind the
running execution lane. Resuming an interaction carries its frozen plan, context
generation and checkpoint. If completion closes ingress first, the durable input
is handled as a subsequent task instead of being lost. A retried request ID
returns its first admission even when the Space default has since changed.
