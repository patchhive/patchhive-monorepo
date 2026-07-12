import { useEffect } from "react";
import { Github, LogOut, Moon, Radio, Sun } from "lucide-react";
import { usePatchHiveTheme } from "./theme.jsx";
import { V3_TEXT } from "./tokens.js";

export { PATCHHIVE_THEME_BOOTSTRAP, PATCHHIVE_THEME_KEY, usePatchHiveTheme } from "./theme.jsx";
export { V3_TEXT } from "./tokens.js";

const PRODUCT_BRANDS = {
  "repo-reaper": { name: "RepoReaper", subtitle: "patch execution cell" },
  "signal-hive": { name: "SignalHive", subtitle: "reconnaissance cell" },
  "review-bee": { name: "ReviewBee", subtitle: "review resolution cell" },
  "trust-gate": { name: "TrustGate", subtitle: "trust guard cell" },
  "repo-memory": { name: "RepoMemory", subtitle: "memory comb" },
  "merge-keeper": { name: "MergeKeeper", subtitle: "comb readiness" },
  "flake-sting": { name: "FlakeSting", subtitle: "CI sting detector" },
  "dep-triage": { name: "DepTriage", subtitle: "dependency comb" },
  "vuln-triage": { name: "VulnTriage", subtitle: "security comb" },
  "refactor-scout": { name: "RefactorScout", subtitle: "cleanup scout cell" },
  "release-sentry": { name: "ReleaseSentry", subtitle: "release watch cell" },
  "hive-core": { name: "HiveCore", subtitle: "control center" },
};

export function ThemeToggle({ className = "" }) {
  const { dark, toggleTheme } = usePatchHiveTheme();
  return (
    <button
      type="button"
      onClick={toggleTheme}
      aria-label={dark ? "Switch to light mode" : "Switch to dark mode"}
      aria-pressed={dark}
      className={`surface-inset h-9 w-9 rounded-full grid place-items-center hover:brightness-110 theme-transition ${className}`}
    >
      {dark
        ? <Sun size={14} className={V3_TEXT.body} />
        : <Moon size={14} className={V3_TEXT.body} />}
    </button>
  );
}

export function ProductShell({ children, productKey }) {
  const brand = PRODUCT_BRANDS[productKey] || { name: productKey, subtitle: "maintenance cell" };
  useEffect(() => {
    const root = document.documentElement;
    const previous = root.dataset.product;
    root.dataset.product = productKey;
    return () => {
      if (previous) root.dataset.product = previous;
      else delete root.dataset.product;
    };
  }, [productKey]);

  return (
    <main
      className="theme-transition min-h-screen relative overflow-hidden"
      style={{ background: "var(--page-bg)", color: "var(--text-body)" }}
    >
      <div aria-hidden className="pointer-events-none fixed inset-0 z-0 overflow-hidden">
        <div className="absolute top-[10%] left-[8%] h-72 w-72 rounded-full opacity-60 blur-3xl dark:opacity-45" style={{ backgroundImage: "var(--orb-1)" }} />
        <div className="absolute top-[40%] right-[6%] h-96 w-96 rounded-full opacity-50 blur-3xl dark:opacity-40" style={{ backgroundImage: "var(--orb-2)" }} />
        <div className="absolute bottom-[6%] left-[30%] h-80 w-80 rounded-full opacity-40 blur-3xl dark:opacity-30" style={{ backgroundImage: "var(--orb-3)" }} />
      </div>
      <div aria-hidden className="pointer-events-none fixed inset-0 z-0 opacity-[0.04] v3-noise" />
      <div className="relative z-10">{children}</div>
      <footer className="relative z-10 px-6 pb-8">
        <div className={`surface-dim mx-auto max-w-[1440px] px-5 py-4 grid gap-1 sm:grid-cols-3 sm:gap-4 text-[11px] ${V3_TEXT.mute}`}>
          <span className={`font-semibold ${V3_TEXT.strong}`}>{brand.name} by PatchHive</span>
          <span className="sm:text-center">{brand.subtitle}</span>
          <span className="sm:text-right">Autonomous maintenance suite</span>
        </div>
      </footer>
    </main>
  );
}

