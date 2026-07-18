# PatchHive Product Naming Strategy

PatchHive should use apiary language to create a coherent product world without
making customers decode obscure biology before they understand what a product
does.

## Core Decision

Use **descriptive compounds for customer-facing products** and **real apiary
language inside the products**.

External names should communicate the job quickly. Internal surfaces can reward
deeper exploration with terms drawn from bees, hives, and coordinated colony
behavior. The biological vocabulary gives PatchHive depth; it should not become
a discoverability tax.

This means:

- do not force `Bee` or `Hive` into every product name;
- do prefer a hive-adjacent compound when it remains clear and credible;
- do not use an obscure biological word as a standalone product name unless the
  function remains obvious without explanation;
- keep product subtitles plain and descriptive even when the name is playful;
- evaluate trademark, package, repository, and domain collisions before
  approving a new external name.

## Protected Names

These names already carry a strong identity or fit the suite well and should not
be changed merely to increase the number of bee references:

| Product | Decision | Reason |
| --- | --- | --- |
| RepoReaper | Keep | PatchHive's strongest established specialist identity. |
| SignalHive | Keep | Clear function and native suite branding. |
| ReviewBee | Keep | Clear, approachable, and naturally bee-oriented. |
| MergeKeeper | Keep | Strong descriptive control-plane name. |
| FlakeSting | Keep | Distinctive and naturally tied to CI pain. |
| RefactorScout | Keep for now | Clearer and more credible than forcing `ScoutBee`. |
| HiveCore | Keep | Correctly names the suite's central orchestration layer. |

## Strong Rename Candidates

These are naming candidates, not implemented renames. A rename becomes approved
only after its migration inventory and final external name are confirmed.

| Current | Candidate | Status | Notes |
| --- | --- | --- | --- |
| VulnTriage | **VulnSting** | Strong candidate | Clear security meaning, memorable, and naturally fits PatchHive. |
| DepTriage | **UpdateHive** | Strong candidate | More approachable and better suited to a dependency-update workflow that may grow beyond triage. |
| ReleaseSentry | **ReleaseKeeper** or **ShipKeeper** | Open | Both are clear; avoid accidental confusion with MergeKeeper before deciding. |
| TrustGate | **GuardBee** or **HiveGuard** | Open; current name preferred | `GuardBee` is branded but may sound less serious. `HiveGuard` is clearer but more generic. |
| RepoMemory | **MemoryComb** | Open; current name preferred | The comb metaphor is stronger as an internal storage concept than as the external product name. |

## Internal Apiary Vocabulary

Internal names can be more specialized because the surrounding product already
provides functional context.

| Product area | Internal term | Intended meaning |
| --- | --- | --- |
| RepoMemory structured memory store | **Comb** | Durable cells of repository knowledge. |
| FailGuard lesson correlation and sharing | **Waggle** | A reviewed failure lesson communicated to other products. |
| Dependency intake queue | **Pollen** | Incoming dependency work gathered for ranking. |
| Coordinated agent execution | **Swarm** | Multiple product-owned agents acting through one constrained plan. |
| HiveMail inbox triage assistant | **InboxBee** | Focused assistant for sorting, drafting, and dispatching mail. |

Use these terms as tab labels, subsystem names, empty-state language, or
architecture concepts only when the UI also supplies a plain-language
description. For example, `Comb` should appear with copy such as “Durable repo
memory,” not by itself on a first-run screen.

### Terms to Avoid

**Propolis** is biologically accurate for a protective layer, but it is obscure,
requires explanation, and risks collision with existing developer/security
products. Do not use it as a product name. Avoid it internally unless a future
use is both clearly labeled and collision-free.

## Rename Safety

A product rename is a platform migration, not a text replacement. Before
implementation, inventory and plan changes to:

- display name, subtitle, icon metadata, and frontend storage keys;
- product slug and unified-backend registry entry;
- API prefixes and capability links;
- environment-variable prefixes and `.env` examples;
- SQLite filenames and migration/compatibility behavior;
- Docker services, images, ports, packages, and exported repositories;
- GitHub application/token documentation and maintained comment signatures;
- internal cross-product references, handoffs, tests, and canonical docs.

Prefer backward-compatible route, environment, and database aliases during a
rename. Do not ship a display-only rename that makes the UI disagree with the
API, filesystem, documentation, or standalone repository unless it is an
explicitly temporary compatibility phase.

## Naming New Products

For a new product, evaluate names in this order:

1. Can a maintainer understand the function from the name and subtitle?
2. Does the name fit PatchHive's specialist vocabulary without sounding
   childish or forced?
3. Is a real hive/bee/sting/keeper/scout metaphor naturally relevant?
4. Is the slug available across GitHub, packages, images, and routes?
5. Will the name remain accurate if the product grows from read-only evidence
   into an approved action workflow?

The goal is a suite that feels deliberately related, not a collection of puns.

