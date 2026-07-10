# VulnTriage UI v3

This is the first real product integration of the Lovable-derived PatchHive UI
v3. It stays separate from the active v2 frontend until its parity audit and
promotion are complete.

It currently wires the Lovable visual system to VulnTriage's real API-key auth,
health, startup checks, overview, history, scan detail, finding detail, source
configuration, and scan dispatch endpoints.

```bash
npm install
npm run dev
```

The development server listens on `http://127.0.0.1:5300` and proxies `/api` to
the standalone VulnTriage backend on `http://127.0.0.1:8110`.

Docker keeps v3 opt-in during the migration:

```bash
docker compose --profile v3-ui up --build backend frontend-v3
```

The UI is then available at `http://localhost:5300`.
