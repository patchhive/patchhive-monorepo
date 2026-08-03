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

/** API slug → the deck's historical hyphen-free id. */
export const ID_BY_SLUG: Record<string, string> = {};

/** The deck's product id → API slug. Derived, so the two cannot disagree. */
export const SLUG_BY_ID: Record<string, string> = {};

/** Display names, keyed by API slug. */
export const TITLE_BY_SLUG: Record<string, string> = {};

export function registerProductSlug(slug: string, title: string): string {
  const id = slug.replaceAll("-", "");
  ID_BY_SLUG[slug] = id;
  SLUG_BY_ID[id] = slug;
  TITLE_BY_SLUG[slug] = title;
  return id;
}

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
