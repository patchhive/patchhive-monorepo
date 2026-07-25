import { createFileRoute } from "@tanstack/react-router";
import { useMemo } from "react";
import { Hexagon, Radio, Terminal, Zap } from "lucide-react";

import { isWriteCapable, type Product, type ProductStatus } from "@/lib/hive-data";
import { BUDGETS, capabilityDrift } from "@/lib/suite-state";
import { useSuiteData } from "@/lib/use-suite";
import { API } from "@/config";
import { HEX, Metric } from "@/components/deck-ui";
import { PauseControl } from "@/components/pause-control";
import { ProductRegistry } from "@/components/product-registry";
import { LiveRuns } from "@/components/live-runs";
import { SuiteTimeline } from "@/components/suite-timeline";
import { ContractDrift } from "@/components/contract-drift";
import { CapabilityMatrix } from "@/components/capability-matrix";
import { BudgetPanel } from "@/components/budget-panel";
import { ApprovalsQueue } from "@/components/approvals-queue";
import { PolicyPanel } from "@/components/policy-panel";
import { MandatesPanel } from "@/components/mandates-panel";
import { BlastRadius } from "@/components/blast-radius";
import { RunHeatmap } from "@/components/run-heatmap";
import { ProcedureHistory } from "@/components/procedure-history";
import { AskHive } from "@/components/ask-hive";
import { ChangeLog } from "@/components/change-log";

type IndexSearch = { section?: string };

const SECTIONS = [
  "registry",
  "runs",
  "timeline",
  "drift",
  "capabilities",
  "budgets",
  "approvals",
  "policy",
  "mandates",
  "blast-radius",
  "heatmap",
  "procedures",
  "ask",
  "changelog",
] as const;

const SECTION_SET = new Set<string>(SECTIONS);

// Runs on every navigation. Anything that fails validation is dropped so a stale or
// hand-edited deep link degrades to the default deck instead of breaking consumers.
function sanitizeSearch(raw: Record<string, unknown>): IndexSearch {
  const section = typeof raw.section === "string" ? raw.section : undefined;
  return section && SECTION_SET.has(section) ? { section } : {};
}

export const Route = createFileRoute("/")({
  validateSearch: sanitizeSearch,
  component: Deck,
});

const NAV: { id: string; label: string }[] = [
  { id: "registry", label: "Registry" },
  { id: "runs", label: "Runs" },
  { id: "timeline", label: "Timeline" },
  { id: "drift", label: "Drift" },
  { id: "budgets", label: "Capacity" },
  { id: "approvals", label: "Approvals" },
  { id: "policy", label: "Policy" },
  { id: "mandates", label: "Mandates" },
];

function Deck() {
  const { products, events, runs, live, error } = useSuiteData();
  const stats = useMemo(() => {
    const byStatus = (status: ProductStatus) =>
      products.filter((product) => product.observed.status === status).length;
    return {
      total: products.length,
      online: byStatus("online"),
      degraded: byStatus("degraded"),
      unobserved: products.filter((p) => p.observed.observedAt === null).length,
      writeCapable: products.filter(isWriteCapable).length,
      drift: products.map(capabilityDrift).filter(Boolean).length,
      events: events.length,
      capabilities: products.reduce((sum, p) => sum + p.declared.length, 0),
    };
  }, [products, events]);

  return (
    <main className="hex-grid relative min-h-screen pb-20">
      <div aria-hidden="true" className="pointer-events-none absolute inset-0 scanline opacity-40" />
      <div className="relative mx-auto max-w-[1400px] px-6 py-8 lg:px-10">
        <TopBar live={live} />
        {error && <ConnectionNotice error={error} />}
        <Hero
          products={products}
          unobserved={stats.unobserved}
          total={stats.total}
          online={stats.online}
          capabilities={stats.capabilities}
        />

        <div className="mt-8 grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
          <Metric
            label="Products"
            value={stats.total}
            detail={`${stats.writeCapable} write-capable`}
            delay={0}
          />
          <Metric
            label="Online"
            value={stats.online}
            tone={stats.online > 0 ? "ok" : "neutral"}
            detail={`${stats.degraded} degraded`}
            delay={60}
          />
          <Metric
            label="Drifted"
            value={stats.drift}
            tone={stats.drift > 0 ? "warn" : "ok"}
            detail="manifest vs runtime"
            delay={120}
          />
          <Metric
            label="PR capacity"
            value={`${BUDGETS.suiteRemaining}/${BUDGETS.suiteLimit}`}
            tone={BUDGETS.suiteRemaining === 0 ? "crit" : "honey"}
            detail="suite ceiling"
            delay={180}
          />
          <Metric
            label="Suite events"
            value={stats.events}
            tone="honey"
            detail="append-only ledger"
            delay={240}
          />
        </div>

        <ProductRegistry products={products} />
        <LiveRuns runs={runs} />
        <SuiteTimeline events={events} />
        <ContractDrift products={products} events={events} />
        <CapabilityMatrix products={products} />
        <BudgetPanel />
        <ApprovalsQueue />
        <PolicyPanel />
        <MandatesPanel />
        <BlastRadius products={products} />
        <RunHeatmap products={products} events={events} />
        <ProcedureHistory />
        <AskHive />
        <ChangeLog events={events} />
        <Footer />
      </div>
    </main>
  );
}

