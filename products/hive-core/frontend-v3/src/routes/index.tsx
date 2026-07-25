import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useMemo, useState } from "react";
import {
  Activity,
  AlertTriangle,
  Bell,
  BellOff,
  CheckCircle2,
  CircleDot,
  Command,
  Cpu,
  ExternalLink,
  Filter,
  GitBranch,
  Hexagon,
  KeyRound,
  Palette,
  Radio,
  Repeat,
  Search,
  Settings,
  Sparkles,
  Terminal,
  X,
  Zap,
} from "lucide-react";
import { PRODUCTS, RUNS, type Product, type RunEvent, type Status } from "@/lib/hive-data";
import { Compass, FlaskConical, Users } from "lucide-react";
import {
  HiveCommand,
  HiveCommandProvider,
  useHiveCommand,
} from "@/components/hive-command";
import { RunDetailDrawer } from "@/components/run-detail-drawer";
import { deriveFailure } from "@/lib/run-failure";
import { IncidentTimeline } from "@/components/incident-timeline";
import { ContractDrift } from "@/components/contract-drift";
import { CapabilitySearch } from "@/components/capability-search";
import { TokenVault } from "@/components/token-vault";
import { LiveTail } from "@/components/live-tail";
import { RunHeatmap } from "@/components/run-heatmap";
import { AuditLog } from "@/components/audit-log";
import { AskHive } from "@/components/ask-hive";
import { RunbookDrawer } from "@/components/runbook-drawer";
import { GuidedTour } from "@/components/guided-tour";
import { PresenceCursors } from "@/components/presence-cursors";
import { Sparkline } from "@/components/sparkline";
import { latencyHistory, latencyStats, runAnomalyZ } from "@/lib/hive-metrics";
import { EmptyHex } from "@/components/empty-hex";
import { DispatchPreview } from "@/components/dispatch-preview";
import { RunbookHistory } from "@/components/runbook-history";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { BookOpen } from "lucide-react";
import { useLiveSuite } from "@/lib/live-sync";


type IndexSearch = { run?: string; filter?: "all" | "warn" | "crit" };

const VALID_FILTERS = new Set(["all", "warn", "crit"]);
const RUN_ID_RE = /^[a-zA-Z0-9_-]{1,64}$/;
// RUNS is replaced on every sync, so membership must be checked when the link is
// read rather than snapshotted at module load.
const isKnownRunId = (id: string) => RUNS.some((run) => run.id === id);

// Runs on the client on every navigation.
// Anything that fails validation is silently dropped, so a hand-crafted
// or stale link degrades to the safe default deck state instead of
// breaking downstream consumers.
function sanitizeSearch(s: Record<string, unknown>): IndexSearch {
  const rawRun = typeof s.run === "string" ? s.run : undefined;
  const run =
    rawRun && RUN_ID_RE.test(rawRun) && isKnownRunId(rawRun) ? rawRun : undefined;

  const rawFilter = typeof s.filter === "string" ? s.filter : undefined;
  const filter =
    rawFilter && VALID_FILTERS.has(rawFilter)
      ? (rawFilter as IndexSearch["filter"])
      : undefined;

  return { run, filter: filter === "all" ? undefined : filter };
}

export const Route = createFileRoute("/")({
  validateSearch: (s: Record<string, unknown>): IndexSearch => sanitizeSearch(s),
  component: Deck,
});

const statusColor: Record<Status, string> = {
  ok: "text-[var(--ok)]",
  warn: "text-[var(--warn)]",
  crit: "text-[var(--crit)]",
  offline: "text-muted-foreground",
};
const statusDot: Record<Status, string> = {
  ok: "bg-[var(--ok)] shadow-[0_0_12px_var(--ok)]",
  warn: "bg-[var(--warn)] shadow-[0_0_12px_var(--warn)]",
  crit: "bg-[var(--crit)] shadow-[0_0_12px_var(--crit)]",
  offline: "bg-muted-foreground",
};

/** Operator-local wall clock, 12-hour. The deck is a single-operator console; UTC
 * was the wrong default and the seconds field is what makes polling feel live. */
function useClock() {
  const [t, setT] = useState<Date | null>(null);
  useEffect(() => {
    setT(new Date());
    const i = setInterval(() => setT(new Date()), 1000);
    return () => clearInterval(i);
  }, []);
  return t;
}

function Deck() {
  return (
    <HiveCommandProvider>
      <DeckInner />
      <HiveCommand />
    </HiveCommandProvider>
  );
}

function DeckInner() {
  const now = useClock();
  const { scanlineOn } = useHiveCommand();
  // Patches PRODUCTS in place and bumps `version`; this component re-rendering is
  // what carries the refreshed status down to every panel below.
  const suite = useLiveSuite();
  const ok = PRODUCTS.filter((p) => p.status === "ok").length;
  const warn = PRODUCTS.filter((p) => p.status === "warn").length;
  // "offline" covers both unreachable and engine-pending. Counting only `crit`
  // left products missing from the strip entirely — 11/0/0 across twelve products.
  const down = PRODUCTS.filter((p) => p.status === "crit" || p.status === "offline").length;

  // Both derived from the real run index rather than seeded per-product numbers.
  const runStats = useMemo(() => {
    const cutoff = Date.now() - 24 * 3_600_000;
    let recent = 0;
    let failed = 0;
    for (const run of RUNS) {
      const at = Date.parse(run.startedAt);
      if (!Number.isNaN(at) && at >= cutoff) recent += 1;
      if (run.status === "failed") failed += 1;
    }
    return { total: RUNS.length, recent, failed };
  }, [suite.version]);

  return (
    <main className="hex-grid relative min-h-screen pb-16">
      <URLSync />
      <SanitizedSearchBanner />
      {scanlineOn && <div className="pointer-events-none absolute inset-0 scanline opacity-40" />}
      <div className="relative mx-auto max-w-[1400px] px-6 py-8 lg:px-10">
        <TopBar now={now} />
        {suite.error && <SyncNotice error={suite.error} />}
        <section id="deck" className="scroll-mt-24 rounded-md">
          <Hero />
          <KpiStrip
            ok={ok}
            warn={warn}
            down={down}
            runStats={runStats}
            live={suite.live}
          />
        </section>
        <section id="registry" className="mt-8 grid scroll-mt-24 gap-6 rounded-md lg:grid-cols-[1fr_380px]">
          <Registry />
          <div id="runs" className="scroll-mt-24"><RunsFeed syncVersion={suite.version} /></div>
        </section>
        <IncidentTimeline syncVersion={suite.version} />
        <ContractDrift syncVersion={suite.version} />
        <CapabilitySearch />
        <TokenVault />
        <RunHeatmap />
        <CapabilityGrid />
        <AskHive />
        <RunbookHistory />
        <AuditLog />
        <Footer />
      </div>
      <LiveTail />
      <RunbookDrawer />
      <GuidedTour />
      <PresenceCursors />
      <DispatchPreviewWrapper />
    </main>
  );
}

