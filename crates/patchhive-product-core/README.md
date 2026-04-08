# patchhive-product-core

Shared Rust backend primitives for PatchHive products.

This crate is the first shared Rust layer for code that is already repeated across multiple PatchHive products.

## Current Scope

- API-key auth hashing, verification, persistence, and middleware enforcement
- typed startup checks with shared logging helpers

## Intent

`patchhive-product-core` should hold the Rust backend seams that are truly shared across 2 or more products.

Product-specific GitHub logic, scoring heuristics, pipelines, and route behavior should stay inside the products until they are clearly generic.
