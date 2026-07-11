# GitHub Token Scopes

PatchHive products do not all need the same GitHub access. Use the narrowest
fine-grained personal access token or GitHub App installation token that supports
the product being tested.

GitHub tokens are separate from PatchHive operator API keys and product service
tokens. Operator API keys log into a local product. Product service tokens let
HiveCore call product-owned APIs. GitHub tokens let product backends read or
write GitHub data.

Most products read `BOT_GITHUB_TOKEN` first and may fall back to `GITHUB_TOKEN`
when the product supports that fallback. Keep raw tokens out of logs, screenshots,
and committed files.

## Token Types

Fine-grained tokens are preferred because they can be limited to specific
repositories and specific repository permissions.

For classic personal access tokens, use the smallest equivalent scope:

- Public read-only scans usually need no classic scope beyond public access, but
  `public_repo` raises the rate limit and allows public-repo write operations
  when a write-capable product explicitly needs them. Protected security feeds
  such as code scanning alerts and Dependabot alerts still need `public_repo`
  for public-only classic-token access.
- Private repository reads and writes usually require `repo`; security alert
  reads should prefer the narrower `security_events` scope when a classic token
  is unavoidable.
- Code scanning alert reads may require `security_events`.
- Workflow file edits require workflow write capability.

Classic tokens are broader and easier to overgrant, so treat them as fallback
credentials.

## Product Matrix

| Product | Base fine-grained permissions | Optional permissions | Why |
| --- | --- | --- | --- |
| SignalHive | Metadata read, Issues read, Contents read | None | Repository discovery, issue/backlog reads, and TODO/FIXME code-search evidence. |
| ReviewBee | Metadata read, Pull requests read | Issues write or Pull requests write | Reads PR metadata, reviews, review comments, and review-thread state. Write access is only for maintained PR checklist comments. |
| TrustGate | None for pasted diffs; Metadata read and Pull requests read for PR diff review | Issues write or Pull requests write; Commit statuses write or Checks write when GitHub reporting is enabled | Pasted diff review is local. GitHub mode reads PR diffs and may publish comments, statuses, or checks. |
| RepoMemory | Metadata read, Pull requests read, Issues read, Contents read | None | Reads merged PRs, review/comment/file context, closed issues, and lightweight file evidence for durable repo memory. |
| MergeKeeper | Metadata read, Pull requests read | Actions read; Issues write or Pull requests write; Commit statuses write or Checks write when publishing is enabled | Reads PR state, reviewer state, mergeability, review pressure, and optionally CI/check evidence. Write access is only for maintained output. |
| FlakeSting | Metadata read, Actions read | None | Reads workflow runs and workflow jobs to detect pass/fail swings and unstable steps. |
| DepTriage | Metadata read, Pull requests read | Dependabot alerts read | Reads dependency PRs. Dependabot alert access enriches security urgency, but the product still ranks dependency PRs when alert access is unavailable. |
| VulnTriage | Metadata read, Code scanning alerts read, Dependabot alerts read, with the target repository selected | None | Reads GitHub security alert feeds. These feeds can still return `403` if alerts are disabled, the repo was not selected for the token, or the token owner lacks security access. |
| RefactorScout | No GitHub token for local filesystem scans | Metadata read, Contents read for future GitHub-backed scans | Current MVP scans configured local paths. |
| ReleaseSentry | Metadata read, Contents read, Actions read, Pull requests read | Code scanning alerts read, Dependabot alerts read, Deployments read, Commit statuses write or Checks write if publishing becomes enabled | Reads release files, tags/check context, PR/release pressure, and optional security/dependency pressure. |
| RepoReaper | Metadata read, Contents read/write, Pull requests read/write, Issues read | Issues write for issue updates, Actions read for validation context, Workflows read/write only when editing `.github/workflows` | Clones or reads repos, creates branches/commits, opens PRs, and may reference issues. This token should belong to the PatchHive bot identity. |
| HiveCore | No GitHub token for current control-plane reads | Metadata read for future suite-level GitHub checks | HiveCore primarily stores product service tokens and calls product APIs. Product GitHub work should stay behind product-owned backends. |

## MergeKeeper Publish Test Token

For a narrow fine-grained PAT publish test, select only the fixture repository
and grant Metadata read, Pull requests read, Commit statuses read/write, and
Issues read/write. MergeKeeper first attempts a check run, then falls back to a
commit status; the fallback lets PAT-based testing succeed without requiring a
GitHub App. Full report delivery requires both the status signal and maintained
PR comment. A partial write remains visible as `report_partial` and does not
verify the publishing path.

## DepTriage Test Token

For the DepTriage rerun, use a fine-grained token selected for the target test
repositories with:

- Metadata: read
- Pull requests: read
- Dependabot alerts: read if security alert enrichment is expected

If Dependabot alerts are disabled on the target repo, or the token owner cannot
see that repo's security alerts, GitHub can return `403 Forbidden`. That should
be shown as unavailable security evidence, not as "no dependency risk exists."

## Common Failure Shapes

- `Resource not accessible by personal access token`: the token does not have
  the needed repository permission, the repo was not selected for the token, or
  the token owner does not have access to that protected data.
- `Code scanning alerts could not be read`: select the target repository and
  grant `Code scanning alerts` read on a fine-grained token, or
  `security_events` on a classic token. The token owner must also be able to see
  that repository's security alerts. For public-only classic-token scans,
  `public_repo` can be used instead.
- `Dependabot alerts could not be read`: select the target repository and grant
  `Dependabot alerts` read on a fine-grained token, or `security_events` on a
  classic token. The token owner must also be able to see that repository's
  security alerts. For public-only classic-token scans, `public_repo` can be
  used instead.
- `Dependabot alerts are disabled for this repository`: the repo does not expose
  Dependabot alert data to this token.
- Empty results with no warning: the product could read the feed and found no
  matching data in the requested window.

## Official Permission References

- GitHub Actions workflow runs and jobs use the `Actions` repository permission.
- Repository issues use the `Issues` repository permission.
- Repository contents and file reads use the `Contents` repository permission.
- Pull request metadata, reviews, and review comments use the `Pull requests`
  repository permission.
- PR checklist comments use the issue-comment API, which accepts `Issues` write
  or `Pull requests` write.
- Code scanning alerts use the `Code scanning alerts` repository permission.
- Dependabot alerts use the `Dependabot alerts` repository permission.
