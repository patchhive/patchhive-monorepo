import { useCallback, useEffect, useMemo, useState } from "react";
import { Archive, Eye, Loader2, Pause, Play, Plus, Radar, RefreshCw } from "lucide-react";
import { toast } from "sonner";

import {
  changeMandateState,
  createMandate,
  fetchConductorTicks,
  fetchMandates,
  runConductorTick,
  type ConductorDecision,
  type ConductorTickRecord,
  type MandateAutonomy,
  type MandateConfig,
  type MandateRecord,
} from "@/lib/mandates";

const EMPTY_FORM: MandateConfig = {
  name: "",
  objective: "",
  scope: {
    search_query: "archived:false",
    topics: [],
    languages: ["rust"],
    min_stars: 25,
    max_repositories: 8,
    issues_per_repository: 30,
    stale_days: 45,
  },
  requested_autonomy: "propose",
  limits: {
    pr_budget: 3,
    cost_budget_cents_per_day: 500,
    per_owner_open_prs: 1,
    cooldown_after_close_days: 14,
  },
};

const tone: Record<string, string> = {
  active: "border-[var(--ok)]/40 text-[var(--ok)]",
  paused: "border-[var(--warn)]/40 text-[var(--warn)]",
  archived: "border-border text-muted-foreground",
  unknown: "border-[var(--crit)]/40 text-[var(--crit)]",
};

const csv = (value: string) => value.split(",").map((part) => part.trim()).filter(Boolean);

