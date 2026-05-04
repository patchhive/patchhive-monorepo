# patchhive-launcher

`patchhive-launcher` is a small localhost-only helper service for HiveCore.

Its job is intentionally narrow:

- detect whether the PatchHive monorepo and `docker compose` are available
- report product preflight details for compose files, `.env`, suite bootstrap, ports, and running state
- prepare `.env` files for the first stack when needed
- sync `PATCHHIVE_SUITE_BOOTSTRAP_SECRET` into HiveCore and the first-stack products
- pull published GHCR images first, then start products without local builds when images are available
- fall back to local `docker compose up -d --build` for development or missing images
- start, stop, restart, or read recent logs for first-stack products with approved `docker compose` commands

HiveCore remains the brain. The launcher is only the host-control layer that
can touch Docker and local product directories safely from one place.

## Endpoints

- `GET /health`
- `GET /products`
- `POST /products/:slug/start`
- `POST /products/:slug/stop`
- `POST /products/:slug/restart`
- `GET /products/:slug/logs?tail=120`
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
- `PATCHHIVE_LAUNCHER_IMAGE_MODE`
- `PATCHHIVE_MONOREPO_ROOT`
- `PATCHHIVE_IMAGE_TAG`
- `PATCHHIVE_SUITE_BOOTSTRAP_SECRET`
- `RUST_LOG`

Image mode:

- `pull` (default): `docker compose pull`, then `docker compose up -d --no-build`, with local build fallback
- `pull-only`: require published images and fail instead of building locally
- `build`: skip image pulls and always build locally

The first-stack compose files default to GHCR images tagged by `PATCHHIVE_IMAGE_TAG`
(`main` by default), while still keeping `build:` entries for fallback and local
development.

## Smoke Check

With the launcher running:

```bash
./scripts/smoke-launcher.sh
```

The smoke script checks health, product preflight, and first-stack readiness without starting containers. To intentionally run `docker compose up -d --build` for the first stack:

```bash
PATCHHIVE_SMOKE_START=1 ./scripts/smoke-launcher.sh
```
