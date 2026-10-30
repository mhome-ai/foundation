# Agent LLM settings

`agent_llm` and `schema/agent-llm.v1.schema.json` own the public business DTOs.
The standard Facade response envelope carries these values in `data` for Full;
Essential uses the shared Agent projection and YAML presentation.

## Read ownership

| Target | Input | Result |
| --- | --- | --- |
| `/app/agent/llm/summary` | Empty | `Summary` for the active `llmMode` |
| `/app/agent/llm/provider/list` | Optional `llmMode`; defaults to active mode | `ProviderList` for the requested mode |
| `/app/agent/llm/provider/get` | `provider`, `llmMode` | `ProviderDetail`, used by UI Operations and native Agent verification |
| `/app/agent/llm/mapping/list` | Empty | `MappingList` for the active `llmMode` |

Reading a mode does not activate it. Provider reads do not initialize mappings.
The summary has no embedded inactive Local settings. A local Space's UI can read
Local providers explicitly while its active LLM mode is Cloud. A cloud Space
cannot hold Local provider configuration.

Provider catalog facts and saved configuration are distinct. `configuration` is
present for Local providers and null for platform-managed Cloud providers.
`hasCredential` means a stored credential exists; it is not a remote authentication
check. `ready` and `readinessByUseCase` describe discovery and runtime availability.
Public credential settings contain only region, credential profile and workspace
ID. Keys, tokens, transport settings and provider model aliases remain private.

Models appear once in `providerModels`, or in `models` for a single provider.
`contextWindowTokens` preserves catalog information; unknown values remain null.
`generationControls` describes controls supported by both model and execution
backend. Metadata source is independent of LLM mode: Local provider catalog
metadata can be supplied by the cloud.

## Writes and mode selection

Mode selection changes the active configuration. It does not delete or overwrite
Cloud mappings, Local mappings or Local provider credentials. Mapping writes use
the active mode. Credential changes always target Local configuration, require a
local Space and a human UI/CLI caller, and do not switch modes. Native Agent
UserActions hand off to the Provider Operation; MCP and external Skills cannot
mutate provider credentials.

The frontend settings controller owns the authoritative loaded snapshot. A failed
mode write retains it. A successful write clears the previous mode's snapshot
before reloading; reload failure reports that the write succeeded and offers retry.
Provider Operations own a separate single-provider controller and never fetch the
whole summary or session policy.

## Presentation

Full preserves the public business DTO, including nullable fields. Essential
omits absent values and internal catalog/configuration metadata while retaining
mode, provider identity, configuration/readiness, model choices and actionable
model controls. Both surfaces preserve result status, warnings and errors.
