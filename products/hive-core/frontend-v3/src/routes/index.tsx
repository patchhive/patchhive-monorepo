import { createFileRoute } from "@tanstack/react-router";
import { useMemo } from "react";
import { Hexagon } from "lucide-react";

import { PRODUCTS, isWriteCapable, type ProductStatus } from "@/lib/hive-data";
import { BUDGETS, SUITE_EVENTS, capabilityDrift } from "@/lib/suite-state";
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
  const stats = useMemo(() => {
    const byStatus = (status: ProductStatus) =>
      PRODUCTS.filter((product) => product.observed.status === status).length;
    return {
      total: PRODUCTS.length,
      online: byStatus("online"),
      degraded: byStatus("degraded"),
      unobserved: PRODUCTS.filter((p) => p.observed.observedAt === null).length,
      writeCapable: PRODUCTS.filter(isWriteCapable).length,
      drift: PRODUCTS.map(capabilityDrift).filter(Boolean).length,
      events: SUITE_EVENTS.length,
    };
  }, []);

  return (
    <main className="relative min-h-screen pb-20">
      <div className="relative mx-auto max-w-[1400px] px-6 py-8 lg:px-10">
        <TopBar />
        <Hero unobserved={stats.unobserved} total={stats.total} />

        <div className="mt-6 grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
          <Metric label="Products" value={stats.total} detail={`${stats.writeCapable} write-capable`} />
          <Metric
            label="Observed"
            value={stats.total - stats.unobserved}
            tone={stats.unobserved === stats.total ? "warn" : "ok"}
            detail={`${stats.unobserved} never polled`}
          />
          <Metric
            label="Drifted"
            value={stats.drift}
            tone={stats.drift > 0 ? "warn" : "ok"}
            detail="manifest vs runtime"
          />
          <Metric
            label="PR capacity"
            value={`${BUDGETS.suiteRemaining}/${BUDGETS.suiteLimit}`}
            tone={BUDGETS.suiteRemaining === 0 ? "crit" : "honey"}
            detail="suite ceiling"
          />
          <Metric
            label="Suite events"
            value={stats.events}
            detail="append-only ledger"
          />
        </div>

        <ProductRegistry />
        <LiveRuns />
        <SuiteTimeline />
        <ContractDrift />
        <CapabilityMatrix />
        <BudgetPanel />
        <ApprovalsQueue />
        <PolicyPanel />
        <MandatesPanel />
        <BlastRadius />
        <RunHeatmap />
        <ProcedureHistory />
        <AskHive />
        <ChangeLog />
        <Footer />
      </div>
    </main>
  );
}

function TopBar() {
  return (
    <header className="flex flex-wrap items-center justify-between gap-4 border-b border-border/60 pb-4">
      <div className="flex items-center gap-3">
        <div className="relative grid h-10 w-10 place-items-center">
          <Hexagon className="absolute inset-0 h-full w-full text-[var(--honey)]" strokeWidth={1.4} />
          <Hexagon className="h-5 w-5 fill-[var(--honey)] text-[var(--honey)]" />
        </div>
        <div className="leading-tight">
          <div className="font-display text-lg font-bold tracking-tight">
            <span className="text-[var(--honey)]">HIVE</span>
            <span className="text-foreground">CORE</span>
          </div>
          <div className="font-display text-[10px] uppercase tracking-[0.25em] text-muted-foreground">
            patchhive control plane
          </div>
        </div>
      </div>
      <nav className="hidden flex-1 items-center justify-center gap-1 lg:flex">
        {NAV.map((item) => (
          <a
            key={item.id}
            href={`#${item.id}`}
            className="rounded-md px-2.5 py-1.5 font-display text-[10px] uppercase tracking-wider text-muted-foreground transition hover:text-[var(--honey)]"
          >
            {item.label}
          </a>
        ))}
      </nav>
      <div className="flex items-center gap-2">
        <span className="hidden font-mono text-[10px] text-muted-foreground sm:inline">{API}</span>
        <PauseControl />
      </div>
    </header>
  );
}

