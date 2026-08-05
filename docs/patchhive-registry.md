# PatchHive Registry

The PatchHive Registry is PatchHive's public, opt-in evidence service. It lets
HiveCore publish a safe view of a running Tendwright installation, exposes
GitHub-verifiable public contribution outcomes, and owns the verified
repository opt-out lifecycle consumed by Tendwright installations.

The core idea:

```text
Products -> HiveCore -> PatchHive Registry -> patchhive.dev
GitHub reconciliation ------------^
Repository-owner controls --------^
```

Products should not each phone home independently. HiveCore is the suite brain,
so HiveCore should collect local state, sanitize it, and publish only the
allowed registry snapshot. The Registry is not the brain. It is the public
evidence and policy relay. A local Tendwright installation remains fully useful
when the Registry is unavailable or disabled.

## Why This Fits PatchHive

Tendwright is a suite, not a pile of separate tools. HiveCore already knows which
products are running, which products are paired, which smoke checks passed, what
capabilities are exposed, and which products are ready for orchestration.

A registry turns that private control-plane state into a public ecosystem
signal when the operator opts in.

Useful outcomes:

- The website can show live PatchHive fleet status instead of static marketing.
- Demo installs can advertise which products are online.
- Public health pages can show latest smoke evidence and release readiness.
- PatchHive can eventually show anonymous adoption and version telemetry.
- The suite can prove that HiveCore really is coordinating a living system.

This also supports the broader PatchHive identity. If PatchHive contributes
autonomously under its own GitHub account, a public registry can become part of
that reputation trail: not by exposing private customer data, but by showing
operational proof.

## Non-Goals

The registry should not become:

- A remote-control service for local products.
- A cloud dependency for local PatchHive use.
- A place where product secrets, tokens, logs, repo names, file paths, issue
  contents, PR contents, or local machine details are uploaded by default.
- A replacement for HiveCore.
- A hidden telemetry system.

Local PatchHive must continue to work with the registry completely disabled.

## Canonical Community And Contribution Model

The Registry may show PatchHive-operated activity and opted-in community
Tendwright installations. Its purpose is to make collective helpful
contribution visible, not to create competition. It must never ship
leaderboards, rankings, streaks, competitive scores, "top contributor" labels,
acceptance-rate comparisons, or volume-based rewards.

The public Registry has three evidence surfaces:

1. **PatchHive-operated contributions:** public PRs opened by PatchHive-owned
   identities and reconciled directly against GitHub.
2. **Community installation profiles:** outbound-only, sanitized snapshots and
   selected public contribution references submitted by operators who opted in.
3. **Collective impact:** evidence-backed totals across eligible public records,
   presented without comparing or ordering installations or people.

An installation profile may show its own factual history, enabled product
slugs, coarse readiness, versions, aggregate activity selected by the operator,
and last-report time. The Registry never probes a local installation, requires
an inbound port, receives a HiveCore control credential, or describes a
heartbeat as proof that an instance is currently reachable.

Visibility is operator-controlled:

- **disabled:** HiveCore sends nothing;
- **anonymous:** adoption and compatibility aggregates only;
- **named private:** visible only to the operator account;
- **unlisted:** available by stable direct link but absent from directories and
  community aggregates; and
- **public:** eligible for the public directory and collective impact totals.

The current service implements `anonymous`, `named-private`, and `public-demo`;
`public-demo` is the compatibility predecessor to the target `public` mode.
Unlisted profiles and operator-managed visibility changes are not implemented
yet. Moving to greater visibility always requires fresh consent. Reducing
visibility must remove the profile from affected public reads immediately.

Public fields carry provenance and freshness. At minimum, consumers distinguish
`github_verified`, `instance_reported`, `stale`, `verification_failed`,
`not_observed`, and `unknown`. These states must not collapse into zero counts,
a reassuring status, or a single `verified` boolean.

Contribution statistics are summaries over durable public PR references. The
outcome lifecycle distinguishes open, merged, closed without merge,
verification failed, not observed, and unknown evidence. Only GitHub-observed
records contribute to verified totals; instance-reported counts stay labeled
and separate.

A real public PR verifies a GitHub lifecycle, but not which installation created
it. Installation attribution additionally requires a Registry-issued receipt
bound before publication or a GitHub identity whose control the operator
verified. Product attribution requires structured publication evidence or PR
disclosure and is never guessed from prose, repository topic, or author alone.

Useful non-competitive summaries include:

- verified PRs opened, currently open, merged, and closed without merge;
- public repositories helped by at least one verified merged contribution;
- maintenance categories addressed;
- outcome totals over an explicit time window;
- median time to first public maintainer response or merge when observable; and
- observation timestamps with links to the underlying public PRs.