function SyncNotice({ error }: { error: string }) {
  return (
    <div className="mt-4 rounded-lg border border-[var(--crit)]/40 bg-[var(--crit)]/[0.06] px-4 py-2.5 backdrop-blur">
      <div className="font-display text-[10px] font-bold uppercase tracking-wider text-[var(--crit)]">
        Not syncing
      </div>
      <p className="mt-0.5 text-xs text-muted-foreground">
        {error} Product status is showing seed values, not live state.
      </p>
    </div>
  );
}

function URLSync() {
  const search = Route.useSearch();
  const navigate = Route.useNavigate();
  const { registryFilter, setRegistryFilter } = useHiveCommand();

  // URL → context (on load / back-forward)
  useEffect(() => {
    const desired = search.filter ?? "all";
    if (desired !== registryFilter) setRegistryFilter(desired);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search.filter]);

  // context → URL (when user changes via palette / X-chip)
  useEffect(() => {
    const current = search.filter ?? "all";
    if (current === registryFilter) return;
    navigate({
      search: (p: IndexSearch) => ({ ...p, filter: registryFilter === "all" ? undefined : registryFilter }),
      replace: true,
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [registryFilter]);

  return null;
}

function SanitizedSearchBanner() {
  const [dropped, setDropped] = useState<{ key: string; value: string; reason: string }[]>([]);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    if (typeof window === "undefined") return;
    const params = new URLSearchParams(window.location.search);
    const bad: { key: string; value: string; reason: string }[] = [];
    const rawRun = params.get("run");
    if (rawRun !== null) {
      if (!RUN_ID_RE.test(rawRun)) bad.push({ key: "run", value: rawRun, reason: "malformed id" });
      else if (!isKnownRunId(rawRun)) bad.push({ key: "run", value: rawRun, reason: "unknown run" });
    }
    const rawFilter = params.get("filter");
    if (rawFilter !== null && !VALID_FILTERS.has(rawFilter)) {
      bad.push({ key: "filter", value: rawFilter, reason: "not one of all|warn|crit" });
    }
    if (bad.length) setDropped(bad);
  }, []);

  if (dismissed || dropped.length === 0) return null;

  return (
    <div className="mx-auto mt-4 max-w-[1400px] px-6 lg:px-10">
      <div className="flex items-start justify-between gap-3 rounded-lg border border-[var(--warn)]/40 bg-[var(--warn)]/[0.06] px-4 py-2.5 text-xs text-foreground">
        <div className="flex items-start gap-2">
          <AlertTriangle className="mt-0.5 h-3.5 w-3.5 flex-shrink-0 text-[var(--warn)]" />
          <div>
            <div className="font-display text-[10px] font-bold uppercase tracking-wider text-[var(--warn)]">
              URL sanitized
            </div>
            <div className="mt-0.5 text-muted-foreground">
              Dropped invalid deep-link {dropped.length === 1 ? "parameter" : "parameters"}:{" "}
              {dropped.map((d, i) => (
                <span key={d.key}>
                  {i > 0 && ", "}
                  <code className="rounded bg-background/60 px-1 py-0.5 font-mono text-[10px] text-foreground">
                    {d.key}={d.value.length > 24 ? `${d.value.slice(0, 24)}…` : d.value}
                  </code>{" "}
                  <span className="text-muted-foreground/70">({d.reason})</span>
                </span>
              ))}
              . Falling back to default deck state.
            </div>
          </div>
        </div>
        <button
          onClick={() => setDismissed(true)}
          className="flex-shrink-0 rounded p-1 text-muted-foreground transition hover:bg-background/60 hover:text-foreground"
          aria-label="Dismiss"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>
    </div>
  );
}

function DispatchPreviewWrapper() {
  const { dispatchPreviewOpen, setDispatchPreviewOpen } = useHiveCommand();
  return <DispatchPreview open={dispatchPreviewOpen} onOpenChange={setDispatchPreviewOpen} />;
}


function TopBar({ now }: { now: Date | null }) {
  const {
    setOpen,
    cycleTheme,
    theme,
    soundOn,
    toggleSound,
    setCheatsheetOpen,
    demoMode,
    toggleDemo,
    presenceOn,
    togglePresence,
    setTourOpen,
    setDispatchPreviewOpen,
  } = useHiveCommand();
  const navItems = [
    { icon: Command, label: "Deck", id: "deck" },
    { icon: Radio, label: "Registry", id: "registry" },
    { icon: Terminal, label: "Runs", id: "runs" },
    { icon: AlertTriangle, label: "Incidents", id: "incidents" },
    { icon: GitBranch, label: "Deps", id: "dependencies" },
    { icon: KeyRound, label: "Tokens", id: "tokens" },
  ];
  return (
    <header className="flex items-center justify-between gap-4 border-b border-border/60 pb-4">
      <div className="flex items-center gap-3">
        <div className="relative grid h-10 w-10 place-items-center">
          <Hexagon className="absolute inset-0 h-full w-full text-[var(--honey)]" strokeWidth={1.4} />
          <Hexagon className="h-5 w-5 fill-[var(--honey)] text-[var(--honey)]" />
        </div>
        <div className="leading-tight">
          <div className="font-display text-lg font-bold tracking-tight">
            <span className="text-gradient-honey">HIVE</span>
            <span className="text-foreground">CORE</span>
          </div>
          <div className="font-display text-[10px] uppercase tracking-[0.25em] text-muted-foreground">
            patchhive control plane // v0.1
          </div>
        </div>
      </div>
      <nav className="hidden items-center gap-1 md:flex">
        {navItems.map((n, i) => (
          <button
            key={n.label}
            onClick={() => {
              const el = document.getElementById(n.id);
              el?.scrollIntoView({ behavior: "smooth", block: "start" });
            }}
            className={`group flex items-center gap-2 rounded-md px-3 py-1.5 font-display text-xs uppercase tracking-wider transition ${
              i === 0
                ? "bg-[color-mix(in_oklab,var(--honey)_15%,transparent)] text-[var(--honey)]"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            <n.icon className="h-3.5 w-3.5" />
            {n.label}
          </button>
        ))}
      </nav>
      <div className="flex items-center gap-2">
        <button
          onClick={cycleTheme}
          className="rounded-md border border-border bg-card/60 p-1.5 text-muted-foreground transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)]"
          title={`Theme: ${theme} (press t)`}
          aria-label="Cycle theme"
        >
          <Palette className="h-3.5 w-3.5" />
        </button>
        <button
          onClick={toggleSound}
          className={`rounded-md border border-border bg-card/60 p-1.5 transition hover:border-[var(--honey)]/50 ${soundOn ? "text-[var(--honey)]" : "text-muted-foreground hover:text-[var(--honey)]"}`}
          title={soundOn ? "Crit alarm armed (press m to mute)" : "Crit alarm muted (press m to arm)"}
          aria-label="Toggle crit alarm"
        >
          {soundOn ? <Bell className="h-3.5 w-3.5" /> : <BellOff className="h-3.5 w-3.5" />}
        </button>
        <button
          onClick={togglePresence}
          className={`rounded-md border border-border bg-card/60 p-1.5 transition hover:border-[var(--honey)]/50 ${presenceOn ? "text-[var(--honey)]" : "text-muted-foreground hover:text-[var(--honey)]"}`}
          title={presenceOn ? "Operator presence on" : "Operator presence off"}
          aria-label="Toggle operator presence"
        >
          <Users className="h-3.5 w-3.5" />
        </button>
        <button
          onClick={toggleDemo}
          className={`rounded-md border border-border bg-card/60 p-1.5 transition hover:border-[var(--honey)]/50 ${demoMode ? "text-[var(--honey)]" : "text-muted-foreground hover:text-[var(--honey)]"}`}
          title={demoMode ? "Demo mode on" : "Demo mode off"}
          aria-label="Toggle demo mode"
        >
          <FlaskConical className="h-3.5 w-3.5" />
        </button>
        <button
          onClick={() => setTourOpen(true)}
          className="rounded-md border border-border bg-card/60 p-1.5 text-muted-foreground transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)]"
          title="Start guided tour"
          aria-label="Start guided tour"
        >
          <Compass className="h-3.5 w-3.5" />
        </button>
        <button
          onClick={() => setCheatsheetOpen(true)}
          className="hidden rounded-md border border-border bg-card/60 px-2 py-1 font-display text-[10px] uppercase tracking-wider text-muted-foreground transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)] sm:inline-flex"
          title="Keyboard shortcuts (press ?)"
        >
          <kbd className="font-display text-[10px]">?</kbd>
        </button>
        <button
          onClick={() => setOpen(true)}
          className="group inline-flex items-center gap-2 rounded-md border border-border bg-card/60 px-2.5 py-1.5 font-display text-[10px] uppercase tracking-wider text-muted-foreground backdrop-blur transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)]"
          title="Open command palette"
          aria-label="Open command palette"
        >
          <Search className="h-3.5 w-3.5" />
          <span className="hidden sm:inline">Command</span>
          <kbd className="rounded border border-border bg-background/60 px-1.5 py-0.5 font-display text-[9px] text-foreground">
            ⌘K
          </kbd>
        </button>
        <div className="hidden font-display text-xs text-muted-foreground sm:block" suppressHydrationWarning>
          <span className="text-[var(--honey)]">●</span>{" "}
          {now
            ? now.toLocaleTimeString(undefined, {
                hour: "numeric",
                minute: "2-digit",
                second: "2-digit",
                hour12: true,
              })
            : "—"}
        </div>
        <button
          onClick={() => setDispatchPreviewOpen(true)}
          className="glow-honey rounded-md bg-[var(--honey)] px-4 py-2 font-display text-xs font-bold uppercase tracking-wider text-primary-foreground transition hover:brightness-110"
        >
          Dispatch
        </button>
      </div>
    </header>
  );

}

function Hero() {
  return (
    <section className="mt-10 grid gap-6 lg:grid-cols-[1.4fr_1fr]">
      <div className="animate-float-up">
        <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-[var(--honey)]/30 bg-[color-mix(in_oklab,var(--honey)_8%,transparent)] px-3 py-1 font-display text-[10px] uppercase tracking-[0.3em] text-[var(--honey)]">
          <span className="relative flex h-2 w-2">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-[var(--honey)] opacity-60" />
            <span className="relative inline-flex h-2 w-2 rounded-full bg-[var(--honey)]" />
          </span>
          Suite online · 11 products tracked
        </div>
        <h1 className="font-display text-5xl font-bold leading-[1.05] tracking-tight md:text-7xl">
          One hive.
          <br />
          <span className="text-gradient-honey">Every product.</span>
          <br />
          Zero blind spots.
        </h1>
        <p className="mt-6 max-w-xl text-base text-muted-foreground md:text-lg">
          HiveCore polls every PatchHive product's <code className="rounded bg-muted px-1.5 py-0.5 font-display text-[0.85em] text-[var(--honey)]">/health</code>,
          <code className="ml-1 rounded bg-muted px-1.5 py-0.5 font-display text-[0.85em] text-[var(--honey)]">/capabilities</code>, and
          <code className="ml-1 rounded bg-muted px-1.5 py-0.5 font-display text-[0.85em] text-[var(--honey)]">/runs</code> contracts,
          then surfaces drift, dispatches advertised actions, and brokers service tokens — all from one operational deck.
        </p>
        <div className="mt-8 flex flex-wrap items-center gap-3">
          <button className="glow-honey group inline-flex items-center gap-2 rounded-md bg-[var(--honey)] px-5 py-3 font-display text-sm font-bold uppercase tracking-wider text-primary-foreground transition hover:brightness-110">
            <Zap className="h-4 w-4" /> Bootstrap suite
          </button>
          <button className="inline-flex items-center gap-2 rounded-md border border-border bg-card/60 px-5 py-3 font-display text-sm font-medium uppercase tracking-wider text-foreground backdrop-blur transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)]">
            <Terminal className="h-4 w-4" /> Open registry
          </button>
        </div>
      </div>
      <HiveOrb />
    </section>
  );
}

function HiveOrb() {
  // Pointy-top honeycomb, 4 staggered rows of 3, HiveCore as the queen cell.
  const INITIALS: Record<string, string> = {
    refactorscout: "RF",
    releasesentry: "RS",
  };
  const initialsFor = (p: typeof PRODUCTS[number]) =>
    INITIALS[p.id] ?? p.name.replace(/[a-z]/g, "").slice(0, 3);

  const byId = Object.fromEntries(PRODUCTS.map((p) => [p.id, p]));
  const layout: string[][] = [
    ["reporeaper", "signalhive", "trustgate"],
    ["repomemory", "hivecore", "reviewbee"],
    ["mergekeeper", "flakesting", "deptriage"],
    ["vulntriage", "refactorscout", "releasesentry"],
  ];

  const dotFor = (s: Status) =>
    s === "ok"
      ? "var(--ok)"
      : s === "warn"
        ? "var(--warn)"
        : s === "crit"
          ? "var(--crit)"
          : "var(--muted-foreground)";
  const latColor = (s: Status) =>
    s === "warn" ? "var(--warn)" : s === "crit" || s === "offline" ? "var(--crit)" : "var(--ok)";

  const HEX = "polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%)";

  return (
    <div className="relative overflow-hidden rounded-2xl border border-border bg-card/40 backdrop-blur-sm shadow-2xl">
      {/* Ambient honey glow */}
      <div className="pointer-events-none absolute -top-24 -right-24 h-64 w-64 rounded-full bg-[var(--honey)]/15 blur-3xl" />
      <div className="pointer-events-none absolute -bottom-24 -left-24 h-64 w-64 rounded-full bg-[var(--amber-deep)]/20 blur-3xl" />
      <div className="pointer-events-none absolute inset-0 scanline opacity-20" />

      {/* Header */}
      <div className="relative flex items-center justify-between border-b border-border/60 bg-[var(--honey)]/[0.04] px-5 py-3">
        <div className="flex items-center gap-2">
          <span className="h-2 w-2 animate-pulse rounded-full bg-[var(--honey)] shadow-[0_0_8px_var(--honey)]" />
          <span className="font-display text-[10px] font-bold uppercase tracking-[0.25em] text-[var(--honey)]/85">
            Live mesh
          </span>
        </div>
        <span className="font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
          {PRODUCTS.length}/{PRODUCTS.length} tracking
        </span>
      </div>

      {/* Mesh */}
      <div className="relative flex flex-col items-center gap-0 px-4 py-8 sm:py-10">
        {layout.map((row, rIdx) => (
          <div
            key={rIdx}
            className={`flex gap-1.5 ${rIdx < layout.length - 1 ? "-mb-4" : ""}`}
          >
            {row.map((id, cIdx) => {
              const p = byId[id];
              if (!p) return null;
              const hero = p.id === "hivecore";
              const initials = initialsFor(p);
              const i = rIdx * 3 + cIdx;
              if (hero) {
                return (
                  <div
                    key={p.id}
                    className="animate-float-up relative z-10 -mt-2 flex h-[104px] w-24 items-center justify-center shadow-[0_0_30px_color-mix(in_oklab,var(--honey)_30%,transparent)]"
                    style={{ clipPath: HEX, background: "var(--honey)", animationDelay: `${i * 50}ms` }}
                    title={`${p.name} · ${p.status} · ${p.latencyMs}ms`}
                  >
                    <div
                      className="absolute inset-[1.5px] bg-background"
                      style={{ clipPath: HEX }}
                    />
                    <div
                      className="pointer-events-none absolute inset-0 bg-gradient-to-br from-[var(--honey)]/25 to-transparent"
                      style={{ clipPath: HEX }}
                    />
                    <div className="relative flex flex-col items-center">
                      <span className="font-display text-xl font-black leading-none text-[var(--honey)]">
                        {initials}
                      </span>
                      <span className="mt-0.5 font-mono text-[10px] font-medium tracking-tight text-[var(--honey)]/80">
                        {p.latencyMs}ms
                      </span>
                    </div>
                    <span className="absolute bottom-2 h-1 w-1 animate-ping rounded-full bg-[var(--honey)]" />
                  </div>
                );
              }
              return (
                <div
                  key={p.id}
                  className="animate-float-up relative flex h-[88px] w-20 items-center justify-center bg-card/80"
                  style={{ clipPath: HEX, animationDelay: `${i * 50}ms` }}
                  title={`${p.name} · ${p.status} · ${p.latencyMs}ms`}
                >
                  <span
                    className="absolute top-2 h-1.5 w-1.5 rounded-full"
                    style={{
                      background: dotFor(p.status),
                      boxShadow: `0 0 8px ${dotFor(p.status)}`,
                    }}
                  />
                  <div className="flex flex-col items-center">
                    <span className="font-display text-lg font-bold leading-none text-foreground/75">
                      {initials}
                    </span>
                    <span
                      className="mt-1 font-mono text-[9px]"
                      style={{ color: `color-mix(in oklab, ${latColor(p.status)} 70%, transparent)` }}
                    >
                      {p.status === "crit" || p.status === "offline"
                        ? "OFFLINE"
                        : `${p.latencyMs}ms`}
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        ))}
      </div>

      {/* Footer */}
      <div className="relative flex items-center justify-between border-t border-border/60 bg-background/40 px-5 py-3">
        <div className="flex items-center gap-1.5">
          <span className="h-1.5 w-1.5 rotate-45 border border-[var(--honey)]/50 bg-[var(--honey)]/20" />
          <span className="font-display text-[9px] font-bold uppercase tracking-[0.25em] text-[var(--honey)]/80">
            Mesh health
          </span>
        </div>
        <span className="font-mono text-[10px] text-[var(--honey)]/90">
          {PRODUCTS.filter((p) => p.status === "ok").length}/{PRODUCTS.length} responding
        </span>
      </div>
    </div>
  );
}

function KpiStrip({
  ok,
  warn,
  down,
  runStats,
  live,
}: {
  ok: number;
  warn: number;
  down: number;
  runStats: { total: number; recent: number; failed: number };
  live: boolean;
}) {
  // Every card is now derived from live state: health from the synced registry,
  // run counts from the suite-wide run index. Nothing here is seeded.
  const kpis = [
    {
      label: "Healthy",
      value: ok,
      icon: CheckCircle2,
      accent: "var(--ok)",
      detail: `of ${PRODUCTS.length} products`,
      live,
    },
    {
      label: "Degraded",
      value: warn,
      icon: AlertTriangle,
      accent: "var(--warn)",
      detail: "startup or health errors",
      live,
    },
    {
      label: "Down",
      value: down,
      icon: CircleDot,
      accent: "var(--crit)",
      detail: "unreachable or not mounted",
      live,
    },
    {
      label: "Runs / 24h",
      value: runStats.recent.toLocaleString(),
      icon: Activity,
      accent: "var(--honey)",
      detail: `${runStats.total.toLocaleString()} retained`,
      live,
    },
    {
      label: "Failed runs",
      value: runStats.failed.toLocaleString(),
      icon: Sparkles,
      accent: runStats.failed > 0 ? "var(--crit)" : "var(--honey)",
      detail: "across retained history",
      live,
    },
  ];
  return (
    <div className="mt-10 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
      {kpis.map((k, i) => (
        <div
          key={k.label}
          className="animate-float-up group relative overflow-hidden rounded-lg border border-border bg-card/60 p-4 backdrop-blur transition hover:border-[var(--honey)]/40"
          style={{ animationDelay: `${i * 60}ms` }}
        >
          <div className="absolute inset-x-0 top-0 h-px overflow-hidden">
            <div className="h-full w-1/3 bg-[var(--honey)] animate-sweep" style={{ animationDelay: `${i * 200}ms` }} />
          </div>
          <div className="flex items-center justify-between">
            <span className="font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">{k.label}</span>
            <k.icon className="h-4 w-4" style={{ color: k.accent }} />
          </div>
          <div className="mt-3 font-display text-3xl font-bold tracking-tight" style={{ color: k.accent }}>
            {k.value}
          </div>
          <div className="mt-1 flex items-center gap-1.5 font-display text-[9px] uppercase tracking-wider">
            {k.live ? (
              <span className="text-[var(--ok)]/80">live</span>
            ) : (
              <span className="text-muted-foreground/70" title="Control plane unreachable; showing last known values">
                stale
              </span>
            )}
            <span className="normal-case tracking-normal text-muted-foreground/60">{k.detail}</span>
          </div>
        </div>
      ))}
    </div>
  );
}

function Registry() {
  const { registryFilter, setRegistryFilter, pulseProductId } = useHiveCommand();
  const filtered =
    registryFilter === "all" ? PRODUCTS : PRODUCTS.filter((p) => p.status === registryFilter);
  return (
    <div className="rounded-xl border border-border bg-card/50 backdrop-blur">
      <div className="flex items-center justify-between border-b border-border/60 px-5 py-3">
        <div className="flex items-center gap-2">
          <Radio className="h-4 w-4 text-[var(--honey)]" />
          <h2 className="font-display text-sm font-bold uppercase tracking-[0.2em]">Product Registry</h2>
          {registryFilter !== "all" && (
            <button
              onClick={() => setRegistryFilter("all")}
              className="ml-2 inline-flex items-center gap-1 rounded border border-[var(--honey)]/40 bg-[color-mix(in_oklab,var(--honey)_10%,transparent)] px-2 py-0.5 font-display text-[9px] uppercase tracking-wider text-[var(--honey)] transition hover:brightness-110"
            >
              filter: {registryFilter === "warn" ? "degraded" : "down"}
              <X className="h-2.5 w-2.5" />
            </button>
          )}
        </div>
        <span className="font-display text-[10px] uppercase tracking-wider text-muted-foreground">
          {filtered.length}/{PRODUCTS.length} · polling every 5s
        </span>
      </div>
      <div className="grid gap-px bg-border/40 sm:grid-cols-2">
        {filtered.map((p, i) => (
          <ProductCard key={p.id} product={p} index={i} pulse={pulseProductId === p.id} />
        ))}
      </div>
    </div>
  );
}

function ProductCard({ product, index, pulse }: { product: Product; index: number; pulse?: boolean }) {
  const samples = useMemo(() => latencyHistory(product, 60), [product]);
  const stats = useMemo(() => latencyStats(product), [product]);
  const { openRunbook, setDispatchPreviewOpen } = useHiveCommand();
  const sparkColor =
    product.status === "crit"
      ? "var(--crit)"
      : product.status === "warn"
        ? "var(--warn)"
        : "var(--honey)";
  const [heatmapOpen, setHeatmapOpen] = useState(false);
  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <div
          id={`product-${product.id}`}
          onDoubleClick={() => setHeatmapOpen(true)}
          className={`animate-float-up group relative cursor-pointer bg-card/80 p-4 transition hover:bg-card ${
            pulse ? "ring-2 ring-[var(--honey)] shadow-[0_0_24px_var(--honey)]" : ""
          }`}
          style={{ animationDelay: `${index * 30}ms` }}
          title="Right-click for actions · double-click for heatmap"
        >
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <div className="flex items-center gap-2">
                <span className={`h-2 w-2 rounded-full ${statusDot[product.status]} animate-pulse-dot`} />
                <h3 className="truncate font-display text-sm font-bold tracking-tight">{product.name}</h3>
                {product.contractDrift > 0 && (
                  <span className="rounded bg-[var(--warn)]/15 px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider text-[var(--warn)]">
                    drift {product.contractDrift}
                  </span>
                )}
              </div>
              <p className="mt-0.5 truncate text-xs text-muted-foreground">{product.tagline}</p>
            </div>
            <a
              href={product.frontend}
              className="opacity-0 transition group-hover:opacity-100"
              title="Open frontend"
              onClick={(e) => e.stopPropagation()}
            >
              <ExternalLink className="h-3.5 w-3.5 text-[var(--honey)]" />
            </a>
          </div>
          <div className="mt-3 rounded border border-border/60 bg-background/40 p-2">
            <div className="mb-1 flex items-center justify-between font-display text-[9px] uppercase tracking-[0.18em] text-muted-foreground">
              <span title="Seeded sample series — the suite records no per-request latency">
                latency · 60s · sampled
              </span>
              <span style={{ color: sparkColor }}>
                {product.status === "crit" ? "—" : `${product.latencyMs}ms`}
              </span>
            </div>
            <Sparkline data={samples} color={sparkColor} width={280} height={28} className="w-full" />
            <div className="mt-2 flex items-center gap-1.5 font-display text-[9px] uppercase tracking-wider">
              <PctChip label="p50" value={stats.p50} tone="ok" />
              <PctChip label="p95" value={stats.p95} tone="warn" />
              <PctChip label="p99" value={stats.p99} tone="crit" />
            </div>
          </div>
          <div className="mt-3 grid grid-cols-3 gap-2 font-display text-[10px]">
            <Stat label="latency" value={product.status === "crit" ? "—" : `${product.latencyMs}ms`} tone={product.status} />
            <Stat label="uptime · sampled" value={`${(product.uptime * 100).toFixed(1)}%`} />
            <Stat label="runs 24h" value={product.runs24h.toLocaleString()} />
          </div>
          <div className="mt-3 flex flex-wrap gap-1">
            {product.capabilities.map((c) => (
              <span key={c} className="rounded border border-border bg-muted/40 px-1.5 py-0.5 font-display text-[9px] text-muted-foreground">
                {c}
              </span>
            ))}
          </div>
          {heatmapOpen && (
            <div
              className="absolute inset-0 z-10 flex flex-col rounded border border-[var(--honey)]/50 bg-background/95 p-3 backdrop-blur"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="mb-2 flex items-center justify-between">
                <div className="font-display text-[10px] uppercase tracking-[0.2em] text-[var(--honey)]">
                  {product.name} · 24h heatmap
                </div>
                <button
                  onClick={() => setHeatmapOpen(false)}
                  className="font-display text-[10px] uppercase tracking-wider text-muted-foreground hover:text-[var(--honey)]"
                >
                  close ✕
                </button>
              </div>
              <RunHeatmap />
              <div className="mt-2 font-display text-[9px] uppercase tracking-wider text-muted-foreground">
                Filtered baseline · {product.id}
              </div>
            </div>
          )}
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent className="w-52">
        <ContextMenuItem onSelect={() => openRunbook(product.id)}>
          <BookOpen className="mr-2 h-3.5 w-3.5" /> Open runbook
        </ContextMenuItem>
        <ContextMenuItem onSelect={() => setDispatchPreviewOpen(true)}>
          <Zap className="mr-2 h-3.5 w-3.5" /> Blast radius
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem onSelect={() => document.getElementById("tokens")?.scrollIntoView({ behavior: "smooth" })}>
          Tokens
        </ContextMenuItem>
        <ContextMenuItem onSelect={() => document.getElementById("incidents")?.scrollIntoView({ behavior: "smooth" })}>
          Incidents
        </ContextMenuItem>
        <ContextMenuItem onSelect={() => setHeatmapOpen(true)}>
          Heatmap
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}

function PctChip({ label, value, tone }: { label: string; value: number; tone: Status }) {
  return (
    <span className={`rounded border border-border bg-background/60 px-1.5 py-0.5 ${statusColor[tone]}`}>
      {label} <span className="text-foreground">{value}ms</span>
    </span>
  );
}

function Stat({ label, value, tone }: { label: string; value: string; tone?: Status }) {
  return (
    <div className="rounded border border-border/60 bg-background/40 px-2 py-1.5">
      <div className="text-[8px] uppercase tracking-wider text-muted-foreground">{label}</div>
      <div className={`mt-0.5 text-xs font-bold ${tone ? statusColor[tone] : "text-foreground"}`}>{value}</div>
    </div>
  );
}

function RunsFeed({ syncVersion }: { syncVersion: number }) {
  const search = Route.useSearch();
  const navigate = Route.useNavigate();
  const activeRun = useMemo(
    () => (search.run ? RUNS.find((r) => r.id === search.run) ?? null : null),
    [search.run, syncVersion],
  );
  // Strip invalid ?run= from the URL so a bad deep-link doesn't leave a phantom
  // param that confuses subsequent navigation.
  useEffect(() => {
    if (search.run && !activeRun) {
      navigate({
        search: (p: IndexSearch) => ({ ...p, run: undefined }),
        replace: true,
      });
    }
  }, [search.run, activeRun, navigate]);
  const setActiveRun = (r: RunEvent | null) => {
    navigate({
      search: (p: IndexSearch) => ({ ...p, run: r?.id }),
      replace: !r,
    });
  };
  const { replayRun } = useHiveCommand();

  const [codeFilter, setCodeFilter] = useState<string>("all");
  const [stageFilter, setStageFilter] = useState<string>("all");
  const [reqIdFilter, setReqIdFilter] = useState<string>("");

  // RUNS is replaced wholesale on every sync, so this must recompute with it.
  const enriched = useMemo(
    () => RUNS.map((r) => ({ run: r, failure: deriveFailure(r) })),
    [syncVersion],
  );
  const codeOptions = useMemo(
    () =>
      Array.from(
        new Set(
          enriched
            .map((e) => (e.failure ? (e.failure.code ?? "failed") : null))
            .filter((c): c is string => !!c),
        ),
      ),
    [enriched],
  );
  const stageOptions = useMemo(
    () =>
      Array.from(
        new Set(enriched.map((e) => e.failure?.stage).filter((s): s is string => !!s)),
      ),
    [enriched],
  );

  const failureFiltersActive =
    codeFilter !== "all" || stageFilter !== "all" || reqIdFilter.trim() !== "";

  const visible = enriched.filter(({ failure }) => {
    if (!failureFiltersActive) return true;
    if (!failure) return false;
    if (codeFilter !== "all" && (failure.code ?? "failed") !== codeFilter) return false;
    if (stageFilter !== "all" && (failure.stage ?? "") !== stageFilter) return false;
    if (
      reqIdFilter.trim() !== "" &&
      !failure.requestId.toLowerCase().includes(reqIdFilter.trim().toLowerCase())
    )
      return false;
    return true;
  });

  const clearFilters = () => {
    setCodeFilter("all");
    setStageFilter("all");
    setReqIdFilter("");
  };

  return (
    <div className="rounded-xl border border-border bg-card/50 backdrop-blur">
      <div className="flex items-center justify-between border-b border-border/60 px-5 py-3">
        <div className="flex items-center gap-2">
          <Terminal className="h-4 w-4 text-[var(--honey)]" />
          <h2 className="font-display text-sm font-bold uppercase tracking-[0.2em]">Live Runs</h2>
        </div>
        <span className="font-display text-[10px] uppercase tracking-wider text-[var(--honey)]">
          ● streaming
        </span>
      </div>
      <div className="border-b border-border/60 bg-background/30 px-4 py-2.5">
        <div className="mb-1.5 flex items-center justify-between">
          <div className="flex items-center gap-1.5 font-display text-[9px] uppercase tracking-[0.2em] text-muted-foreground">
            <Filter className="h-3 w-3 text-[var(--honey)]" /> Failure filters
          </div>
          {failureFiltersActive && (
            <button
              onClick={clearFilters}
              className="inline-flex items-center gap-1 font-display text-[9px] uppercase tracking-wider text-[var(--honey)] hover:brightness-110"
            >
              clear <X className="h-2.5 w-2.5" />
            </button>
          )}
        </div>
        <div className="grid grid-cols-1 gap-1.5 sm:grid-cols-3">
          <select
            aria-label="Filter by error code"
            value={codeFilter}
            onChange={(e) => setCodeFilter(e.target.value)}
            className="w-full rounded border border-border bg-background/60 px-2 py-1 font-display text-[10px] text-foreground transition focus:border-[var(--honey)]/60 focus:outline-none"
          >
            <option value="all">All error codes</option>
            {codeOptions.map((c) => (
              <option key={c} value={c}>{c}</option>
            ))}
          </select>
          <select
            aria-label="Filter by stage"
            value={stageFilter}
            onChange={(e) => setStageFilter(e.target.value)}
            className="w-full rounded border border-border bg-background/60 px-2 py-1 font-display text-[10px] text-foreground transition focus:border-[var(--honey)]/60 focus:outline-none"
          >
            <option value="all">All stages</option>
            {stageOptions.map((s) => (
              <option key={s} value={s}>{s}</option>
            ))}
          </select>
          <input
            aria-label="Filter by request ID"
            value={reqIdFilter}
            onChange={(e) => setReqIdFilter(e.target.value)}
            placeholder="req_id contains…"
            className="w-full rounded border border-border bg-background/60 px-2 py-1 font-display text-[10px] text-foreground placeholder:text-muted-foreground/60 focus:border-[var(--honey)]/60 focus:outline-none"
          />
        </div>
      </div>
      <ul className="max-h-[560px] divide-y divide-border/40 overflow-y-auto overscroll-contain">
        {visible.length === 0 && (
          <li className="px-4 py-8">
            <EmptyHex title="no runs match these filters" hint="clear filters or widen your search" />
          </li>
        )}
        {visible.map(({ run: r, failure }, i) => (
          <li
            key={r.id}
            className="animate-float-up group relative"
            style={{ animationDelay: `${i * 40}ms` }}
          >
            <div
              role="button"
              tabIndex={0}
              onClick={() => setActiveRun(r)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  setActiveRun(r);
                }
              }}
              className="flex w-full cursor-pointer items-center gap-3 px-4 py-2.5 text-left transition hover:bg-muted/30 focus:bg-muted/40 focus:outline-none"
            >
              <span
                className={`h-1.5 w-1.5 rounded-full ${
                  r.status === "success"
                    ? "bg-[var(--ok)] shadow-[0_0_8px_var(--ok)]"
                    : r.status === "running"
                    ? "bg-[var(--honey)] shadow-[0_0_8px_var(--honey)] animate-pulse-dot"
                    : "bg-[var(--crit)] shadow-[0_0_8px_var(--crit)]"
                }`}
              />
              <div className="min-w-0 flex-1">
                <div className="flex items-baseline justify-between gap-2">
                  <span className="truncate font-display text-xs font-semibold">{r.product}</span>
                  <span className="font-display text-[10px] text-muted-foreground">{r.ts}</span>
                </div>
                <div className="flex items-center justify-between gap-2">
                  <code className="truncate font-display text-[10px] text-[var(--honey)]">{r.capability}</code>
                  <span className="font-display text-[10px] text-muted-foreground">
                    {r.status === "running" ? "…" : `${r.durationMs}ms`}
                  </span>
                </div>
                {failure && (
                  <div className="mt-1 flex flex-wrap items-center gap-1">
                    <span className="rounded border border-[var(--crit)]/40 bg-[var(--crit)]/10 px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider text-[var(--crit)]">
                      {failure.code ?? "failed"}
                    </span>
                    <span className="truncate font-display text-[9px] text-muted-foreground">
                      {failure.stage ? `${failure.stage} · ` : ""}
                      {failure.requestId}
                    </span>
                  </div>
                )}
              </div>
              {r.status !== "running" && (
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    replayRun(r);
                  }}
                  className="opacity-0 transition group-hover:opacity-100 inline-flex shrink-0 items-center gap-1 rounded border border-border bg-card/80 px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider text-muted-foreground hover:border-[var(--honey)]/50 hover:text-[var(--honey)]"
                  title="Replay run"
                  aria-label="Replay run"
                >
                  <Repeat className="h-2.5 w-2.5" /> replay
                </button>
              )}
            </div>
          </li>
        ))}

      </ul>

      <div className="border-t border-border/60 px-4 py-2 text-center">
        <button className="font-display text-[10px] uppercase tracking-wider text-muted-foreground transition hover:text-[var(--honey)]">
          {visible.length} of {RUNS.length} runs →
        </button>
      </div>
      <RunDetailDrawer run={activeRun} onClose={() => setActiveRun(null)} />
    </div>
  );
}

function CapabilityGrid() {
  const allCaps = Array.from(new Set(PRODUCTS.flatMap((p) => p.capabilities)));
  return (
    <section className="mt-8 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="mb-5 flex items-center justify-between">
        <div>
          <h2 className="font-display text-sm font-bold uppercase tracking-[0.2em]">Capability Matrix</h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Advertised actions across the suite. Dispatch only through this contract — no hidden API drift.
          </p>
        </div>
        <div className="hidden items-center gap-4 font-display text-[10px] uppercase tracking-wider text-muted-foreground md:flex">
          <span className="flex items-center gap-1.5"><span className="h-2 w-2 rounded-sm bg-[var(--honey)]" /> advertised</span>
          <span className="flex items-center gap-1.5"><span className="h-2 w-2 rounded-sm border border-border" /> not offered</span>
        </div>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full min-w-[700px] border-collapse">
          <thead>
            <tr>
              <th className="sticky left-0 z-10 bg-card/80 px-2 py-2 text-left font-display text-[10px] uppercase tracking-wider text-muted-foreground">
                Product
              </th>
              {allCaps.map((c) => (
                <th key={c} className="px-1 py-2 text-center font-display text-[10px] uppercase tracking-wider text-muted-foreground">
                  <div className="inline-block -rotate-45 origin-bottom-left whitespace-nowrap">{c}</div>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {PRODUCTS.map((p) => (
              <tr key={p.id} className="border-t border-border/40 transition hover:bg-muted/20">
                <td className="sticky left-0 z-10 bg-card/80 px-2 py-2 font-display text-xs font-semibold">
                  <span className="flex items-center gap-2">
                    <span className={`h-1.5 w-1.5 rounded-full ${statusDot[p.status]}`} />
                    {p.name}
                  </span>
                </td>
                {allCaps.map((c) => {
                  const has = p.capabilities.includes(c);
                  return (
                    <td key={c} className="px-1 py-2 text-center">
                      <span
                        className={`inline-block h-3 w-3 rounded-sm ${
                          has ? "bg-[var(--honey)] shadow-[0_0_8px_var(--honey)]" : "border border-border bg-transparent"
                        }`}
                      />
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function Footer() {
  return (
    <footer className="mt-12 flex flex-wrap items-center justify-between gap-4 border-t border-border/60 pt-6 font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
      <div className="flex items-center gap-2">
        <GitBranch className="h-3 w-3" />
        main · hivecore v0.1.0
      </div>
      <div>polling /health · /capabilities · /runs · /startup/checks</div>
      <div>© PatchHive · the hive remembers</div>
    </footer>
  );
}
