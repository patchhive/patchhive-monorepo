# HiveCore Repository Safety and PR Budgets

Status: **future design; not implemented**

This document records three suite-wide controls that should exist before
PatchHive expands autonomous outbound work:

1. a public repository-owner opt-out;
2. an operator-managed trusted-repository setting; and
3. hierarchical pull-request budgets owned by HiveCore.

These are control-plane policies, not product preferences. Products may keep
temporary local controls during migration, but the final authority should be
HiveCore and the shared backend so every product reaches the same decision.

## 1. Public repository opt-out

Repository owners should be able to opt out of all PatchHive automation from
`patchhive.dev`.

### Minimal surface

The first version should remain intentionally small:

- one `repository_opt_outs` table;
- two API endpoints; and
- one form page on `patchhive.dev`.

Suggested table:

```text
repository_opt_outs
  repository          TEXT PRIMARY KEY  -- normalized owner/repo
  opted_out            INTEGER NOT NULL
  github_actor         TEXT NOT NULL
  verified_permission  TEXT NOT NULL
  reason               TEXT NOT NULL DEFAULT ''
  created_at           TEXT NOT NULL
  updated_at           TEXT NOT NULL
```

Suggested endpoints:

```text
GET  /api/repository-opt-outs/:owner/:repo
POST /api/repository-opt-outs
```

The `GET` endpoint returns the current decision for product checks. The `POST`
endpoint is an authenticated upsert and accepts both opt-out and restoration,
so removal does not require a third endpoint.

The form page should:

1. authenticate the requester through GitHub;
2. verify that the requester has repository administration or maintain-level
   authority;
3. show the exact normalized repository being changed;
4. submit the opt-out or restoration; and
5. show a durable confirmation with the effective time.

An arbitrary GitHub user must not be able to opt out somebody else's
repository. Organization-owned repositories must use the requester's actual
repository permission, not organization membership alone.

### Enforcement

`opt_out` is the strongest repository policy and cannot be overridden by a
trusted-repository setting, allowlist, schedule, approval, or remaining PR
budget.

Every product must check the canonical decision before it performs meaningful
repository automation, including:

- detailed issue or pull-request analysis;
- repository-content or security scanning;
- cloning or fetching;
- running tests;
- creating branches, commits, comments, issues, statuses, or pull requests;
- scheduling follow-up work; and
- dispatching another product against the repository.

Products may encounter a repository name during broad discovery, but once the
repository is known to be opted out they must stop before fetching or storing
detailed repository evidence.

Mutating actions must fail closed when the canonical opt-out decision cannot be
verified. Read-only discovery should skip the repository or leave a visible
`policy unavailable` result rather than treating an unavailable policy service
as permission.

Short-lived caching is acceptable for scale, but product runs must record the
policy decision, source, and evaluation time. Opt-out restoration should take
effect through the same API and audit path.

## 2. Suite-wide trusted repositories

HiveCore should own an operator-managed set of trusted repositories. Trust is
an explicit elevation for repos where the operator permits operations that are
blocked for unknown or untrusted repositories, such as:

- executing repository-provided test commands;
- using a broader sandbox profile;
- allowing longer test timeouts or larger resource budgets;
- enabling selected write-capable product actions; and
- permitting product-specific higher-risk operations that advertise a trust
  requirement.

Trust must never bypass:

- public repository opt-out;
- denylist policy;
- GitHub permission checks;
- TrustGate or validation failures;
- approval requirements;
- product or suite PR budgets; or
- product-specific safety boundaries.

The initial setting may be a repository-level boolean. The stored record should
leave room for future capability-specific grants so PatchHive can eventually
distinguish `tests allowed` from broader write authority without replacing the
policy model.

Every higher-risk action should ask HiveCore for a typed policy decision such
as:

```json
{
  "repository": "owner/repo",
  "operation": "execute_repository_tests",
  "decision": "allowed",
  "trusted": true,
  "opted_out": false,
  "policy_version": "...",
  "evaluated_at": "..."
}
```

The product must save that decision with its run evidence. A trusted repo is a
permission to attempt the operation under its normal sandbox and validation
rules, not evidence that the repository code is safe.

## 3. Hierarchical PR budgets

HiveCore should own two budget layers:

- **Per-product maximum:** configurable for each write-capable product, such as
  RepoReaper `5` or MergeKeeper `3`.
- **Suite-wide ceiling:** one number for all PatchHive products, such as `10`.

A product may reserve a PR slot only when both layers have capacity. Effective
capacity is always:

```text
min(product remaining, suite remaining)
```

If a product has five slots remaining but the suite has two, that product can
reserve at most two. A product maximum can never expand the suite ceiling.

### Atomic reservation, not check-then-write

The eventual API should not let products independently read a count and then
decrement it. Concurrent products could all observe the same remaining slot.
HiveCore must perform the check and reservation atomically in one database
transaction.

Recommended lifecycle:

1. Product submits a reservation request with product, repo, run, and intended
   action.
2. HiveCore checks opt-out and product eligibility.
3. HiveCore atomically checks both budget layers and reserves one slot.
4. HiveCore returns a short-lived reservation ID.
5. Product opens the PR and commits the reservation with the GitHub PR URL.
6. If PR creation fails, the product releases the reservation.
7. Expired uncommitted reservations are reclaimed automatically.
8. Reconciliation compares committed reservations with GitHub so drift remains
   visible and repairable.

Products must fail closed on PR creation when HiveCore cannot grant a
reservation. A UI-side counter is informative only; the backend reservation is
the authority.

### Counting window to decide before implementation

The word `maximum` needs an explicit replenishment rule. The recommended first
model is **concurrent open PatchHive PR slots**:

- a committed slot stays consumed while the PatchHive-authored PR is open;
- merge or close releases the slot; and
- HiveCore periodically reconciles GitHub state.

This directly supports the requirement that PatchHive never has more than the
suite ceiling of active outbound PRs. Rolling daily or weekly creation limits
can be added later as a separate anti-spam guardrail; they should not be hidden
inside the concurrent-slot counter.

### Minimum budget state

HiveCore will need durable records for:

- the single suite ceiling;
- each product maximum;
- active and expired reservations;
- committed PR URLs and current state;
- release/reconciliation reason; and
- an audit event for every grant, denial, commit, release, expiry, and repair.

Budget denials should be normal product evidence, not generic server errors.
The response should say whether the product limit or suite ceiling won and show
the next safe action.

## Policy order

For any write-capable repository action, the intended evaluation order is:

1. public repository opt-out;
2. operator denylist;
3. allowlist or directed-scope eligibility;
4. operation-specific trust requirement;
5. product safety and approval requirements;
6. per-product PR capacity;
7. suite-wide PR capacity; and
8. atomic reservation immediately before PR creation.

Earlier denials cannot be overridden by later grants. In particular, trust and
budget capacity never override opt-out.

## Ownership

- `patchhive.dev` hosts the repository-owner opt-out form and canonical public
  decision API.
- HiveCore presents and manages suite trust settings and PR budgets.
- The shared backend owns policy evaluation, atomic budget state, and audit
  records.
- Products declare the operation they want, request a decision or reservation,
  obey it, and store the returned evidence with the run.

## Implementation boundary

Nothing in this document is currently active merely because it is documented.
Before rollout, the API contract, authentication model, GitHub ownership check,
database migrations, caching behavior, failure policy, and product integration
tests must be implemented and verified across every product capable of acting
on repositories.
