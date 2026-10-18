# Local runtime environment

`env` identifies a local runtime: `prod`, `dev`, or `e2e`. It is independent of
build profile and the cloud endpoint. Startup fixes it from the runtime workdir:
the standard runtime root is prod, its `dev` and `e2e` children select those
respective environments. Other custom workdirs retain the prod default. Standard
Dev/E2E directories must not be symlinks redirecting into another environment.

A missing wire field or TXT property means prod. Prod writers omit `env` to keep
existing peers compatible. Non-prod writers must include it. Empty, null, unknown,
non-string or malformed TXT values are rejected; they never default to prod.

Host, Hub and Node discovery uses the existing prod service types. Dev/E2E append
`-dev`/`-e2e` to the component service name and also publish `env` in TXT. Browsers
use only their own service type and validate TXT before caching candidates.
Host discovery snapshots and explicit commissioning URLs obey the same boundary.

The environment is authenticated at the existing connection/bootstrap boundaries:

- Host signed challenges, claim/administrator grants, access tokens and receipts;
  every management binding includes its environment.
- Core setup/pairing requests and responses, including the signed Host TLS binding.
- Hub WebSocket connection proofs; the environment is part of the signed payload.
- Signed Node challenges and Hub-issued Node JWTs, including restored credentials.

Hub connection proofs bind the selected environment. Core commissioning TLS
proofs use the v2 framing and include environment plus runtime Space policy for
all environments. Receiving peers require their own environment before
using keys, provisioning, admitting credentials or executing management requests.
Existing non-prod credentials missing env require pairing again; they are not
automatically rewritten or deleted.

Host/plugin runtime management in non-prod requires a direct, authenticated Core
connection. Both Client cloud fallback/explicit cloud sends and Core cloud ingress
reject `/app/system/hosts/*` and `/app/plugin/*`, except the cloud-owned
`/app/plugin/catalog/list`. Prod cloud routing and ordinary business requests keep
their existing behavior. Env is not added to every business message, database row,
health reply or installation operation: those paths already use the owning workdir,
Host identity, protected local transport, or authenticated connection.
