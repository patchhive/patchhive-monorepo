# Product Operating Modes

PatchHive treats **who starts a run** and **how a target is selected** as two
independent decisions. Do not use `autonomous` as a synonym for `discovery`.

## Run triggers

- **Operator**: the operator clicks Run or dispatches the action directly.
- **Schedule**: a product-local or HiveCore schedule starts the action.
- **Webhook**: an authenticated external event starts the action.
- **Orchestration**: HiveCore or another authorized PatchHive product dispatches
  the action.

Operator-run and self-run behavior are both first-class. Automation does not
replace the Run button, and the Run button is not a temporary development path.

## Target selection

- **Direct**: the run receives a concrete repo, PR, diff, release, package,
  workflow, or local path.
- **Discovery**: the run receives a bounded scope such as topics, languages,
  organizations, repository policy, or saved scope and finds concrete targets.

This produces four valid combinations:

| Trigger | Target selection | Example |
| --- | --- | --- |
| Operator | Direct | Review this PR now |
| Operator | Discovery | Search these Rust topics now |
| Schedule/webhook/orchestration | Direct | Reassess this PR when it changes |
| Schedule/orchestration | Discovery | Hunt for suitable maintenance work nightly |

Every product should retain operator-run behavior while gaining appropriate
self-run triggers. Every maintenance product should retain direct targeting
while gaining bounded discovery where its data source and safety model allow
it. Products must advertise only combinations their current engine supports.

## Defaults

Products should prefer this behavior:

1. If a concrete target is present, use direct target selection.
2. If the target is blank and a discovery scope is present, use discovery target selection.
3. If neither target nor scope is present, show a clear empty-state prompt instead of inventing risky defaults.
4. Scan/read actions stay read-only by default.
5. Fix/write actions stay separate, explicit, and approval-aware.

## Scope Policy Controls

Discovery is only trustworthy when scope controls are visible and
enforced. Any product that discovers repos or work items should expose the
policy posture that shaped the run:

- allowlist count and active matches
- denylist/blocklist count and active matches
- opt-out count and active matches
- saved scope name or inline discovery scope
- repo caps, owner/org caps, and PR caps when relevant

Direct target selection still uses these controls. A direct target that matches
`opt_out` or `denylist` should be blocked with a clear explanation. A directed
target outside a configured allowlist should be blocked unless the operator
explicitly changes policy.

Run history should record the policy result, not just the target:

```json
{
  "trigger_mode": "operator",
  "target_selection_mode": "direct",
  "target_repo": "owner/repo",
  "policy_result": "allowed | blocked | opt_out",
  "policy_scope": "saved-scope-name or inline summary"
}
```

## UI Language

Recommended labels:

| Mode | UI language | Meaning |
| --- | --- | --- |
| Direct target | `Target repo`, `Target PR`, `Target release`, `Local path` | Analyze or act on the supplied concrete target |
| Discovery target | `Topic query`, `Language`, `Max repos`, `Discovery scope` | Discover targets inside a bounded scope |
| Operator trigger | `Run`, `Run now`, `Assess`, `Scan` | Start the work immediately |
| Self-run trigger | `Automation`, `Schedule`, `Webhook`, `Run with HiveCore` | Configure how PatchHive starts future work |

Avoid making discovery look like a failure to choose a target. Also avoid
making direct targeting or operator-run behavior feel less "PatchHive"; those
controls make self-running discovery testable and trustworthy.

## Run History

History and run APIs should expose both axes independently.

Minimum metadata:

```json
{
  "trigger_mode": "operator | schedule | webhook | orchestration",
  "target_selection_mode": "direct | discovery",
  "target_repo": "owner/repo",
  "discovery_scope": "topic/language/org/schedule summary"
}
```

UI expectations:

- Do not infer target selection from the trigger. A scheduled direct reassessment
  is automated but not discovery; an operator-triggered hunt is discovery but
  not self-started.
- If a single mixed list is used, show trigger and target-selection chips when
  the distinction matters.
- Keep selected-run detail panels style-neutral unless the style changes the meaning of the evidence.

This is the suite-wide default as automation and discovery are added.

## Product Expectations

| Product | Direct target selection | Discovery target selection |
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
- **Target repo blank**: run GitHub repo discovery using topic, language, stars,
  labels, and repo caps.

Either selection mode may be started by the operator or by an enabled schedule
or HiveCore orchestration.

Both modes must honor allowlist, denylist, and opt-out controls before patching or opening pull requests. Dry Stalk should support the same target rules but remain no-write.

## HiveCore Direction

HiveCore should expose both dimensions at suite level:

- Run the suite against this repo, PR, release, or local product.
- Run the suite across this topic, org, saved scope, schedule, or product registry.

HiveCore owns cross-product automation, but standalone products keep their Run
controls and product-local triggers. That lets PatchHive run itself without
taking away the operator's ability to start, test, compare, and focus work.
