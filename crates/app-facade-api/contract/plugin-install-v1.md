# Plugin installation

These platform-owned App operations run on the selected local Space's Hub. They
are App-only, require a personal User with Space management access, and retain
Host-owned authorization and identity pinning whenever Host management is needed.
A verified available service can be commissioned without Host software-management
permission or a Host installation receipt; it retains the existing Node admission
and Space management checks. Agent/CLI/MCP do not acquire a new
installation entrypoint. The shared schema describes business input and data;
requests and responses still use the canonical App Facade envelopes.

- `/app/plugin/install/prepare` accepts `{nodeType}`. It inspects Hosts reachable
  from this Hub. It immediately returns a `preparing` session; status eventually
  returns `prepared` with immutable per-Host actions or `failed` with an error.
  Cardinality is unresolved (`null`) while preparing and selection is disabled.
  `single` permits one selected Host; `multiple` permits one or more. An existing
  single binding prevents another add. Unsupported or unknown observations are
  ineligible, not treated as missing software. Compatible installed services are
  reused without an implicit update or repair. Starting a stopped service is
  explicit in the action shown to the User. Prepare never starts service work.
- `/app/plugin/install/start` accepts `{sessionId,hostIds}` and submits once.
  Repeated identical submission returns the existing session, including failure;
  a different selection for a submitted ID is rejected. Plans expire; no silent
  replanning occurs. Each Host independently installs if necessary, waits for the
  exact service and release, then commissions into this Space. Failure stops that
  Host's dependent stages. Other selected Hosts continue. Commissioning waits for
  the existing Space mutation lock before one attempt.
- `/app/plugin/install/status` accepts `{sessionId}` and reads the originating
  User/tenant/Space session. It never executes or retries a stage. `running` is
  acceptance/progress, not success. All selected targets must succeed for
  `succeeded`; mixed success/failure is `partiallySucceeded`, zero success is
  `failed`, and any unconfirmed target makes the overall state `unknown`.

Each Host retains separate installation, readiness and commissioning outcomes,
with errors and submitted task/session references. `notRequired` means the path
omits a stage; `notRun` means its prerequisite failed or execution was interrupted.
A service installed before commissioning failed remains installed. No rollback,
automatic retry, retry endpoint, or automatic resumption is provided. Polling an
existing task after a lost response is observation, never a repeated write. A Core
restart records interrupted work and preserves completed facts; it never restarts
installation/commissioning. Existing provisional-credential cleanup is retained.
Host tasks already accepted may continue independently; interruption is not proof
that installation failed. Warnings are preserved even when targets succeed.

Results omit credentials, signing evidence and private execution plans. Completion
of this operation does not force device/account/model setup.