export function MandateControl({ syncVersion = 0 }: { syncVersion?: number }) {
  const [mandates, setMandates] = useState<MandateRecord[]>([]);
  const [ticks, setTicks] = useState<ConductorTickRecord[]>([]);
  const [form, setForm] = useState<MandateConfig>(EMPTY_FORM);
  const [topics, setTopics] = useState("");
  const [languages, setLanguages] = useState("rust");
  const [reason, setReason] = useState("Operator changed mandate state from HiveCore.");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");

  const refresh = useCallback(async (signal?: AbortSignal) => {
    try {
      const [nextMandates, nextTicks] = await Promise.all([
        fetchMandates(signal),
        fetchConductorTicks(signal),
      ]);
      setMandates(nextMandates);
      setTicks(nextTicks);
      setError("");
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      setError(cause instanceof Error ? cause.message : "Could not read conductor state.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    void refresh(controller.signal);
    return () => controller.abort();
  }, [refresh, syncVersion]);

  const counts = useMemo(() => {
    const result = { active: 0, paused: 0, unknown: 0 };
    for (const mandate of mandates) {
      if (mandate.lifecycle.state === "active") result.active += 1;
      if (mandate.lifecycle.state === "paused") result.paused += 1;
      if (mandate.lifecycle.state === "unknown") result.unknown += 1;
    }
    return result;
  }, [mandates]);

  async function submitMandate() {
    setBusy("create");
    try {
      await createMandate({
        ...form,
        scope: { ...form.scope, topics: csv(topics), languages: csv(languages) },
      });
      toast.success("Mandate created", { description: "The next admitted conductor tick can dispatch bounded discovery." });
      setForm(EMPTY_FORM);
      setTopics("");
      setLanguages("rust");
      await refresh();
    } catch (cause) {
      toast.error("Could not create mandate", { description: message(cause) });
    } finally {
      setBusy("");
    }
  }

  async function transition(mandate: MandateRecord, action: "activate" | "pause" | "archive") {
    setBusy(mandate.id);
    try {
      await changeMandateState(mandate.id, action, reason.trim());
      toast.success(`Mandate ${action === "activate" ? "activated" : `${action}d`}`);
      await refresh();
    } catch (cause) {
      toast.error(`Could not ${action} mandate`, { description: message(cause) });
    } finally {
      setBusy("");
    }
  }

  async function tick() {
    setBusy("tick");
    try {
      const outcome = await runConductorTick();
      if (outcome.outcome === "busy") {
        toast.warning("Conductor already running", { description: `${outcome.active_tick_id} owns the lease until ${new Date(outcome.lease_until).toLocaleString()}.` });
      } else {
        const decisions = outcome.tick.lifecycle.state === "completed" ? outcome.tick.lifecycle.decisions.length : 0;
        toast.success("Proposal tick settled", { description: `${decisions} mandate decisions recorded; zero actions dispatched.` });
      }
      await refresh();
    } catch (cause) {
      toast.error("Conductor tick failed", { description: message(cause) });
    } finally {
      setBusy("");
    }
  }

  return (
    <section id="mandates" className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="mb-5 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]"><Radar className="h-4 w-4 text-[var(--honey)]" /> Mandates & Conductor</h2>
          <p className="mt-1 max-w-3xl text-xs text-muted-foreground">Standing operator intent in SQLite. Every tick is bounded, lease-owned, restartable, resource-gated, and limited to autonomy earned by durable smoke evidence.</p>
        </div>
        <div className="flex flex-wrap gap-2 font-display text-[9px] uppercase tracking-wider">
          <span className="rounded border border-[var(--ok)]/40 px-2 py-1 text-[var(--ok)]">{counts.active} active</span>
          <span className="rounded border border-[var(--warn)]/40 px-2 py-1 text-[var(--warn)]">{counts.paused} paused</span>
          {counts.unknown > 0 && <span className="rounded border border-[var(--crit)]/40 px-2 py-1 text-[var(--crit)]">{counts.unknown} unknown</span>}
          <button disabled={busy !== ""} onClick={() => void tick()} className="inline-flex items-center gap-1 rounded border border-[var(--honey)]/40 px-2 py-1 text-[var(--honey)] hover:bg-[var(--honey)]/10 disabled:opacity-40"><RefreshCw className={`h-3 w-3 ${busy === "tick" ? "animate-spin" : ""}`} /> Run conductor tick</button>
        </div>
      </div>

      {error && <div className="mb-4 rounded-lg border border-[var(--crit)]/40 bg-[var(--crit)]/[0.06] p-3 text-xs text-muted-foreground">{error}</div>}

      <details className="rounded-lg border border-border/70 bg-background/40 p-4">
        <summary className="cursor-pointer font-display text-[10px] font-bold uppercase tracking-[0.18em] text-[var(--honey)]">Create standing intent</summary>
        <div className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <Field label="Name" value={form.name} onChange={(name) => setForm((current) => ({ ...current, name }))} placeholder="rust-cli-maintenance" />
          <Field label="Objective" value={form.objective} onChange={(objective) => setForm((current) => ({ ...current, objective }))} placeholder="Reduce maintenance pressure…" wide />
          <Field label="Search query" value={form.scope.search_query} onChange={(search_query) => setForm((current) => ({ ...current, scope: { ...current.scope, search_query } }))} />
          <Field label="Topics (comma separated)" value={topics} onChange={setTopics} placeholder="cli, developer-tools" />
          <Field label="Languages (comma separated)" value={languages} onChange={setLanguages} placeholder="rust, typescript" />
          <Select label="Requested autonomy" value={form.requested_autonomy} onChange={(requested_autonomy) => setForm((current) => ({ ...current, requested_autonomy }))} />
          <NumberField label="Minimum stars" value={form.scope.min_stars} onChange={(min_stars) => setForm((current) => ({ ...current, scope: { ...current.scope, min_stars } }))} />
          <NumberField label="Max repositories" value={form.scope.max_repositories} onChange={(max_repositories) => setForm((current) => ({ ...current, scope: { ...current.scope, max_repositories } }))} />
          <NumberField label="PR budget" value={form.limits.pr_budget} onChange={(pr_budget) => setForm((current) => ({ ...current, limits: { ...current.limits, pr_budget } }))} />
          <NumberField label="Daily cost budget (cents)" value={form.limits.cost_budget_cents_per_day} onChange={(cost_budget_cents_per_day) => setForm((current) => ({ ...current, limits: { ...current.limits, cost_budget_cents_per_day } }))} />
        </div>
        <div className="mt-4 flex items-center justify-between gap-3">
          <span className="text-[10px] text-muted-foreground">Effective autonomy is recalculated from smoke and reputation evidence at each dispatch.</span>
          <button disabled={busy !== "" || !form.name.trim() || !form.objective.trim()} onClick={() => void submitMandate()} className="inline-flex items-center gap-1.5 rounded border border-[var(--honey)]/40 px-3 py-2 font-display text-[9px] font-bold uppercase tracking-wider text-[var(--honey)] hover:bg-[var(--honey)]/10 disabled:opacity-40"><Plus className="h-3 w-3" /> Create mandate</button>
        </div>
      </details>

      <div className="mt-4">
        <label className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">Lifecycle reason</label>
        <input value={reason} onChange={(event) => setReason(event.target.value)} className="mt-1 w-full rounded border border-border bg-background/60 px-2 py-1.5 text-[11px] outline-none focus:border-[var(--honey)]/50" />
      </div>

      {loading ? (
        <div className="flex items-center gap-2 py-8 text-xs text-muted-foreground"><Loader2 className="h-3.5 w-3.5 animate-spin" /> Reading mandates…</div>
      ) : mandates.length === 0 ? (
        <div className="mt-4 rounded-lg border border-dashed border-border p-8 text-center text-xs text-muted-foreground">No standing intent yet. Create a mandate to let the conductor admit and dispatch discovery.</div>
      ) : (
        <div className="mt-4 grid gap-3 lg:grid-cols-2">
          {mandates.map((mandate) => <MandateCard key={mandate.id} mandate={mandate} busy={busy === mandate.id} reasonReady={Boolean(reason.trim())} onAction={transition} />)}
        </div>
      )}

      <TickHistory ticks={ticks} mandates={mandates} />
    </section>
  );
}

function MandateCard({ mandate, busy, reasonReady, onAction }: { mandate: MandateRecord; busy: boolean; reasonReady: boolean; onAction: (mandate: MandateRecord, action: "activate" | "pause" | "archive") => Promise<void> }) {
  const state = mandate.lifecycle.state;
  const autonomy = mandate.config.requested_autonomy.replaceAll("_", " ");
  return (
    <article className="rounded-lg border border-border/70 bg-background/40 p-4">
      <div className="flex items-start justify-between gap-3">
        <div><div className="font-display text-xs font-bold text-foreground">{mandate.config.name}</div><div className="mt-1 text-[11px] text-muted-foreground">{mandate.config.objective}</div></div>
        <span className={`rounded border px-2 py-1 font-display text-[9px] uppercase tracking-wider ${tone[state] ?? tone.unknown}`}>{state}</span>
      </div>
      <div className="mt-3 grid grid-cols-2 gap-2 text-[10px]">
        <Fact label="Requested" value={autonomy} />
        <Fact label="Effective" value="evaluated per dispatch" />
        <Fact label="Discovery" value={[...mandate.config.scope.topics, ...mandate.config.scope.languages].join(", ") || mandate.config.scope.search_query} />
        <Fact label="Bounds" value={`${mandate.config.scope.max_repositories} repos · ${mandate.config.limits.pr_budget} PR budget`} />
      </div>
      <div className="mt-3 flex flex-wrap gap-2">
        {state === "paused" && <SmallButton disabled={busy} onClick={() => void onAction(mandate, "activate")} icon={Play}>Activate</SmallButton>}
        {state === "active" && <SmallButton disabled={busy || !reasonReady} onClick={() => void onAction(mandate, "pause")} icon={Pause}>Pause</SmallButton>}
        {(state === "active" || state === "paused") && <SmallButton disabled={busy || !reasonReady} onClick={() => void onAction(mandate, "archive")} icon={Archive}>Archive</SmallButton>}
      </div>
      <details className="mt-3"><summary className="cursor-pointer font-display text-[9px] uppercase tracking-wider text-muted-foreground">Exact mandate state</summary><pre className="mt-2 max-h-64 overflow-auto rounded border border-border bg-background/60 p-2 font-mono text-[9px] text-muted-foreground">{JSON.stringify(mandate, null, 2)}</pre></details>
    </article>
  );
}

function TickHistory({ ticks, mandates }: { ticks: ConductorTickRecord[]; mandates: MandateRecord[] }) {
  const names = Object.fromEntries(mandates.map((mandate) => [mandate.id, mandate.config.name]));
  return (
    <div className="mt-6 border-t border-border/70 pt-4">
      <div className="mb-3 flex items-center gap-2 font-display text-[10px] font-bold uppercase tracking-[0.18em]"><Eye className="h-3.5 w-3.5 text-[var(--honey)]" /> Conductor tick history</div>
      {ticks.length === 0 ? <div className="text-xs text-muted-foreground">No conductor tick has run yet.</div> : (
        <div className="space-y-2">{ticks.slice(0, 8).map((tick) => {
          const decisions = tick.lifecycle.state === "completed" ? tick.lifecycle.decisions : [];
          return <details key={tick.id} className="rounded border border-border/70 bg-background/40 p-3"><summary className="cursor-pointer"><span className="font-mono text-[10px] text-foreground">{tick.id}</span><span className="ml-2 font-display text-[9px] uppercase tracking-wider text-muted-foreground">{tick.trigger} · {tick.lifecycle.state} · {decisions.length} decisions</span></summary><div className="mt-2 space-y-1">{decisions.map((decision, index) => <DecisionRow key={`${tick.id}:${index}`} decision={decision} name={names[decision.mandate_id] ?? decision.mandate_id} />)}{decisions.length === 0 && <div className="text-[10px] text-muted-foreground">No active mandate required a conductor decision.</div>}</div></details>;
        })}</div>
      )}
    </div>
  );
}

function DecisionRow({ decision, name }: { decision: ConductorDecision; name: string }) {
  const text = decision.decision === "planned_discovery"
    ? `${decision.proposed_dispatch.product_slug} / ${decision.proposed_dispatch.action_id} · ${decision.capacity.admitted_repositories} repos admitted · ${decision.capacity.concrete_backlog} concrete backlog · effective ${decision.effective_autonomy}`
    : decision.decision === "capacity_deferred"
      ? `${decision.reason} Limits: ${decision.limiting_layers.join(", ") || "unavailable capacity"}.`
      : decision.reason;
  return <div className="grid gap-1 rounded border border-border/60 px-2 py-1.5 text-[10px] sm:grid-cols-[140px_120px_1fr]"><span className="font-display text-foreground">{name}</span><span className="font-mono text-[var(--honey)]">{decision.decision.replaceAll("_", " ")}</span><span className="text-muted-foreground">{text}</span></div>;
}

function Field({ label, value, onChange, placeholder = "", wide = false }: { label: string; value: string; onChange: (value: string) => void; placeholder?: string; wide?: boolean }) {
  return <label className={wide ? "sm:col-span-2" : ""}><span className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">{label}</span><input value={value} onChange={(event) => onChange(event.target.value)} placeholder={placeholder} className="mt-1 w-full rounded border border-border bg-background/60 px-2 py-1.5 text-[11px] outline-none focus:border-[var(--honey)]/50" /></label>;
}

function NumberField({ label, value, onChange }: { label: string; value: number; onChange: (value: number) => void }) {
  return <label><span className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">{label}</span><input type="number" min={0} value={value} onChange={(event) => onChange(Number(event.target.value))} className="mt-1 w-full rounded border border-border bg-background/60 px-2 py-1.5 text-[11px] outline-none focus:border-[var(--honey)]/50" /></label>;
}

function Select({ label, value, onChange }: { label: string; value: MandateAutonomy; onChange: (value: MandateAutonomy) => void }) {
  return <label><span className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">{label}</span><select value={value} onChange={(event) => onChange(event.target.value as MandateAutonomy)} className="mt-1 w-full rounded border border-border bg-background/60 px-2 py-1.5 text-[11px] outline-none focus:border-[var(--honey)]/50"><option value="observe">Observe</option><option value="propose">Propose</option><option value="act_with_approval">Act with approval</option><option value="act">Act</option></select></label>;
}

function Fact({ label, value }: { label: string; value: string }) {
  return <div><div className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">{label}</div><div className="mt-0.5 text-foreground">{value}</div></div>;
}

function SmallButton({ disabled, onClick, icon: Icon, children }: { disabled: boolean; onClick: () => void; icon: typeof Play; children: string }) {
  return <button disabled={disabled} onClick={onClick} className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-display text-[9px] uppercase tracking-wider text-muted-foreground hover:border-[var(--honey)]/40 hover:text-[var(--honey)] disabled:opacity-40"><Icon className="h-3 w-3" />{children}</button>;
}

function message(cause: unknown): string {
  return cause instanceof Error ? cause.message : "Unknown conductor error.";
}