export function ProductHeader({
  activeTab,
  githubLabel = "patchhive",
  icon: Icon,
  onRun,
  onSignOut,
  onTabChange,
  productName,
  runDisabled = false,
  runLabel = "Run scan",
  subtitle,
  tabs = [],
}) {
  return (
    <header className="px-3 sm:px-6 pt-3 sm:pt-6">
      <div className="surface mx-auto max-w-[1440px] px-3 sm:px-5 min-h-16 flex items-center justify-between gap-2 overflow-hidden">
        <div className="flex shrink-0 items-center gap-3">
          <div className="h-10 w-10 rounded-xl grid place-items-center text-white" style={{ backgroundImage: "linear-gradient(135deg, var(--accent-2), var(--accent-3))", boxShadow: "var(--accent-glow)" }}>
            {Icon ? <Icon size={18} /> : null}
          </div>
          <div className="leading-tight">
            <div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>PatchHive · {productName}</div>
            <div className={`font-display text-[16px] font-semibold tracking-tight ${V3_TEXT.strong}`}>
              {productName} <span className={`${V3_TEXT.dim} font-normal hidden lg:inline`}>— {subtitle}</span>
            </div>
          </div>
        </div>
        <nav className="surface-inset hidden min-w-0 items-center gap-1 overflow-x-auto rounded-full p-1 md:flex" aria-label={`${productName} sections`}>
          {tabs.map((tab) => (
            <button
              key={tab.id}
              type="button"
              onClick={() => onTabChange?.(tab.id)}
              className={`h-8 shrink-0 whitespace-nowrap rounded-full px-4 text-[12px] transition ${activeTab === tab.id ? `bg-white shadow ${V3_TEXT.strong} dark:bg-white/15` : `${V3_TEXT.mute} hover:opacity-100`}`}
            >
              {tab.label}
            </button>
          ))}
        </nav>
        <div className="flex shrink-0 items-center gap-2">
          <ThemeToggle />
          <div className={`surface-inset h-9 px-3 rounded-full text-[12px] ${V3_TEXT.body} hidden sm:flex items-center gap-2`}>
            <Github size={13} /> <span className="hidden sm:inline">{githubLabel}</span>
          </div>
          <button
            type="button"
            onClick={onRun}
            disabled={runDisabled}
            className="h-9 px-3 sm:px-4 rounded-full text-[12px] font-semibold text-white hover:brightness-110 disabled:opacity-50 flex items-center gap-2 whitespace-nowrap"
            style={{ backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))", boxShadow: "var(--accent-glow)" }}
          >
            <Radio size={13} /> {runLabel}
          </button>
          {onSignOut ? (
            <button
              type="button"
              onClick={onSignOut}
              aria-label={`Sign out of ${productName}`}
              className={`surface-inset hidden h-9 w-9 rounded-full place-items-center hover:brightness-110 sm:grid ${V3_TEXT.mute}`}
            >
              <LogOut size={13} />
            </button>
          ) : null}
        </div>
      </div>
      <nav className="surface-inset mx-auto mt-2 flex max-w-[1440px] items-center gap-1 overflow-x-auto rounded-full p-1 md:hidden" aria-label={`${productName} mobile sections`}>
        {tabs.map((tab) => (
          <button
            key={tab.id}
            type="button"
            onClick={() => onTabChange?.(tab.id)}
            className={`h-8 shrink-0 whitespace-nowrap rounded-full px-4 text-[12px] transition ${activeTab === tab.id ? `bg-white shadow ${V3_TEXT.strong} dark:bg-white/15` : `${V3_TEXT.mute} hover:opacity-100`}`}
          >
            {tab.label}
          </button>
        ))}
        {onSignOut ? (
          <button type="button" onClick={onSignOut} className={`h-8 shrink-0 rounded-full px-4 text-[12px] ${V3_TEXT.mute}`}>
            Sign out
          </button>
        ) : null}
      </nav>
    </header>
  );
}

export function Surface({ as: Component = "div", className = "", children, ...props }) {
  return <Component className={`surface ${className}`} {...props}>{children}</Component>;
}

export function MetricCard({ label, value, footerLeft, footerRight, tone = "from-stone-500/70 to-stone-800/60", icon: Icon }) {
  return (
    <div className="surface p-5 overflow-hidden">
      <div className={`absolute -top-8 -right-8 h-28 w-28 rounded-full bg-gradient-to-br ${tone} blur-2xl`} />
      <div className="relative">
        <div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>{label}</div>
        <div className={`mt-3 font-display text-[46px] font-semibold tabular-nums ${V3_TEXT.strong} leading-none`}>
          {String(value ?? 0)}
        </div>
        <div className={`mt-4 flex flex-col items-start gap-1 text-[11px] sm:flex-row sm:items-center sm:justify-between ${V3_TEXT.mute}`}>
          <span className="flex items-center gap-1">{Icon ? <Icon size={11} /> : null}{footerLeft}</span>
          <span>{footerRight}</span>
        </div>
      </div>
    </div>
  );
}

export { countLabel, IntegratedProductApp, ProductLoginScreen } from "./integrated-product.jsx";
export { ActivityTimeline, CopyMarkdownButton, DashboardControls, GitHubPermissionGuidance, GuidanceNotice, HistoryDashboard, ProgressiveList, ScanWarnings, StartupCheckList, useSavedDashboardViews } from "./workspace.jsx";
