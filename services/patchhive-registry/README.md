# PatchHive Registry Service

`patchhive-registry` is the hosted-service MVP for the PatchHive Registry plan.
It accepts opt-in, sanitized PatchHive suite snapshots and exposes only public
`public-demo` records.

The target Registry also includes operator-controlled community instance
profiles and GitHub-verified public contribution evidence. Those capabilities
are not implemented by this MVP. The Registry is intentionally
non-competitive: it will not rank installations or contributors, award
volume-based scores, or present instance-reported counts as verified facts. See
[`docs/patchhive-registry.md`](../../docs/patchhive-registry.md) for the
canonical boundary and implementation plan.

This service does not require HiveCore yet. HiveCore can later publish the same
snapshot shape to these endpoints.

## Run

```bash
cd services/patchhive-registry
PATCHHIVE_REGISTRY_BIND_ADDR=127.0.0.1:8130 cargo run
```

The SQLite database defaults to `patchhive-registry.db`. Override it with:

```bash
PATCHHIVE_REGISTRY_DB_PATH=/tmp/patchhive-registry.db cargo run
```

Set `PATCHHIVE_REGISTRY_OPT_OUT_SYNC_KEY` to a machine-random secret before
exposing the repository-owner opt-out feed. HiveCore uses the same value as
`PATCHHIVE_OPT_OUT_SYNC_KEY`.

## API

- `GET /health`
- `POST /v1/installs/register`
- `POST /v1/installs/:install_id/heartbeat`
- `POST /v1/installs/:install_id/smoke`
- `GET /v1/public/installs`
- `GET /v1/public/installs/:public_slug`
- `POST /v1/repository-opt-outs` — assert an opt-out after GitHub administrator verification
- `DELETE /v1/repository-opt-outs/:owner/:repo` — revoke an assertion after the same verification
- `GET /v1/sync/repository-opt-outs` — authenticated full lifecycle feed for HiveCore

The assert and revoke routes require the repository administrator's GitHub
token as `Authorization: Bearer <github_token>`. The sync route requires
`X-PatchHive-Opt-Out-Sync-Key` instead. Active and revoked assertions are both
durable so an interrupted or stale consumer cannot invent the current state.

Registration returns a one-time registry token. Send it on update endpoints as:

```text
Authorization: Bearer <registry_token>
```

or:

```text
X-PatchHive-Registry-Token: <registry_token>
```

Tokens are stored as SHA-256 hashes. Public endpoints only return installs that
were registered with `install_mode: "public-demo"` and have submitted a snapshot.

## Example

```bash
curl -s http://127.0.0.1:8130/v1/installs/register \
  -H 'content-type: application/json' \
  -d '{"install_mode":"public-demo","display_name":"PatchHive public demo","public_slug":"patchhive-public-demo"}'
```
