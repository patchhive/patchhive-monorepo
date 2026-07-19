# GitHub Credential Contract

PatchHive uses one suite-wide credential for GitHub reads and separate
product-owned credentials for GitHub writes. GitHub credentials are distinct
from PatchHive operator API keys and peer-service tokens.

## Canonical variables

| Variable | Owner | Use |
| --- | --- | --- |
| `PATCHHIVE_GITHUB_TOKEN_RO` | Suite | Repository, pull-request, issue, Actions, release, history, and security reads. |
| `MERGE_KEEPER_GITHUB_TOKEN_RW` | MergeKeeper | Explicitly requested status and maintained-comment publishing. |
| `REVIEW_BEE_GITHUB_TOKEN_RW` | ReviewBee | Explicitly requested maintained checklist comments. |
| `TRUST_GATE_GITHUB_TOKEN_RW` | TrustGate | Explicitly requested status and maintained-comment publishing. |
| `REPO_REAPER_GITHUB_TOKEN_RW` | RepoReaper | Discovery, forks, branches, commits, and pull requests. |

`BOT_GITHUB_TOKEN` and `GITHUB_TOKEN` remain temporary read-path compatibility
aliases. New configuration and publishing code must not use them. The shared
read credential is never a fallback for a write client.

## Recommended classic PATs

Use a classic PAT with:

- `public_repo` when PatchHive only works with public repositories;
- `repo` when private repositories are intentionally in scope;
- `security_events` in addition when code-scanning or Dependabot alert reads
  require it;
- `workflow` only on RepoReaper's dedicated credential when RepoReaper is
  explicitly allowed to change files under `.github/workflows`.

GitHub classic PAT scopes are not truly read-only. `public_repo` and `repo`
authorize more than reads. PatchHive enforces the read boundary in code by
constructing read clients only from `PATCHHIVE_GITHUB_TOKEN_RO` and refusing to
use it for publishing.

For least privilege beyond that code boundary, use a GitHub App. Native check
runs require GitHub App authentication; PAT-backed publishers use commit
statuses plus maintained PR comments.

## Product matrix

| Product | Shared read credential | Dedicated write credential |
| --- | --- | --- |
| SignalHive | Repository, issue, and code-search evidence | None |
| RepoMemory | Merged PR, review, file, and closed-issue history | None |
| FlakeSting | Actions workflow runs and jobs | None |
| DepTriage | Dependency PRs and optional Dependabot alerts | None |
| VulnTriage | Code-scanning and Dependabot alerts | None |
| RefactorScout | Public GitHub clone and repository metadata | None |
| ReleaseSentry | Repository, tag, release, PR, issue, status, and Actions evidence | None |
| MergeKeeper | PR, review, mergeability, and check evidence | `MERGE_KEEPER_GITHUB_TOKEN_RW` |
| ReviewBee | PR reviews, comments, and thread state | `REVIEW_BEE_GITHUB_TOKEN_RW` |
| TrustGate | PR metadata and diff reads | `TRUST_GATE_GITHUB_TOKEN_RW` |
| RepoReaper | None; its autonomous workflow is one write-capable boundary | `REPO_REAPER_GITHUB_TOKEN_RW` |
| HiveCore | No direct GitHub work in the current control plane | None |

## Permission behavior

- Token acceptance only proves that GitHub recognizes the identity.
- Every product verifies target-specific read access during the run.
- A configured write credential is not reported as verified until a
  target-specific write succeeds.
- Repository security settings and the token owner's access can still make
  protected alert APIs return `403`.
- Missing protected evidence remains a warning. It is never interpreted as zero
  risk.
- Public-repository write access still depends on the token owner having
  collaborator or equivalent repository permission.

## Common failures

- `Resource not accessible by personal access token`: the token owner lacks
  target access, a required classic scope is absent, or the endpoint requires a
  GitHub App.
- `You must authenticate via a GitHub App`: native check runs do not accept a
  PAT. PatchHive should use its commit-status fallback.
- Security-alert `403`: add `security_events`, confirm the repository exposes
  the feed, and confirm the token owner can view repository security alerts.
- Empty results with no warning: the product read the feed and found no matching
  evidence in the requested scope.

Keep raw tokens only in the ignored canonical root `.env`. Never put them in
logs, screenshots, committed files, or frontend storage.
