# HiveCore cockpit

The HiveCore cockpit. Derived from a Lovable export, converted to a Vite SPA, and
re-pointed at the real PatchHive domain model.

- Frontend: `http://localhost:5183` (container and dev), preview `4311`
- API: `http://localhost:8100`

```bash
npm install          # also available from the root npm workspace
npm run dev
npm test             # frontend API-contract tests
npm run build        # vite build (generates routeTree.gen.ts) then tsc --noEmit
```

## Deliberate divergences

**This is not a specialist product UI.** HiveCore is a control plane, not a
specialist product, so it keeps a distinct cockpit architecture (AGENTS.md). It
does not use `@patchhivehq/ui`, keeps its own honey/amber visual language, and is
dark-only rather than following the suite-wide `patchhive.theme` preference.

**SPA, not TanStack Start.** The export shipped SSR with `createServerFn`. HiveCore is
a single-operator console behind an API key: SSR buys nothing, and a Node server would
become a second place holding the operator credential — the export already had an LLM
call and a provider key living there. TanStack Router is kept; Start, nitro, and every
`.server.ts` are gone. The operator key stays in memory for the current page lifetime
and travels as `X-API-Key` to `VITE_API_URL` and nowhere else. Reloading requires a
fresh login; the frontend also clears keys left in Web Storage by earlier builds.

## What the data is

`src/lib/hive-data.ts` mirrors `services/patchhive-backend/registry/products/*.toml`
(identity, safety posture, declared capabilities) and `scripts/suite-common.sh`
(ports). It is not invented and it is not a placeholder for those facts.

Runtime values start unavailable and are populated only by live API responses;
activity arrays start empty. See
[docs/hivecore-architecture.md](../../../docs/hivecore-architecture.md) for the
canonical control-plane architecture.

## Promotion record

The cockpit was promoted to the canonical `products/hive-core/frontend/` tree on
2026-08-03 after its parity audit passed. The audit verified production build and
type checking, API-contract tests, Docker and CI wiring, operator authentication,
suite bootstrap, settings and repository policy, PR budgets, product dispatch,
run details, approvals, mandates, conductor decisions, work ledger, governance,
suite runs, runbooks, and Ask Hive. No missing workflow from the former production
frontend remained. The obsolete frontend trees and unused Lovable-export residue
were removed during promotion.

## Panels

| Panel | Backing state | Status |
| --- | --- | --- |
| Product registry | product manifests + `GET /api/products/runtime` | live columns come from durable typed background snapshots (B1, B2) |
| Live runs | `GET /api/products/runs` | durable materialized run index with explicit unavailable states (B2) |
| Suite timeline | service events + HiveCore ledger events | unified durable work, dispatch, and outcome activity |
| Contract drift | manifest vs `GET /capabilities` | live manifest/runtime comparison |
| Capability matrix | manifest + observed actions | manifest side real |
| Outbound capacity | `GET /pr-budgets` | live, with bounded leases and durable GitHub reconciliation state (B5) |
| Approvals | `GET/POST /approvals` | live, exact and single-use (B3) |
| Work ledger | `GET /work-items` + `/work-items/findings` | live concrete work, idempotent receipts, leased dispatch, approvals, and outcomes |
| Repository policy | `GET /repository-policies` | structured rows plus durable public opt-out sync evidence (B4) |
| Control Center | settings, policy, budgets, governance, products, health, startup | compact controls for pause authority, resources, smoke authority, reputation, and diagnostics |
| Mandates | `GET/POST /mandates` | live SQLite intent, lifecycle, limits, and autonomy (§3.6) |
| Conductor ticks | `GET/POST /conductor/ticks` | live bounded, leased, resource-gated discovery and downstream work dispatch (§3.7) |
| Blast radius | documented edges + `GET /blast-radius/:slug` | live ledger-derived affected work and finding counts over the safety graph (§3.8) |
| Run volume | durable materialized run index | live retained run totals and 24-hour window |
| Suite bootstrap | launcher + durable smoke/fleet jobs | start/stop first stack, ready/all launch, per-product lifecycle, logs, credentials, pairing, and all smoke tiers |
| Runbook history | runbook records + smoke/fleet evidence | live durable reads (B6) |
| Ask the hive | `POST /ask` | model call lives in Rust behind `PATCHHIVE_AI_URL` |
| Change log | service events + HiveCore suite ledger + product action events | durable sources merged into one ordered operator timeline |

## Implemented orchestration boundaries

- The conductor dispatches admitted SignalHive discovery, ingests concrete findings,
  and advances the resulting RepoReaper work through durable leased lifecycle states.
- PR headroom and live GitHub-rate evidence backpressure discovery. AI spend,
  sandbox slots, mandate spend, and owner politeness gate concrete work.
- Suite Runs supports ordered dispatch, explicit target fan-out, declarative TOML,
  and bounded fail-closed result-expression gates.
- The blast-radius drawer combines the documented dependency graph with live
  work-ledger and finding-receipt impact counts.

## Rules that still bind

- HiveCore and the launcher never return GitHub tokens, provider keys, product service
  tokens, or launcher secrets to a read. Values the operator enters are write-only,
  held only in the active form, and cleared after a successful save.
- No direct browser-to-AI-provider calls. `PATCHHIVE_AI_URL` via the Rust backend.
- Responses use the contract-v1 envelope `{status, data, error, meta}`.
- `VITE_API_URL` is the only configurable value that reaches the client.
