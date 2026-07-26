# PatchHive UI v2 — quarantined compatibility package

This package is not part of the active specialist product architecture and is
excluded from the root npm workspace. Specialist products use
`@patchhivehq/ui-v3` from their canonical `frontend/` directories.

The files remain only because an untouched HiveCore compatibility frontend
still imports them. Do not add features, products, or new consumers here. The
package can be deleted when HiveCore is explicitly brought into scope and that
remaining consumer is retired or replaced.