function Hero({ unobserved, total }: { unobserved: number; total: number }) {
  const wired = unobserved < total;
  return (
    <section className="mt-10 grid gap-8 lg:grid-cols-[1.5fr_1fr]">
      <div>
        <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-[var(--honey)]/30 bg-[color-mix(in_oklab,var(--honey)_8%,transparent)] px-3 py-1 font-display text-[10px] uppercase tracking-[0.3em] text-[var(--honey)]">
          <span className="relative flex h-2 w-2">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-[var(--honey)] opacity-60" />
            <span className="relative inline-flex h-2 w-2 rounded-full bg-[var(--honey)]" />
          </span>
          {wired ? `${total - unobserved}/${total} products observed` : "Registry loaded · not yet polling"}
        </div>
        <h1 className="font-display text-5xl font-bold leading-[1.05] tracking-tight md:text-6xl">
          One hive.
          <br />
          <span className="text-[var(--honey)]">Every product.</span>
          <br />
          Zero blind spots.
        </h1>
        <p className="mt-6 max-w-xl text-base text-muted-foreground">
          HiveCore owns suite authority: repository policy, outbound PR capacity, approvals, and
          the record of every decision. Products are its capabilities.
        </p>
      </div>
      <Honeycomb />
    </section>
  );
}

function Honeycomb() {
  const layout: string[][] = [
    ["repo-reaper", "signal-hive", "trust-gate"],
    ["repo-memory", "hive-core", "review-bee"],
    ["merge-keeper", "flake-sting", "dep-triage"],
    ["vuln-triage", "refactor-scout", "release-sentry"],
  ];
  const byKey = Object.fromEntries(PRODUCTS.map((product) => [product.key, product]));

  return (
    <div className="relative overflow-hidden rounded-2xl border border-border bg-card/40 backdrop-blur-sm">
      <div className="pointer-events-none absolute -right-24 -top-24 h-64 w-64 rounded-full bg-[var(--honey)]/10 blur-3xl" />
      <div className="relative flex items-center justify-between border-b border-border/60 bg-[var(--honey)]/[0.04] px-5 py-3">
        <span className="font-display text-[10px] font-bold uppercase tracking-[0.25em] text-[var(--honey)]/85">
          Suite mesh
        </span>
        <span className="font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
          {PRODUCTS.length} registered
        </span>
      </div>
      <div className="relative flex flex-col items-center px-4 py-8">
        {layout.map((row, rowIndex) => (
          <div
            key={rowIndex}
            className={`flex gap-1.5 ${rowIndex < layout.length - 1 ? "-mb-4" : ""}`}
          >
            {row.map((key) => {
              const product = byKey[key];
              if (!product) return null;
              const queen = product.key === "hive-core";
              const write = isWriteCapable(product);
              return (
                <div
                  key={product.key}
                  title={`${product.name} — ${product.role}`}
                  className={`relative flex items-center justify-center ${queen ? "z-10 -mt-2 h-[104px] w-24" : "h-[88px] w-20"}`}
                  style={{
                    clipPath: HEX,
                    background: queen ? "var(--honey)" : "color-mix(in oklab, var(--card) 85%, transparent)",
                  }}
                >
                  {queen && (
                    <div className="absolute inset-[1.5px] bg-background" style={{ clipPath: HEX }} />
                  )}
                  <div className="relative flex flex-col items-center">
                    <span
                      className={`font-display font-black leading-none ${queen ? "text-xl text-[var(--honey)]" : "text-lg text-foreground/75"}`}
                    >
                      {product.code}
                    </span>
                    <span
                      className="mt-1 font-mono text-[9px]"
                      style={{ color: write ? "var(--warn)" : "var(--ok)" }}
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
    </div>
  );
}

function Footer() {
  return (
    <footer className="mt-12 border-t border-border/60 pt-6 text-center">
      <div className="font-display text-xs font-bold uppercase tracking-wider text-foreground">
        HiveCore by PatchHive
      </div>
      <div className="mt-1 text-[11px] text-muted-foreground">
        Suite control plane · Autonomous maintenance suite
      </div>
    </footer>
  );
}