Unsuccessful outcomes remain visible and may feed reviewed FailGuard learning;
they are not hidden to improve a score.

### Current Implementation Boundary

The hosted-service MVP currently implements install registration,
per-install update credentials, authenticated heartbeat and smoke ingestion,
sanitized public-demo reads, GitHub-verified repository-owner opt-out
assertions/revocations, and the typed HiveCore opt-out feed.

It does not yet implement HiveCore publishing, contribution ingestion and
GitHub reconciliation, verified PR statistics, unlisted profiles, credential
rotation, visibility management, install unpublish/deletion, Maintenance Brief
publication, or live `patchhive.dev` consumption. The website must not imply
that any of those are live, and checked-in fixtures must be labeled as demo
data.

## Privacy Boundary

The registry should be opt-in and sanitized by default.

Never send:

- GitHub tokens, API keys, service tokens, AI provider keys, or suite bootstrap
  secrets.
- Raw logs.
- Local filesystem paths.
- Private repo names or URLs.
- Issue titles, PR titles, commit messages, or file names from private targets.
- Product `.env` values.
- Usernames or machine hostnames unless explicitly configured for a public demo.

Allowed by default after opt-in:

- Installation ID generated by HiveCore.
- HiveCore version.
- Product slugs and versions.
- Product status categories such as `online`, `degraded`, `offline`, or
  `blocked`.
- Capability IDs and contract versions.
- Smoke tier names, timestamps, and pass/warn/fail counts.
- Image mode and image tag summary.
- Whether launcher support is available.
- Whether the install is `private`, `anonymous`, or `public-demo`.

Richer data should require explicit per-install and per-field consent.

## Registry Modes

### Disabled

Default mode. HiveCore does not send anything to the registry.

### Anonymous

HiveCore sends aggregate installation health without any public identity.

Example:

- product count
- product versions
- smoke pass/warn/fail counts
- launcher available or unavailable
- image mode

This is useful for adoption and compatibility telemetry.

### Named Private

HiveCore sends a named install visible only to the operator account on the
PatchHive website.

Example:

- "Jeremy local lab"
- "PatchHive staging"
- "Demo laptop"

This gives the operator a remote status page without publishing it.

### Public Demo

HiveCore sends a public install profile that the website can show to visitors.

Example:

- "PatchHive public demo"
- live product cards
- latest smoke evidence
- release readiness
- public launch links, if configured

This mode should require an explicit confirmation because it is intended to be
public-facing.

## HiveCore Snapshot Shape

HiveCore should expose a local sanitized snapshot endpoint first.

Candidate endpoint:

```text
GET /registry/snapshot
```

Example response:

```json
{
  "schema_version": "registry.snapshot.v1",
  "install_mode": "anonymous",
  "install_id": "hc_01j...",
  "generated_at": "2026-06-15T18:00:00Z",
  "hivecore": {
    "version": "0.1.0",
    "status": "online",
    "launcher_available": true,
    "suite_bootstrap_enabled": true
  },
  "fleet": {
    "products_total": 12,
    "products_online": 12,
    "products_degraded": 0,
    "products_blocked": 0,
    "products_paired": 12
  },
  "products": [
    {
      "slug": "signal-hive",
      "version": "0.1.0",
      "status": "online",
      "capability_ids": ["scan", "smoke_check"],
      "contract_version": "product-api-contract-v1",
      "image_tag": "main"
    }
  ],
  "smoke": {
    "latest_tier": "read-only-fleet",
    "latest_status": "ready",
    "passed": 47,
    "warned": 16,
    "failed": 0,
    "skipped": 0
  }
}
```

The local snapshot endpoint is useful even before the hosted registry exists.
It gives the website, docs, and local demos one stable contract to consume.

## Hosted Registry API

Once the local snapshot is stable, HiveCore can publish outbound heartbeats to a
hosted registry.

Candidate endpoints:

```text
POST /v1/installs/register
POST /v1/installs/:install_id/heartbeat
POST /v1/installs/:install_id/smoke
POST /v1/installs/:install_id/briefs
POST /v1/installs/:install_id/briefs/:snapshot_id/unpublish
POST /v1/installs/:install_id/contributions
PATCH /v1/installs/:install_id/visibility
DELETE /v1/installs/:install_id
GET  /v1/public/installs
GET  /v1/public/installs/:public_slug
GET  /v1/public/contributions
GET  /v1/public/contributions/summary
GET  /v1/public/briefs/:owner/:repo
GET  /v1/public/briefs/:owner/:repo/versions/:snapshot_id
```

Registration should return a registry token that HiveCore stores locally. The
token should only allow that install to update its own registry record.

