# HiveCore deck (frontend-v3)

The HiveCore cockpit. Derived from a Lovable export, converted to a Vite SPA, and
re-pointed at the real PatchHive domain model.

- Frontend: `http://localhost:5183` (container and dev), preview `4311`
- API: `http://localhost:8100`

```bash
npm install          # also available from the root npm workspace
npm run dev
npm run build        # vite build (generates routeTree.gen.ts) then tsc --noEmit
```

## Deliberate divergences

**This is not a v3 product UI and is not meant to become one.** HiveCore is a control
plane, not a specialist product, and stays outside the v3 migration (AGENTS.md). It
does not use `@patchhivehq/ui-v3`, keeps its own honey/amber visual language, and is
dark-only rather than following the suite-wide `patchhive.theme` preference.

**SPA, not TanStack Start.** The export shipped SSR with `createServerFn`. HiveCore is
a single-operator console behind an API key: SSR buys nothing, and a Node server would
become a second place holding the operator credential — the export already had an LLM
call and a provider key living there. TanStack Router is kept; Start, nitro, and every
`.server.ts` are gone. The operator key stays in `localStorage` and travels as
`X-API-Key` to `VITE_API_URL` and nowhere else.

## What the data is

`src/lib/hive-data.ts` mirrors `services/patchhive-backend/registry/products/*.toml`
(identity, safety posture, declared capabilities) and `scripts/suite-common.sh`
(ports). It is not invented and it is not a placeholder for those facts.

Runtime values start unavailable and are populated only by live API responses;
activity arrays start empty. See
[docs/hivecore-architecture.md](../../../docs/hivecore-architecture.md) for the
remaining architecture work.

## Panels

| Panel | Backing state | Status |
| --- | --- | --- |
| Product registry | product manifests + `GET /products` | manifest data real; observed columns need the poller (B1, B2) |
| Live runs | `product_runs_index` | needs materialized run index (B2) |
| Suite timeline | `suite_events` | ledger does not exist yet (§3.3) |
| Contract drift | manifest vs `GET /capabilities` | needs the poller (B2) |
| Capability matrix | manifest + observed actions | manifest side real |
| Outbound capacity | `GET /pr-budgets` | live, with bounded reserved and committed leases |
| Approvals | `POST /approvals` | not built — this is what unblocks dispatch (B3) |
| Repository policy | `GET /repository-policies` | endpoint exists; needs structured rows (B4) |
| Mandates | `GET /mandates` | not built (§3.6) |
| Blast radius | work items + mandates | not built (§3.8) |
| Run volume | `suite_events` | ledger does not exist yet |
| Runbook history | smoke tiers + fleet jobs | backend exists; fleet jobs need durable storage (B6) |
| Ask the hive | `POST /ask` | model call lives in Rust behind `PATCHHIVE_AI_URL` |
| Change log | `suite_events` | `product_action_events` should fold into the ledger |

## Rules that still bind

- The browser never receives GitHub tokens, provider keys, product service tokens, or
  launcher secrets.
- No direct browser-to-AI-provider calls. `PATCHHIVE_AI_URL` via the Rust backend.
- Responses use the contract-v1 envelope `{status, data, error, meta}`.
- `VITE_API_URL` is the only configurable value that reaches the client.
