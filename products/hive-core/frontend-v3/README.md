# HiveCore deck (frontend-v3)

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

**This is not a v3 product UI and is not meant to become one.** HiveCore is a control
plane, not a specialist product, and stays outside the v3 migration (AGENTS.md). It
does not use `@patchhivehq/ui-v3`, keeps its own honey/amber visual language, and is
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
remaining architecture work.

## Panels

| Panel | Backing state | Status |
| --- | --- | --- |
| Product registry | product manifests + `GET /api/products/runtime` | live columns come from durable typed background snapshots (B1, B2) |
| Live runs | `GET /api/products/runs` | durable materialized run index with explicit unavailable states (B2) |
| Suite timeline | `suite_events` | durable event feed; not yet the unified work/outcome ledger (§3.3) |
| Contract drift | manifest vs `GET /capabilities` | live manifest/runtime comparison |
| Capability matrix | manifest + observed actions | manifest side real |
| Outbound capacity | `GET /pr-budgets` | live, with bounded leases and durable GitHub reconciliation state (B5) |
| Approvals | `GET/POST /approvals` | live, exact and single-use (B3) |
| Work ledger | `GET /work-items` + `/work-items/findings` | live concrete work plus idempotent source receipts; transitions remain (B6) |
| Repository policy | `GET /repository-policies` | structured rows plus durable public opt-out sync evidence (B4) |
| Control Center | `GET/PUT /settings`, `/repository-policies`, `/pr-budgets`; `/products`; `/health`; `/startup/checks` | v1 policy, per-product contract evidence, and diagnostics parity in compact collapsible v3 sections |
| Mandates | `GET/POST /mandates` | live SQLite intent, lifecycle, limits, and autonomy (§3.6) |
| Conductor ticks | `GET/POST /conductor/ticks` | live bounded, leased, PR-capacity-aware proposal planning (§3.7) |
| Blast radius | documented safety dependency edges | labels enforced edges as live and incomplete handoffs as planned; work-ledger-derived radius is not built (§3.8) |
| Run volume | durable materialized run index | live retained run totals and 24-hour window |
| Suite bootstrap | launcher + durable smoke/fleet jobs | start/stop first stack, ready/all launch, per-product lifecycle, logs, credentials, pairing, and all smoke tiers |
| Runbook history | runbook records + smoke/fleet evidence | live durable reads (B6) |
| Ask the hive | `POST /ask` | model call lives in Rust behind `PATCHHIVE_AI_URL` |
| Change log | `suite_events` + `product_action_events` | both durable sources shown separately from tab-local audit events |

## Deliberate current boundaries

- The conductor is proposal-only. It records discovery plans and concrete finding
  receipts but has no work-item dispatch or lifecycle-advance transition.
- PR headroom backpressures discovery. GitHub-rate, AI-spend, sandbox-slot, and
  per-owner-politeness admission gates are not implemented and are never displayed
  as observed capacity.
- Suite Runs supports ordered dispatch steps and explicit target fan-out. Declarative
  TOML pipelines and result-expression gates remain design work.
- The blast-radius drawer uses the documented safety-edge inventory. It does not infer
  a radius from the work ledger or pretend planned product handoffs are enforced.

## Rules that still bind

- HiveCore and the launcher never return GitHub tokens, provider keys, product service
  tokens, or launcher secrets to a read. Values the operator enters are write-only,
  held only in the active form, and cleared after a successful save.
- No direct browser-to-AI-provider calls. `PATCHHIVE_AI_URL` via the Rust backend.
- Responses use the contract-v1 envelope `{status, data, error, meta}`.
- `VITE_API_URL` is the only configurable value that reaches the client.
