# Product Operating Modes

PatchHive products should support two operator postures when the product domain allows it:

- **Directed mode**: the operator supplies a specific repo, PR, diff, release, package, or local path.
- **Autonomous mode**: the operator supplies a broad scope such as topic, language, org, schedule, or policy, and the product discovers the concrete targets.

Neither mode replaces the other. Directed mode keeps testing, known work, and trusted-repo workflows fast. Autonomous mode preserves PatchHive's core identity: the suite should find maintenance work instead of waiting for every target to be hand-picked.

## Defaults

Products should prefer this behavior:

1. If a concrete target is present, run directed mode.
2. If the target is blank and a discovery scope is present, run autonomous mode.
3. If neither target nor scope is present, show a clear empty-state prompt instead of inventing risky defaults.
4. Scan/read actions stay read-only by default.
5. Fix/write actions stay separate, explicit, and approval-aware.

## UI Language

Recommended labels:

| Mode | UI language | Meaning |
| --- | --- | --- |
| Directed | `Target repo`, `Target PR`, `Target release`, `Local path` | Analyze or act on the supplied concrete target |
| Autonomous | `Topic query`, `Language`, `Max repos`, `Discovery scope` | Discover targets inside a bounded scope |
| Hybrid | `Blank target = autonomous hunt` | A product can expose one form where a blank target falls back to discovery |

Avoid making autonomous discovery look like a failure to choose a target. Also avoid making directed mode feel less "PatchHive"; it is the control surface that makes autonomous behavior testable and trustworthy.

## Run History

When a product supports both directed and autonomous runs, its history UI and run APIs should expose the run style.

Minimum metadata:

```json
{
  "run_style": "directed | targeted | autonomous",
  "target_repo": "owner/repo",
  "discovery_scope": "topic/language/org/schedule summary"
}
```

UI expectations:

- Do not label all saved work as "autonomous runs" once directed runs exist.
- Split history into clear sections when both styles are common, such as `Targeted runs` and `Autonomous runs`.
- If a single mixed list is more compact, include a visible style chip on every row.
- Keep selected-run detail panels style-neutral unless the style changes the meaning of the evidence.

This should become the default pattern for every product as autonomous/batch discovery is added.

## Product Expectations

| Product | Directed mode | Autonomous mode |
| --- | --- | --- |
| SignalHive | Scan a supplied repo or saved scope | Discover maintenance-pressure repos from topics, languages, allowlists, or schedules |
| ReviewBee | Analyze a supplied PR | Later: find PRs with unresolved review pressure across repos |
| TrustGate | Review a supplied diff or PR | Later: inspect generated diffs from suite runs before write actions proceed |
| RepoMemory | Ingest a supplied repo | Later: refresh memory for scheduled or suite-selected repos |
| MergeKeeper | Assess a supplied PR | Later: watch PR queues and surface blocked merge lanes |
| FlakeSting | Scan a supplied repo/workflow | Later: watch Actions history across suite scopes |
| DepTriage | Scan dependency PRs/alerts for a supplied repo | Later: discover repos with urgent dependency pressure |
| VulnTriage | Scan security feeds for a supplied repo | Later: public advisory fallback and owner-scoped security sweeps |
| RefactorScout | Scan a local path or GitHub repo | Later: batch scout across trusted repos and create fix candidates |
| ReleaseSentry | Check a supplied release target | Later: monitor release candidates across products |
| RepoReaper | Hunt issues inside a supplied repo | Discover candidate bug issues across GitHub, then patch and open PRs |
| HiveCore | Run a suite against a selected target or stack | Own broad suite runs, schedules, and cross-product dispatch |

## RepoReaper Rule

RepoReaper should support both modes:

- **Target repo filled**: hunt open issues only inside `owner/repo`.
- **Target repo blank**: run the autonomous GitHub repo discovery flow using topic, language, stars, labels, and repo caps.

Both modes must honor allowlist, denylist, and opt-out controls before patching or opening pull requests. Dry Stalk should support the same target rules but remain no-write.

## HiveCore Direction

HiveCore should eventually expose the same distinction at suite level:

- Run the suite against this repo, PR, release, or local product.
- Run the suite across this topic, org, saved scope, schedule, or product registry.

That lets PatchHive stay radically autonomous without taking away the operator's ability to test, compare, and focus the system on known work.
