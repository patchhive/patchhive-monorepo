# VulnTriage UI

This is the canonical VulnTriage frontend, built on the Lovable-derived
PatchHive UI v3 system. The v1 and v2 implementations were removed after the
final parity audit.

It currently wires the Lovable visual system to VulnTriage's real API-key auth,
health, startup checks, overview, history, scan detail, finding detail, source
configuration, and scan dispatch endpoints.

```bash
npm install
npm run dev
```

The development server listens on `http://127.0.0.1:5300` and talks to the
unified backend on port 8100 by default. To use the standalone backend on port
8110, start the frontend with `VITE_API_URL=/api npm run dev`; Vite proxies
`/api` to that backend.

Docker runs this frontend by default:

```bash
docker compose up --build backend frontend
```

The Docker UI is available at `http://localhost:5181`. The direct Vite
development server remains available at `http://127.0.0.1:5300`.