function TopBar({ live }: { live: boolean }) {
  return (
    <header className="flex flex-wrap items-center justify-between gap-4 border-b border-border/60 pb-4">
      <div className="flex items-center gap-3">
        <div className="relative grid h-10 w-10 place-items-center">
          <Hexagon
            className="absolute inset-0 h-full w-full text-[var(--honey)]"
            strokeWidth={1.4}
          />
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
      <nav className="hidden flex-1 items-center justify-center gap-1 lg:flex">
        {NAV.map((item) => (
          <a
            key={item.id}
            href={`#${item.id}`}
            className="rounded-md px-2.5 py-1.5 font-display text-[10px] uppercase tracking-wider text-muted-foreground transition hover:bg-[color-mix(in_oklab,var(--honey)_12%,transparent)] hover:text-[var(--honey)]"
          >
            {item.label}
          </a>
        ))}
      </nav>
      <div className="flex items-center gap-2">
        <span
          className="hidden items-center gap-1.5 rounded-md border border-border bg-card/60 px-2 py-1.5 font-mono text-[10px] text-muted-foreground backdrop-blur sm:inline-flex"
          title={
            live
              ? "Control plane reachable"
              : "Control plane unreachable — showing registry defaults"
          }
        >
          <span
            className="inline-block h-1.5 w-1.5 animate-pulse-dot rounded-full"
            style={{
              background: live ? "var(--ok)" : "var(--crit)",
              boxShadow: `0 0 8px ${live ? "var(--ok)" : "var(--crit)"}`,
            }}
          />
          {API}
        </span>
        <PauseControl />
      </div>
    </header>
  );
}

function Hero({
  products,
  unobserved,
  total,
  online,
  capabilities,
}: {
  products: Product[];
  unobserved: number;
  total: number;
  online: number;
  capabilities: number;
}) {
  const wired = unobserved < total;
  return (
    <section className="mt-10 grid gap-8 lg:grid-cols-[1.4fr_1fr]">
      <div className="animate-float-up">
        <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-[var(--honey)]/30 bg-[color-mix(in_oklab,var(--honey)_8%,transparent)] px-3 py-1 font-display text-[10px] uppercase tracking-[0.3em] text-[var(--honey)]">
          <span className="relative flex h-2 w-2">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-[var(--honey)] opacity-60" />
            <span className="relative inline-flex h-2 w-2 rounded-full bg-[var(--honey)]" />
          </span>
          {wired
            ? `${online}/${total} products online · ${capabilities} capabilities`
            : "Registry loaded · control plane unreachable"}
        </div>
        <h1 className="font-display text-5xl font-bold leading-[1.05] tracking-tight md:text-7xl">
          One hive.
          <br />
          <span className="text-gradient-honey">Every product.</span>
          <br />
          Zero blind spots.
        </h1>
        <p className="mt-6 max-w-xl text-base text-muted-foreground md:text-lg">
          HiveCore owns suite authority: repository{" "}
          <code className="rounded bg-muted px-1.5 py-0.5 font-display text-[0.85em] text-[var(--honey)]">
            policy
          </code>
          , outbound PR{" "}
          <code className="rounded bg-muted px-1.5 py-0.5 font-display text-[0.85em] text-[var(--honey)]">
            capacity
          </code>
          ,{" "}
          <code className="rounded bg-muted px-1.5 py-0.5 font-display text-[0.85em] text-[var(--honey)]">
            approvals
          </code>
          , and the record of every decision. Products are its capabilities.
        </p>
        <div className="mt-8 flex flex-wrap items-center gap-3">
          <a
            href="#registry"
            className="glow-honey group inline-flex items-center gap-2 rounded-md bg-[var(--honey)] px-5 py-3 font-display text-sm font-bold uppercase tracking-wider text-primary-foreground transition hover:brightness-110"
          >
            <Zap className="h-4 w-4" /> Open registry
          </a>
          <a
            href="#timeline"
            className="inline-flex items-center gap-2 rounded-md border border-border bg-card/60 px-5 py-3 font-display text-sm font-medium uppercase tracking-wider text-foreground backdrop-blur transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)]"
          >
            <Terminal className="h-4 w-4" /> Suite timeline
          </a>
        </div>
      </div>
      <Honeycomb products={products} />
    </section>
  );
}

function ConnectionNotice({ error }: { error: string }) {
  return (
    <div className="mt-4 rounded-lg border border-[var(--crit)]/40 bg-[var(--crit)]/[0.06] px-4 py-2.5 backdrop-blur">
      <div className="font-display text-[10px] font-bold uppercase tracking-wider text-[var(--crit)]">
        Control plane unreachable
      </div>
      <p className="mt-0.5 text-xs text-muted-foreground">
        {error} Showing the registry defaults compiled into the deck — status, capabilities, and
        events are not live.
      </p>
    </div>
  );
}

