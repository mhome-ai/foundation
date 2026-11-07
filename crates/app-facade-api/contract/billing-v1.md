# Billing reads v1

All three App Facades run in the cloud under UserRead, using the authenticated user's identity. They accept no userId, tenantId or Space selector. A Space/background principal cannot read an owner's private account. These reads do not require an active Space.

| Target | Input | Business output |
| --- | --- | --- |
| `/app/billing/summary/get` | `{}` | Current plan, subscription status, balance and used credits (`Summary`) |
| `/app/billing/payment/list` | `{}` | `{payments}` ordered newest first; signed currency minor units and Unix milliseconds |
| `/app/billing/credit/usage` | `{fromDate?,toDate?}` | Personal payer's UTC daily credits by source, including tracking coverage |

Summary retains the existing consistent plan/balance read; it does not query payments or daily history. Payment list retains successful invoices and refunds. Daily usage defaults to the 30 days ending today in UTC, permits ordered ranges of at most 366 inclusive days, and clamps the start to tracking availability. Errors are not replaced with zero usage. The existing `/app/agent/credit/usage` is Space-wide and remains distinct.

Full returns the business DTO. Essential encodes the same DTO as YAML without extra reads. UI, Agent, MCP and CLI use these same targets and identity rules. Schemas and fictional examples are in `schema/billing.v1.schema.json` and `fixtures/billing.conformance.json`.
