# PatchHive Sites

Static HTML pages for PatchHive product sites. No build step — serve directly with any static file server (Caddy, nginx, etc).

## Structure

```
apps/
  patchhive/      → Main site (patchhive.dev)
  signalhive/     → patchhive.dev/signalhive
  reviewbee/      → patchhive.dev/reviewbee
  trustgate/      → patchhive.dev/trustgate
  repomemory/     → patchhive.dev/repomemory
  mergekeeper/    → patchhive.dev/mergekeeper
  flakesting/     → patchhive.dev/flakesting
  deptriage/      → patchhive.dev/deptriage
  vulntriage/     → patchhive.dev/vulntriage
  refactorscout/  → patchhive.dev/refactorscout
  reporeaper/     → patchhive.dev/reporeaper
  hivecore/       → patchhive.dev/hivecore
  failguard/      → patchhive.dev/failguard (cross-cutting capability)
  contributor/    → patchhive.dev/contributor
  transparency/   → patchhive.dev/transparency
assets/
  patchhive3.png  → shared logo asset
```

## Deploy

Each subfolder has its own `index.html`. You need individual `handle` blocks in Caddy — a single `root` won't work because there's no index in the apps/ root.

Caddyfile:
```
patchhive.dev {
    handle /assets* {
        root * /path/to/patchhive-sites
        file_server
    }

    handle /signalhive* {
        root * /path/to/patchhive-sites/apps/signalhive
        try_files /index.html
        file_server
    }

    handle /reporeaper* {
        root * /path/to/patchhive-sites/apps/reporeaper
        try_files /index.html
        file_server
    }

    handle /contributor* {
        root * /path/to/patchhive-sites/apps/contributor
        try_files /index.html
        file_server
    }

    handle /transparency* {
        root * /path/to/patchhive-sites/apps/transparency
        try_files /index.html
        file_server
    }

    # ... repeat for each product
    # try_files /index.html is required so /productname serves index.html

    handle {
        root * /path/to/patchhive-sites/apps/patchhive
        file_server
    }
}
```

## Reference Docs

- `Patchive.md` — Project overview
- `Plan_v1___Plan_Product_Sites` — Site planning doc
- `Site_Wireframes_—_All_Products.md` — Wireframe reference