Heartbeats should be idempotent and rate-limited. The registry should tolerate
offline installs and show stale status clearly instead of pretending the fleet is
still live.

Contribution, visibility, deletion, and brief routes are target contracts, not
current MVP endpoints. Contribution submission accepts bounded public PR
references and source attribution, then queues independent GitHub verification;
it does not accept an installation's claimed merge state as verified truth.

The brief endpoints are future contract direction, not part of the current
hosted-service MVP. Publishing a brief should use the installation's scoped
registry token plus an explicit per-brief approval record. The latest brief
endpoint may resolve a current public pointer, but every stored version should
remain addressable by an immutable snapshot ID.

Snapshot immutability means published content is never edited in place; it does
not mean publication is irreversible. An authenticated unpublish action should
remove the snapshot from public indexes and latest pointers, return `410 Gone`
for its former public version route, and retain a tombstone plus audit record.
Registry administrators need a narrowly controlled hard-delete path for leaked
secrets, legal requirements, repository-owner opt-outs, and compromised
installations. An active repository opt-out blocks new publication and
unpublishes existing latest pointers without rewriting snapshot history.

## Maintenance Brief Hosting Architecture

Public Maintenance Briefs should use the same private-to-public boundary as
other Registry data:

```text
Specialist products
    -> HiveCore suite run and private Maintenance Brief
    -> operator preview, redaction, and publication approval
    -> PatchHive Registry versioned public snapshot
    -> patchhive.dev report, download, trend, showcase, and badge views
```

Responsibilities remain separate:

- **Specialist products** own analysis, product-specific evidence, findings,
  scores, warnings, and run history.
- **HiveCore** owns cross-product orchestration, private evidence links,
  coverage/freshness calculation, suggested-action state, and construction of
  the complete private brief.
- **The operator** previews the exact public payload, removes or suppresses
  fields when needed, and explicitly approves each published brief.
- **The Registry** authenticates publication, enforces a public schema and size
  limits, stores immutable/versioned sanitized snapshots, exposes public reads,
  and marks the latest snapshot stale when its freshness window expires.
- **`patchhive.dev`** renders the polished public experience. It does not receive
  local product credentials, read private product APIs, or reconstruct missing
  evidence in the browser.

The public schema is an allowlist, not a recursive copy of the private brief.
It must reject arbitrary HTML or executable content, private or loopback URLs,
local filesystem paths, credentials, private product API references, and
unrecognized embedded fields. Public links must use typed link kinds and
allowed HTTPS hosts. Product/run/finding identifiers may be retained for audit
correlation only when the operator explicitly marks them public; otherwise the
snapshot carries non-resolvable public references.

Candidate public website routes:

```text
/briefs/:owner/:repo
/briefs/:owner/:repo/versions/:snapshot_id
/briefs/:owner/:repo/download.html
/briefs/:owner/:repo/badge.svg
/analyze?repo=:owner/:repo
```

The latest brief route should show the assessed commit, publication time,
evidence freshness, coverage, missing feeds, and a link to immutable versions.
HTML downloads and print-to-PDF should render the same sanitized snapshot rather
than a separate report truth. A badge should link to the brief and degrade to
`stale` or `incomplete evidence` when appropriate; it should not preserve an old
positive status indefinitely.

`/analyze?repo=...` may initially prefill an intake form or explain how to run
PatchHive locally. It must not cause the public website or Registry to call a
private installation. If public visitors can eventually request a fresh scan,
execution belongs to a separate PatchHive-owned hosted runner or hosted HiveCore
deployment with:

- public-repository-only targeting and repository opt-out enforcement;
- strict anonymous and account-level rate limits, queues, and cost budgets;
- pinned-commit inputs and bounded, visible coverage;
- no GitHub write credentials or mutating product actions in the public lane;
- untrusted-repository and prompt-injection handling;
- an explicit rule for whether a completed result is private to the requester,
  operator-reviewed, or eligible for public publication.

The Registry must remain a snapshot store and public read service. It must not
become the job scheduler, AI executor, or remote-control path for local suites.

## Website Surfaces

The PatchHive website can use registry data to become a living status surface.

Possible website sections:

- PatchHive-operated contribution history with direct public PR evidence
- Collective verified impact across opted-in installations
- A community installation directory with no ordering by output or quality
- Individual installation profiles showing factual history without comparison
- Last-reported suite card: "12 products reported ready · 4 minutes ago"
- Product fleet map with current versions
- Latest smoke evidence
- Public demo launch links
- Release readiness status
- Operator-published Maintenance Briefs for public repositories, pinned to the
  assessed commit and carrying only explicitly sanitized specialist evidence
