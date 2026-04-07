// theme.js — PatchHive base dark theme
// Each product imports this and optionally overrides --accent

export const BASE_THEME = {
  "--bg":          "#080810",
  "--bg-panel":    "#0d0d18",
  "--bg-input":    "#10101e",
  "--bg-hover":    "#14142a",
  "--border":      "#1c1c30",
  "--border-in":   "#222238",
  "--text":        "#d4d4e8",
  "--text-dim":    "#484868",
  "--text-muted":  "#282840",
  "--text-input":  "#8888bb",
  "--gold":        "#c8922a",
  "--purple":      "#7b2d8b",
  "--green":       "#2a8a4a",
  "--blue":        "#2a6aaa",
  "--red":         "#c41e3a",
};

// Product accent palettes — import and pass to applyTheme
export const PRODUCT_THEMES = {
  "repo-reaper":   { "--accent": "#c41e3a", "--accent-dim": "#8b1528" }, // crimson
  "signal-hive":   { "--accent": "#2a6aaa", "--accent-dim": "#1a4a7a" }, // blue
  "review-bee":    { "--accent": "#c8922a", "--accent-dim": "#8b6018" }, // amber
  "trust-gate":    { "--accent": "#7b2d8b", "--accent-dim": "#4a1a55" }, // purple
  "repo-memory":   { "--accent": "#2a8a4a", "--accent-dim": "#1a5a30" }, // green
  "merge-keeper":  { "--accent": "#2a6aaa", "--accent-dim": "#1a4a7a" }, // blue
  "flake-sting":   { "--accent": "#c87020", "--accent-dim": "#8b4a10" }, // orange
  "dep-triage":    { "--accent": "#c8922a", "--accent-dim": "#8b6018" }, // amber
  "vuln-triage":   { "--accent": "#c41e3a", "--accent-dim": "#8b1528" }, // crimson
  "refactor-scout":{ "--accent": "#2a8a4a", "--accent-dim": "#1a5a30" }, // green
};

/**
 * Apply PatchHive theme to the document root.
 * @param {string} product - product key from PRODUCT_THEMES, e.g. "repo-reaper"
 */
export function applyTheme(product = "repo-reaper") {
  const root = document.documentElement;
  const vars = { ...BASE_THEME, ...(PRODUCT_THEMES[product] || PRODUCT_THEMES["repo-reaper"]) };
  Object.entries(vars).forEach(([k, v]) => root.style.setProperty(k, v));
}

// Provider metadata — shared across all products
export const PROVIDERS = {
  anthropic: { label:"Anthropic", color:"#c87941", icon:"◆", keyHint:"sk-ant-…" },
  openai:    { label:"OpenAI",    color:"#19a37f", icon:"◉", keyHint:"sk-…" },
  gemini:    { label:"Gemini",    color:"#4a8af0", icon:"◈", keyHint:"AIza…" },
  groq:      { label:"Groq",      color:"#9060e0", icon:"⬡", keyHint:"gsk_…" },
  ollama:    { label:"Ollama",    color:"#40a080", icon:"◎", keyHint:"(no key needed)", noKey:true },
};

// RepoReaper agent roles — product-specific but exported for convenience
export const ROLE_META = {
  scout:      { label:"Scout",      icon:"◎", color:"var(--blue)",   desc:"Hunts repos & scores issues" },
  judge:      { label:"Judge",      icon:"⚖", color:"var(--gold)",   desc:"Targets relevant files" },
  reaper:     { label:"Reaper",     icon:"⚔", color:"var(--accent)", desc:"Forges the killing patch" },
  smith:      { label:"Smith",      icon:"⬢", color:"var(--purple)", desc:"Refines & improves patches" },
  gatekeeper: { label:"Gatekeeper", icon:"🔒", color:"var(--green)",  desc:"Validates & opens PRs" },
};

export const STATUS_LABELS = {
  queued:   "Queued",
  running:  "In progress",
  fixed:    "Kill confirmed",
  skipped:  "Escaped",
  rejected: "Rejected by smith",
  error:    "Error",
};

export const PHASE_LABEL = { scan:"Hunting", triage:"Judging", fix:"Reaping", done:"Done" };
export const PHASE_ICON  = { scan:"◎", triage:"⚖", fix:"⚔", done:"✓" };