const MESH_LAYOUT: string[][] = [
  ["repo-reaper", "signal-hive", "trust-gate"],
  ["repo-memory", "hive-core", "review-bee"],
  ["merge-keeper", "flake-sting", "dep-triage"],
  ["vuln-triage", "refactor-scout", "release-sentry"],
];

function statusColor(product: Product): string {
  switch (product.observed.status) {
    case "online":
      return "var(--ok)";
    case "degraded":
      return "var(--warn)";
    case "offline":
      return "var(--crit)";
    case "unconfigured":
      return "var(--honey)";
    default:
      return "var(--muted-foreground)";
  }
}

function Honeycomb({ products }: { products: Product[] }) {
  const byKey = Object.fromEntries(products.map((product) => [product.key, product]));
  const online = products.filter((p) => p.observed.status === "online").length;

  return (
    <div className="relative overflow-hidden rounded-2xl border border-border bg-card/40 shadow-2xl backdrop-blur-sm">
      {/* ambient honey + amber glow bleeding off opposite corners */}
      <div className="pointer-events-none absolute -right-24 -top-24 h-64 w-64 rounded-full bg-[var(--honey)]/15 blur-3xl" />
      <div className="pointer-events-none absolute -bottom-24 -left-24 h-64 w-64 rounded-full bg-[var(--amber-deep)]/20 blur-3xl" />
      <div className="pointer-events-none absolute inset-0 scanline opacity-20" />

      <div className="relative flex items-center justify-between border-b border-border/60 bg-[var(--honey)]/[0.04] px-5 py-3">
        <div className="flex items-center gap-2">
          <Radio className="h-3 w-3 text-[var(--honey)]" />
          <span className="font-display text-[10px] font-bold uppercase tracking-[0.25em] text-[var(--honey)]/85">
            Suite mesh
          </span>
        </div>
        <span className="font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
          {online}/{products.length} online
        </span>
      </div>

      <div className="relative flex flex-col items-center px-4 py-8 sm:py-10">
        {MESH_LAYOUT.map((row, rowIndex) => (
          <div
            key={rowIndex}
            className={`flex gap-1.5 ${rowIndex < MESH_LAYOUT.length - 1 ? "-mb-4" : ""}`}
          >
            {row.map((key, columnIndex) => {
              const product = byKey[key];
              if (!product) return null;
              const queen = product.key === "hive-core";
              const tone = statusColor(product);
              const write = isWriteCapable(product);
              const index = rowIndex * 3 + columnIndex;

              if (queen) {
                return (
                  <div
                    key={product.key}
                    title={`${product.name} — ${product.role}`}
                    style={{
                      clipPath: HEX,
                      background: "var(--honey)",
                      animationDelay: `${index * 50}ms`,
                    }}
                    className="animate-float-up relative z-10 -mt-2 flex h-[104px] w-24 items-center justify-center shadow-[0_0_30px_color-mix(in_oklab,var(--honey)_30%,transparent)]"
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
                        {product.code}
                      </span>
                      <span className="mt-0.5 font-mono text-[10px] font-medium tracking-tight text-[var(--honey)]/80">
                        control
                      </span>
                    </div>
                    <span className="absolute bottom-2 h-1 w-1 animate-ping rounded-full bg-[var(--honey)]" />
                  </div>
                );
              }

              return (
                <div
                  key={product.key}
                  title={`${product.name} — ${product.role} (${product.observed.status})`}
                  style={{ clipPath: HEX, animationDelay: `${index * 50}ms` }}
                  className="animate-float-up relative flex h-[88px] w-20 items-center justify-center bg-card/80"
                >
                  <span
                    className="absolute top-2 h-1.5 w-1.5 rounded-full"
                    style={{ background: tone, boxShadow: `0 0 8px ${tone}` }}
                  />
                  <div className="flex flex-col items-center">
                    <span className="font-display text-lg font-bold leading-none text-foreground/75">
                      {product.code}
                    </span>
                    <span
                      className="mt-1 font-mono text-[9px]"
                      style={{
                        color: `color-mix(in oklab, ${write ? "var(--warn)" : "var(--ok)"} 70%, transparent)`,
                      }}
                    >
                      {write ? "write" : "read"}
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        ))}
      </div>

      <div className="relative flex items-center justify-between border-t border-border/60 bg-background/40 px-5 py-3">
        <div className="flex items-center gap-1.5">
          <span className="h-1.5 w-1.5 rotate-45 border border-[var(--honey)]/50 bg-[var(--honey)]/20" />
          <span className="font-display text-[9px] font-bold uppercase tracking-[0.25em] text-[var(--honey)]/80">
            Mesh health
          </span>
        </div>
        <span className="font-mono text-[10px] text-[var(--honey)]/90">
          {products.filter(isWriteCapable).length} write-capable
        </span>
      </div>
    </div>
  );
}

function Footer() {
  return (
    <footer className="mt-12 border-t border-border/60 pt-6 text-center">
      <div className="font-display text-xs font-bold uppercase tracking-wider text-foreground">
        <span className="text-gradient-honey">HiveCore</span> by PatchHive
      </div>
      <div className="mt-1 text-[11px] text-muted-foreground">
        Suite control plane · Autonomous maintenance suite
      </div>
    </footer>
  );
}
