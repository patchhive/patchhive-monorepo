// The one place the deck translates between its product ids and the API's slugs.
//
// This mapping existed in four copies — twice inside live-sync.ts four lines apart,
// once in token-status.ts, and it was about to be written a fourth time for runbooks.
// It is exactly the shape the repository-policy work was about: several stores of the
// same fact, agreeing right up until one is edited. Adding a product to three of four
// copies produces a deck where a panel silently drops it, which is indistinguishable
// from a product that is genuinely absent.
//
// The deck's ids are historical (`vulntriage`); the API's keys are kebab-case product
// slugs (`vuln-triage`). Neither is derivable from the other by rule — `repo-reaper`
// is `reporeaper` but `hive-core` could as easily have been `hivecore` or `core` — so
// the mapping is explicit and lives here.

/** API slug → the deck's product id. */
export const ID_BY_SLUG: Record<string, string> = {
  "repo-reaper": "reporeaper",
  "signal-hive": "signalhive",
  "trust-gate": "trustgate",
  "repo-memory": "repomemory",
  "review-bee": "reviewbee",
  "merge-keeper": "mergekeeper",
  "flake-sting": "flakesting",
  "dep-triage": "deptriage",
  "vuln-triage": "vulntriage",
  "refactor-scout": "refactorscout",
  "release-sentry": "releasesentry",
  "hive-core": "hivecore",
};

/** The deck's product id → API slug. Derived, so the two cannot disagree. */
export const SLUG_BY_ID: Record<string, string> = Object.fromEntries(
  Object.entries(ID_BY_SLUG).map(([slug, id]) => [id, slug]),
);

/** Display names, keyed by API slug. */
export const TITLE_BY_SLUG: Record<string, string> = {
  "repo-reaper": "RepoReaper",
  "signal-hive": "SignalHive",
  "trust-gate": "TrustGate",
  "repo-memory": "RepoMemory",
  "review-bee": "ReviewBee",
  "merge-keeper": "MergeKeeper",
  "flake-sting": "FlakeSting",
  "dep-triage": "DepTriage",
  "vuln-triage": "VulnTriage",
  "refactor-scout": "RefactorScout",
  "release-sentry": "ReleaseSentry",
  "hive-core": "HiveCore",
};

/**
 * Falls back to the input rather than throwing.
 *
 * A product the deck does not recognise should still reach the API under whatever
 * key it arrived with — an unknown product is better handled by the backend saying
 * "unknown product" than by the browser inventing a slug or dropping the request.
 */
export function slugForId(id: string): string {
  return SLUG_BY_ID[id] ?? id;
}

export function idForSlug(slug: string): string {
  return ID_BY_SLUG[slug] ?? slug;
}
