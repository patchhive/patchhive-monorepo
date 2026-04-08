# 🔱 RepoReaper by PatchHive

> Resolve selected repository issues automatically and open validated pull requests.

RepoReaper is a multi-agent GitHub bug-fixing system. It hunts open issues, scores them for fixability, generates patches, has them reviewed and refined, runs tests, and opens validated PRs — all autonomously.

---

## Agent Roles

| Role | Icon | Job |
|------|------|-----|
| Scout | ◎ | Hunts repos and scores issues for fixability |
| Judge | ⚖ | Targets the most relevant files for the kill |
| Reaper | ⚔ | Forges the killing patch |
| Smith | ⬢ | Refines and improves patches before they ship |
| Gatekeeper | 🔒 | Runs tests and opens the PR |

---

## Features

- **Multi-provider AI** — Anthropic, OpenAI, Gemini, Groq, Ollama
- **Confidence scoring** — Reaper sets confidence per patch, Smith can reject low-confidence work
- **Rejected patches log** — Every Smith rejection is recorded with feedback
- **Self-healing patches** — Auto-retries on apply failure
- **Configurable test retries** — Set how many times to retry on test failure
- **Watch Mode** — Webhook-triggered auto-hunt on new bug issues
- **Dry Stalk** — Preview targets without making any changes
- **Presets** — Save and reload team configurations
- **Cost tracking** — Per-run and lifetime cost across all providers
- **PR Monitor** — Track all opened PRs and auto-cleanup merged branches

---

## Quick Start

```bash
cp .env.example .env
# Fill in BOT_GITHUB_TOKEN, BOT_GITHUB_USER, and either PROVIDER_API_KEY or PATCHHIVE_AI_URL

# Dev
cd backend && cargo run
cd ../frontend && npm install && npm run dev

# Docker
docker-compose up --build
```

Backend: `http://localhost:8000`
Frontend: `http://localhost:5173`

---

## Stack

- **Backend** — Rust, axum, rusqlite, reqwest, tokio
- **Frontend** — React, Vite
- **AI** — Direct HTTP to all providers (no SDK dependencies)

## Standalone Repo Notes

- The frontend installs `@patchhivehq/ui` from the public npm registry.
- The standalone GitHub Actions workflow checks `cargo check --locked` for the backend and `npm run build` for the frontend.
- The PatchHive monorepo remains the source of truth, but this repository is intended to be usable on its own.

## Local AI Gateway

RepoReaper supports `PATCHHIVE_AI_URL` for OpenAI-compatible local gateways.

```bash
# Start patchhive-ai-local in its own repo or from the PatchHive monorepo.
# Then point RepoReaper at that local gateway:

export PATCHHIVE_AI_URL=http://127.0.0.1:8787/v1
cd backend
cargo run
```

If `PATCHHIVE_AI_URL` is set, RepoReaper uses it for the `openai` provider. `OPENAI_BASE_URL` still works as a compatibility fallback.

---

*RepoReaper by PatchHive — part of the PatchHive maintenance platform*
