# Shared Product Scheduling

PatchHive products support two independent choices:

- **trigger** — operator, schedule, webhook, or HiveCore orchestration;
- **target selection** — direct target or product-owned discovery.

A schedule is therefore not synonymous with discovery. A product may schedule a
known repository or PR, schedule a bounded discovery scope, or expose only one
of those target modes until its engine supports both.

The operator-facing names are **Target repo** and **Autonomous discovery**.
Every schedule record stores the corresponding backend mode explicitly as
`direct` or `discovery`; products must not infer autonomous behavior from an
empty repository field. Target-repo schedules may accept a product-specific
target shape. RefactorScout additionally accepts an allowed local path in that
mode.

In specialist v3 frontends, scheduling belongs in a **Controls** tab alongside
presets, target/scope configuration, repository policy controls, and suite
service integration when those capabilities exist. Products should not add a
standalone Schedules tab.

## Shared Substrate

`patchhive_product_core::scheduling` owns the repeated backend mechanics:

- `SaveProductScheduleRequest<T>` with a product-owned typed action payload;
- `ProductSchedule` persistence in `patchhive_product_schedules`;
- explicit `target_selection_mode` persistence with legacy schedules migrated
  to `direct`;
- stable conversion to `SuiteScheduleRecord`;
- schedule-name and cadence bounds;
- transactional due-work claims so two pollers do not claim the same run;
- next-run advancement before dispatch;
- last-run ID, status, timestamp, and error recording;
- preservation of run evidence when schedule configuration changes.

The common product API is:

```text
GET    /schedules
POST   /schedules
DELETE /schedules/:name
POST   /schedules/:name/run
```

`@patchhivehq/ui-v3` provides `ProductScheduleManager` for the corresponding
operator surface: save current inputs, load them, pause or enable the schedule,
run now, inspect the next/last run, and delete it.

## Product Ownership

The shared layer does not execute arbitrary product work. Each product owns:

- the typed action payload;
- direct/discovery capability declarations;
- scope and credential validation;
- repository opt-out, allowlist, denylist, and trusted-repo checks;
- the background executor;
- concurrency and output caps;
- whether an action is read-only or write-capable;
- any human approval, PR-budget, test, or publishing gate.

Scheduling must never expand the authority of the underlying action. A local
RefactorScout schedule still needs an allowed filesystem root. A future
RepoReaper schedule may discover work automatically, but opening a pull request
must still pass trust, validation, opt-out, and budget policy.

## Run Evidence

Scheduled results enter the normal product run/history store with:

- `trigger_type: "schedule"`;
- the schedule name and stable schedule ID;
- the same product run ID shape used by operator runs;
- the original target-selection payload;
- normal findings, artifacts, warnings, and lifecycle status.

This keeps scheduled work inspectable without a second history system.

## HiveCore Boundary

Products remain independently runnable and keep their schedules in their own
product database through the common schema. HiveCore should later index the
suite-facing records and dispatch product-owned actions through APIs. It must
not reach into product databases.

HiveCore eventually owns suite-wide concerns such as:

- fleet-wide enable/pause controls;
- overlapping-run policy and global concurrency;
- global repository policy and opt-out visibility;
- trusted-repository state;
- per-product and suite-wide write budgets;
- cross-product handoffs and recurring-run health.

## Rollout Status

- **SignalHive** — migrated to the shared store while retaining its existing
  direct/discovery schedule executor and legacy-record migration.
- **RefactorScout** — target-repo schedules accept allowed local paths or public
  GitHub repositories. Autonomous-discovery schedules save a bounded GitHub
  query/topic/language scope, select one eligible repository per run, and avoid
  repositories selected by the same schedule during its configured cooldown.
- **Other specialist products** — should adopt this substrate as their
  operator and autonomous execution modes are finalized. Do not build another
  private schedule table or one-off schedule UI.
- **HiveCore** — suite-level indexing and fleet controls remain a later
  integration step.
