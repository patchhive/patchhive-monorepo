# PatchHive Registry Service

`patchhive-registry` is the hosted-service MVP for the PatchHive Registry plan.
It accepts opt-in, sanitized PatchHive suite snapshots and exposes only public
`public-demo` records.

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

## API

- `GET /health`
- `POST /v1/installs/register`
- `POST /v1/installs/:install_id/heartbeat`
- `POST /v1/installs/:install_id/smoke`
- `GET /v1/public/installs`
- `GET /v1/public/installs/:public_slug`

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
