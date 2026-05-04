# patchhive-launcher

`patchhive-launcher` is a small localhost-only helper service for HiveCore.

Its job is intentionally narrow:

- detect whether the PatchHive monorepo and `docker compose` are available
- prepare `.env` files for the first stack when needed
- sync `PATCHHIVE_SUITE_BOOTSTRAP_SECRET` into HiveCore and the first-stack products
- start or stop the first stack with approved `docker compose` commands

HiveCore remains the brain. The launcher is only the host-control layer that
can touch Docker and local product directories safely from one place.

## Endpoints

- `GET /health`
- `GET /stacks/first`
- `POST /stacks/first/start`
- `POST /stacks/first/stop`

## Local Run

```bash
cd services/patchhive-launcher
cargo run
```

Default bind:

- `http://127.0.0.1:8210`

Optional env:

- `PATCHHIVE_LAUNCHER_BIND_ADDR`
- `PATCHHIVE_MONOREPO_ROOT`
- `RUST_LOG`