- Anonymous install/version adoption charts
- GHCR image freshness

The website must visually separate PatchHive-operated activity, community
activity, GitHub-verified evidence, and instance-reported evidence. Community
copy describes shared helpful impact and must not introduce rankings,
comparisons, streaks, scores, or volume rewards.

Maintenance Brief publication must be snapshot-based. HiveCore should assemble
the authenticated cross-product brief, show the operator the exact sanitized
payload, and publish an immutable or versioned snapshot only after explicit
approval. The public website may offer a shareable repository or brief URL, but
that URL must read saved Registry data rather than directly reaching a local
PatchHive product or triggering a new scan.

Every public brief should show:

- repository, assessed commit, scan time, and snapshot freshness;
- which specialists contributed and links or public references permitted by the
  operator;
- coverage bounds, unavailable feeds, and warnings;
- prioritized evidence and any later public PatchHive PR outcome;
- a clear distinction between observed maintenance pressure and certification.

A curated public-repository showcase is acceptable when PatchHive owns the scan
and publication decision. Arbitrary unauthenticated scans, private evidence, and
unbounded public AI execution do not belong in the Registry.

For marketing, the most important effect is emotional: PatchHive should look
like a live system with a pulse, not a static set of screenshots.

## Security Model

Minimum expectations:

- Registry publishing is disabled by default.
- Registry tokens are scoped per install.
- Heartbeats are signed or authenticated.
- HiveCore owns the allowlist of fields that can leave the machine.
- Operators can preview the exact JSON before enabling publishing.
- Maintenance Brief publication requires an explicit per-brief preview and
  approval even when the installation is already in public-demo mode.
- Public mode requires an explicit confirmation.
- The website never receives registry data directly from browsers on local
  networks.

If customer installs ever use the registry, add:

- data retention controls
- delete install endpoint
- audit log for registry updates
- organization ownership
- clear privacy terms

## Build Plan

### Phase 1: Local Snapshot

Add `GET /registry/snapshot` to HiveCore. It should derive a sanitized snapshot
from existing HiveCore state: product catalog, runtime status, launcher status,
capabilities, pairing, and latest smoke evidence.

No hosted service yet.

### Phase 2: Website Mock Integration

Point the local website or a dev-only page at a checked-in sample snapshot. This
lets the public design evolve before network publishing exists.

### Phase 3: Hosted Registry MVP

Create a small `patchhive-registry` service with:

- install registration
- heartbeat ingestion
- public demo reads
- simple SQLite or Postgres storage
- rate limiting
- per-install registry tokens

This can be a service under `services/` first. It only needs its own repo if it
becomes independently deployed.

Current non-HiveCore implementation:

- Static registry website exists in `patchhive-sites/apps/registry/` and reads a
  checked-in public-demo fixture.
- Hosted-service MVP exists in `services/patchhive-registry/` with install
  registration, authenticated heartbeat and smoke ingestion, SQLite storage,
  shared PatchHive rate limiting, public-demo read endpoints, GitHub-verified
  repository-owner opt-out assertions/revocations, and an authenticated typed
  lifecycle feed for HiveCore.
- HiveCore is not required for either piece yet; Phase 1 and Phase 4 remain the
  HiveCore-specific work.

### Phase 4: HiveCore Publisher

Add HiveCore settings for:

- registry disabled/anonymous/named/private/public-demo mode
- registry URL
- registry token
- preview snapshot
- publish heartbeat now
- heartbeat interval

HiveCore should publish through its backend, not the browser.

### Phase 5: Public Website

Add website components that read public registry records and show:

- live demo fleets
- latest suite smoke
- product versions
- release readiness
- explicitly published public-repository Maintenance Brief snapshots

## Relationship To MaintainerBot

MaintainerBot can use the registry later as proof of service health.

Example public or private evidence:

- maintained crate count
- maintenance SLA status
- latest crate smoke/check result
- number of proposed PRs
- number of blocked/gated actions

The registry should still avoid exposing repo details unless the maintainer has
explicitly opted into public reporting.

## Decision

The PatchHive Registry is a HiveCore-fed, opt-in public evidence network and
repository-owner policy authority, not separate phone-home behavior inside
every product. Its community purpose is collective helpful contribution; it
will not ship competitive mechanics.

The implementation sequence from the current hosted-service MVP is:

1. HiveCore generates and previews a sanitized local registry snapshot.
2. Add visibility changes, credential rotation, immediate unpublish, and
   deletion before inviting community installations.
3. Add contribution receipts, source attribution, and GitHub reconciliation.
4. Connect `patchhive.dev`, keeping fixtures visibly labeled as demo data until
   real Registry evidence is available.
