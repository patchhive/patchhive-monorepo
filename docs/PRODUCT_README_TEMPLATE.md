# Product README Template Standard

This document defines the **canonical structure for every product `README.md`** in the
PatchHive monorepo (`products/<slug>/README.md`).

## Purpose

Product READMEs are the **getting-started entry point**. They are short, consistent, and
opinionated about what lives here versus in the full product doc:

- **README** → what the product is, how to run it locally, the config you'll actually touch,
  and its safety posture. Target length: **90–150 lines**.
- **Full doc** (`docs/products/<slug>.md`) → API reference, technical architecture, complete
  configuration reference, monitoring, deployment, troubleshooting, related products, status.

The split keeps the README scannable while the detailed doc stays the single source of truth
for depth. READMEs must **never duplicate** the detailed doc's reference content; they point to
it.

## Canonical Section Order

Every product README MUST use this section order. Optional product-specific context sections
may appear only in the slot marked below.

```
# <ProductName> by PatchHive
## Documentation
## Core Workflow
## <optional product-specific context section(s), e.g. Operating Model / Product Boundary / Port Reference / Cross-Product Use / FailGuard Lessons — placed together immediately after Core Workflow>
## Quick Start
### Docker
### Split Backend and Frontend
## Configuration
## Safety Boundary
## HiveCore Fit
## Standalone Repository
```

### Section rules

| Section | Required | Content |
| --- | --- | --- |
| Title | ✅ | `# <Name> by PatchHive` |
| `## Documentation` | ✅ | Link block + "Where to find what" doc-map table (see below) |
| `## Core Workflow` | ✅ | 3–7 bullet steps of the primary loop |
| Product-specific context | ⬜ | One optional section after Core Workflow (Operating Model, Product Boundary, Port Reference, etc.) |
| `## Quick Start` | ✅ | `### Docker` and `### Split Backend and Frontend` with copy-paste commands + default ports |
| `## Configuration` | ✅ | Essential config table (the variables an operator must set or is likely to change) + suite auth/bootstrap notes |
| `## Safety Boundary` | ✅ | Concrete guarantees about what the product will NOT do |
| `## HiveCore Fit` | ✅ | How HiveCore surfaces or dispatches this product |
| `## Standalone Repository` | ✅ | Monorepo source-of-truth note + exported mirror link |

## Documentation link block

Every README opens its `## Documentation` section with:

```markdown
## Documentation

- Full product doc: [docs/products/<slug>.md](../../docs/products/<slug>.md)
- Product docs index: [docs/products/README.md](../../docs/products/README.md)

> This README is the getting-started entry point. The full product doc carries the API
> reference, technical architecture, complete configuration reference, monitoring, deployment,
> and troubleshooting.
```

## "Where to find what" doc-map

Immediately after the link block, include a table mapping reader intent to the detailed doc's
section anchors. **Only list rows for section headings that actually exist** in
`docs/products/<slug>.md` (verify by checking the document's `## ` headings). Use the exact
heading text lowercased with spaces replaced by hyphens as the anchor (e.g. `## API Endpoints`
→ `#api-endpoints`).

```markdown
### Where to find what

| If you need… | See in the full doc |
| --- | --- |
| API endpoints and request/response shapes | `#api-endpoints` |
| Service layout and dependencies | `#technical-architecture` |
| Every configuration variable | `#configuration` |
| Health checks and metrics | `#monitoring` |
| Production deployment steps | `#deployment` |
| Symptom → cause → fix | `#troubleshooting` |
| How it relates to other products | `#related-products` |
| What is / isn't built yet | `#current-status` |
```

Drop any row whose target section does not exist in the detailed doc.

## Authoring rules

1. **No fabrication.** Every config variable, port, endpoint, and claim in a README must come
   from the product's existing `README.md` or its `docs/products/<slug>.md`. Do not invent.
2. **Concise.** Keep the README between 90 and 150 lines. Move depth to the detailed doc.
3. **Preserve accurate facts.** If the current README states a workflow step, config var, or
   safety guarantee, it must survive the rewrite.
4. **Consistent links.** Use the relative paths `../../docs/products/<slug>.md` and
   `../../docs/products/README.md`.
5. **English only.** Product READMEs are written in English.
6. **One source of truth.** The detailed doc owns reference content. The README points, it does
   not copy.

## Relationship to the detailed-doc template

The detailed product doc follows its own 14-section standard
(Product Role, Core Workflow, Inputs, Outputs, Safety Boundary, Local Development,
Configuration, Technical Architecture, API Endpoints, Monitoring, Deployment, Troubleshooting,
Related Products, Current Status). This README template is the lightweight companion: it shares
`Core Workflow`, `Configuration`, `Safety Boundary`, and `HiveCore Fit` intent but stays
getting-started scoped and adds the doc-map that connects the two.
