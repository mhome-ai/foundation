# Billing v1

All three read App Facades run in the cloud under UserRead, using the authenticated user's identity. They accept no userId, tenantId or Space selector. A Space/background principal cannot read an owner's private account. These reads do not require an active Space.

| Target | Input | Business output |
| --- | --- | --- |
| `/app/billing/summary/get` | `{}` | Current plan, subscription status, balance and used credits (`Summary`) |
| `/app/billing/payment/list` | `{}` | `{payments}` ordered newest first; signed currency minor units and Unix milliseconds |
| `/app/billing/credit/usage` | `{fromDate?,toDate?}` | Personal payer's UTC daily credits by source, including tracking coverage |

Summary retains the existing consistent plan/balance read; it does not query payments or daily history. Payment list retains successful invoices and refunds. Daily usage defaults to the 30 days ending today in UTC, permits ordered ranges of at most 366 inclusive days, and clamps the start to tracking availability. Errors are not replaced with zero usage. The existing `/app/agent/credit/usage` is Space-wide and remains distinct.

## Account units and plan codes

`totalBalance` is the period's credit allowance, not the remaining balance. The
remaining credits are `totalBalance - usedAutomation - usedChat - usedIntegration`.
These four summary fields and daily `rows[].credits` are raw account credit units:
**100 credits = 1 product token displayed by the App**. For example, `100000`
credits is 1000 product tokens; 100 used credits is 1 used product token. These
product tokens are not an LLM's input/output tokens. Keep the wire values intact;
convert only when explaining the product-token amounts to a user.

The current tier codes are `E` (Internal), `T` (shown as Free), `V0` (Free), `V1`
(Go), and `V2` (Plus). `currentTier` describes the current plan; `refreshTier`
describes the plan at the next refresh. Either field and
`stripeSubscriptionStatus` can be null; null means unavailable/absent, not a
confirmed Free plan or active subscription. `nextRefreshTimestamp` is Unix
milliseconds. Product names such as `plus` are not tier codes.

Payment `amount` uses Stripe invoice/refund minor units for the returned
`currency`, independently of account credits. Preserve the refund sign. Follow
[Stripe's currency rules](https://docs.stripe.com/currencies), including zero-
decimal currencies and the ISK/UGX two-decimal API exceptions; do not assume USD
or divide all amounts by 100.

Full returns the business DTO. Essential encodes the same DTO as YAML without extra reads. UI, Agent, MCP and CLI use these same targets and identity rules. Schemas and fictional examples are in `schema/billing.v1.schema.json` and `fixtures/billing.conformance.json`.

## Subscription commands

These cloud-only UserWrite Facades use the authenticated user's account without an active Space. Input cannot select another user. The current UI exposes these commands on Web/desktop, with confirmation before submission; native mobile keeps its existing hidden purchase controls. Agent/MCP tool catalogs and delegated execution do not expose these commands.

| Target | Input | Business output |
| --- | --- | --- |
| `/app/billing/subscription/checkout/create` | `{productId}` | `{url}` for navigation to Stripe Checkout |
| `/app/billing/subscription/cancel` | `{}` | Subscription state; cancellation takes effect at period end |
| `/app/billing/subscription/resume` | `{}` | Subscription state; automatic renewal restored |
| `/app/billing/subscription/change` | `{productId}` | Subscription state; upgrades start immediately, downgrades start next period, choosing the current tier removes a pending change |

Product IDs retain the existing case-insensitive `go`/`plus` and legacy `pro`/`max` aliases. Subscription output contains `currentTier`, `cancelAtPeriodEnd`, `currentPeriodEndTimestamp`, `scheduledTier` and `scheduledTierStartTimestamp`; timestamps are Unix milliseconds. The standard Facade envelope carries success/failure, without a second business `success` flag. Full and Essential project the same command result without additional reads.

The UI does not automatically replay writes on reconnect or timeout. Checkout keeps its persistent attempt, frozen parameters, Stripe idempotency key and recoverable session URL. Once accepted, Checkout continues after caller disconnect; a lost response does not prove payment creation failed. Existing subscription management, proration and reconciliation stay owned by the Stripe manager. Stripe SDK calls, hosted Checkout navigation and signed `POST /webhook/stripe` delivery remain HTTP(S); the retired `/api/v1/payment/stripe/*` user command controllers are not webhook endpoints.
