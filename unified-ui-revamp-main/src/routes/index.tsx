import { createFileRoute } from "@tanstack/react-router";
import { useMemo, useState } from "react";
import { Link } from "@tanstack/react-router";
import {
  ShieldAlert,
  Github,
  Search,
  Radio,
  Sparkles,
  Activity,
  ChevronRight,
  Zap,
  Clock,
  Cpu,
  ArrowUpRight,
  CheckCircle2,
  MoonStar,
  X,
} from "lucide-react";
import { ThemeToggle } from "../components/theme-toggle";
import {
  useFindings,
  findingsStore,
  OWNERS,
  type Finding,
  type Rec,
} from "../lib/findings-store";

export const Route = createFileRoute("/")({
  component: VulnTriageGlass,
});

const METRICS = [
  { label: "Fix now",   value: 7,  tone: "from-orange-700/70 to-red-900/60" },
  { label: "Plan next", value: 14, tone: "from-amber-600/70 to-yellow-800/50" },
  { label: "Watch",     value: 32, tone: "from-slate-500/70 to-slate-800/60" },
  { label: "Runtime",   value: 4,  tone: "from-stone-500/70 to-stone-800/60" },
];

// Shared surface classes — defined as `@utility` in src/styles.css so the
// same look ships everywhere and swaps cleanly on `.dark`.
const GLASS = "surface";
const GLASS_DIM = "surface-dim";
// Text tokens (bound to CSS vars in styles.css)
const T_STRONG = "text-[color:var(--text-strong)]";
const T_BODY   = "text-[color:var(--text-body)]";
const T_MUTE   = "text-[color:var(--text-mute)]";
const T_DIM    = "text-[color:var(--text-dim)]";

function VulnTriageGlass() {
  const [bucket, setBucket] = useState<"all" | "fix now" | "plan next" | "watch">("all");
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const FINDINGS = useFindings();

  const filtered = useMemo(() => {
    return FINDINGS.filter((f) => {
      if (bucket !== "all" && f.rec !== bucket) return false;
      if (query && !`${f.title} ${f.loc} ${f.pkg} ${f.id}`.toLowerCase().includes(query.toLowerCase())) return false;
      return true;
    });
  }, [bucket, query, FINDINGS]);

  const toggle = (id: string) =>
    setSelected((s) => {
      const n = new Set(s);
      if (n.has(id)) n.delete(id); else n.add(id);
      return n;
    });
  const toggleAll = () =>
    setSelected((s) => {
      const allIds = filtered.map((f) => f.id);
      const allSelected = allIds.every((id) => s.has(id));
      return allSelected ? new Set() : new Set(allIds);
    });
  const clearSelection = () => setSelected(new Set());
  const selectedIds = Array.from(selected);
  const allSelected = filtered.length > 0 && filtered.every((f) => selected.has(f.id));

  return (
    <main
      className="theme-transition min-h-screen relative overflow-hidden"
      style={{ background: "var(--page-bg)", color: "var(--text-body)" }}
    >
      <div aria-hidden className="pointer-events-none absolute inset-0 -z-10">
        <div className="absolute top-[10%] left-[8%] h-72 w-72 rounded-full opacity-60 blur-3xl dark:opacity-45" style={{ backgroundImage: "var(--orb-1)" }} />
        <div className="absolute top-[40%] right-[6%] h-96 w-96 rounded-full opacity-50 blur-3xl dark:opacity-40" style={{ backgroundImage: "var(--orb-2)" }} />
        <div className="absolute bottom-[6%] left-[30%] h-80 w-80 rounded-full opacity-40 blur-3xl dark:opacity-30" style={{ backgroundImage: "var(--orb-3)" }} />
      </div>
      <div aria-hidden className="pointer-events-none absolute inset-0 -z-10 opacity-[0.04]" style={{ backgroundImage: "url(\"data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='140' height='140'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2'/></filter><rect width='100%' height='100%' filter='url(%23n)'/></svg>\")" }} />

      <header className="px-6 pt-6">
        <div className={`${GLASS} mx-auto max-w-[1440px] px-5 h-16 flex items-center justify-between`}>
          <div className="flex items-center gap-3">
            <div className="h-10 w-10 rounded-xl grid place-items-center" style={{ backgroundImage: "linear-gradient(135deg, var(--accent-2), var(--accent-3))", boxShadow: "var(--accent-glow)" }}>
              <ShieldAlert size={18} className="text-white" />
            </div>
            <div className="leading-tight">
              <div className={`text-[10px] uppercase tracking-[0.22em] ${T_MUTE}`}>PatchHive · Suite</div>
              <div className={`font-display text-[16px] font-semibold tracking-tight ${T_STRONG}`}>
                VulnTriage <span className={`${T_DIM} font-normal`}>— security queue</span>
              </div>
            </div>
          </div>
          <nav className="surface-inset hidden md:flex items-center gap-1 rounded-full p-1">
            {["Triage","History","Checks","Sources"].map((t,i) => (
              <button key={t} className={`px-4 h-8 text-[12px] rounded-full transition ${i===0 ? `bg-white shadow ${T_STRONG} dark:bg-white/15` : `${T_MUTE} hover:opacity-100`}`}>{t}</button>
            ))}
          </nav>
          <div className="flex items-center gap-2">
            <ThemeToggle />
            <button className={`surface-inset h-9 px-3 rounded-full text-[12px] ${T_BODY} hover:brightness-110 flex items-center gap-2`}>
              <Github size={13} /> patchhive
            </button>
            <button className="h-9 px-4 rounded-full text-[12px] font-semibold text-white hover:brightness-110 flex items-center gap-2" style={{ backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))", boxShadow: "var(--accent-glow)" }}>
              <Radio size={13} /> Run scan
            </button>
          </div>
        </div>
      </header>

      <div className="mx-auto max-w-[1440px] px-6 pt-10 pb-24">
        <section className="grid grid-cols-12 gap-6 items-stretch">
          <div className={`${GLASS} col-span-8 p-10`}>
            <div className={`flex items-center gap-2 text-[11px] tracking-[0.2em] uppercase ${T_MUTE}`}>
              <Sparkles size={12} style={{ color: "var(--accent-2)" }} /> Security queue · patchhive/api
            </div>
            <h1 className={`font-display mt-4 text-[68px] leading-[0.95] tracking-[-0.03em] font-semibold ${T_STRONG}`}>
              Seven findings <br />
              need a decision{" "}
              <span className="bg-clip-text text-transparent" style={{ backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2), #cbd5e1)" }}>today.</span>
            </h1>
            <p className={`mt-6 max-w-xl text-[15px] ${T_BODY} leading-relaxed`}>
              Reads GitHub code scanning + Dependabot alerts across your repos and sorts them into
              fix-now, plan-next, watch — all in one calm, luminous surface.
            </p>
            <div className="mt-8 flex flex-wrap gap-2">
              {["patchhive/api","patchhive/edge","patchhive/web","patchhive/workers"].map((r,i) => (
                <span key={r} className={`surface-inset px-3 h-8 rounded-full text-[12px] flex items-center gap-2 ${i===0 ? T_STRONG : T_MUTE}`}>
                  <span className="h-1.5 w-1.5 rounded-full" style={{ background: i===0 ? "var(--accent-2)" : "var(--text-dim)" }} /> {r}
                </span>
              ))}
            </div>
          </div>

          <div className={`${GLASS} col-span-4 p-6 overflow-hidden`}>
            <div className="absolute -top-20 -right-16 h-56 w-56 rounded-full opacity-40 blur-2xl" style={{ backgroundImage: "var(--orb-1)" }} />
            <div className="relative">
              <div className={`text-[10px] uppercase tracking-[0.22em] ${T_MUTE} flex items-center gap-1.5`}>
                <Clock size={11} /> Next auto-scan
              </div>
              <div className={`mt-3 font-display text-[46px] font-semibold tabular-nums ${T_STRONG} leading-none`}>00:14:22</div>
              <div className={`mt-2 text-[12px] ${T_MUTE}`}>Cadence · every 15 min</div>

              <div className="mt-6 h-2 rounded-full overflow-hidden" style={{ background: "var(--surface-border)" }}>
                <div className="h-full w-[70%] rounded-full" style={{ backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))" }} />
              </div>

              <div className="mt-6 grid grid-cols-3 gap-2">
                {[["Scans/24h","96"],["Repos","12"],["Avg","1.4s"]].map(([l,v]) => (
                  <div key={l} className="surface-inset rounded-xl p-2.5">
                    <div className={`text-[10px] uppercase tracking-wider ${T_MUTE}`}>{l}</div>
                    <div className={`font-display text-[16px] font-semibold tabular-nums ${T_STRONG}`}>{v}</div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </section>

        <section className="mt-8 grid grid-cols-4 gap-4">
          {METRICS.map((m) => (
            <div key={m.label} className={`${GLASS} p-5 overflow-hidden`}>
              <div className={`absolute -top-8 -right-8 h-28 w-28 rounded-full bg-gradient-to-br ${m.tone} blur-2xl`} />
              <div className="relative">
                <div className={`text-[10px] uppercase tracking-[0.22em] ${T_MUTE}`}>{m.label}</div>
                <div className={`mt-3 font-display text-[46px] font-semibold tabular-nums ${T_STRONG} leading-none`}>
                  {String(m.value).padStart(2,"0")}
                </div>
                <div className={`mt-4 flex items-center justify-between text-[11px] ${T_MUTE}`}>
                  <span className="flex items-center gap-1"><Activity size={11}/> live</span>
                  <ArrowUpRight size={13} className={T_DIM} />
                </div>
              </div>
            </div>
          ))}
        </section>

        <section className="mt-8 grid grid-cols-12 gap-6">
          <div className="col-span-8 space-y-6">
            <div className={`${GLASS} p-5`}>
              <div className="flex items-end justify-between gap-3 mb-4">
                <div>
                  <div className={`text-[10px] uppercase tracking-[0.22em] ${T_MUTE}`}>Findings</div>
                  <div className={`font-display text-2xl mt-0.5 tracking-tight ${T_STRONG}`}>
                    {filtered.length} in view <span className={`${T_DIM} font-normal`}>/ {FINDINGS.length} tracked</span>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <div className="surface-inset flex items-center gap-2 rounded-full px-3 h-9 w-[240px]">
                    <Search size={13} className={T_DIM} />
                    <input
                      value={query}
                      onChange={(e) => setQuery(e.target.value)}
                      placeholder="Search CVE, file, package…"
                      className={`bg-transparent outline-none text-[12px] w-full ${T_STRONG} placeholder:text-[color:var(--text-dim)]`}
                    />
                  </div>
                  <div className="surface-inset flex rounded-full p-1">
                    {(["all","fix now","plan next","watch"] as const).map((b) => (
                      <button
                        key={b}
                        onClick={() => setBucket(b)}
                        className={`px-3 h-7 rounded-full text-[11px] capitalize transition ${bucket===b ? `bg-white shadow ${T_STRONG} dark:bg-white/15` : T_MUTE}`}
                      >{b}</button>
                    ))}
                  </div>
                </div>
              </div>

              <div className="mb-3 flex items-center gap-3 px-1">
                <label className={`flex items-center gap-2 text-[11px] ${T_MUTE} cursor-pointer`}>
                  <input
                    type="checkbox"
                    checked={allSelected}
                    onChange={toggleAll}
                    className="accent-[color:var(--accent-2)]"
                  />
                  Select all in view
                </label>
                {selectedIds.length > 0 && (
                  <span className={`text-[11px] ${T_STRONG}`}>{selectedIds.length} selected</span>
                )}
              </div>

              {selectedIds.length > 0 && (
                <div className="surface-inset mb-3 flex flex-wrap items-center gap-2 rounded-xl p-2.5">
                  <span className={`text-[10px] uppercase tracking-widest ${T_MUTE} px-2`}>Bulk</span>
                  <button
                    onClick={() => { findingsStore.setStatusMany(selectedIds, "fixed"); clearSelection(); }}
                    className="h-8 px-3 rounded-full text-[11px] font-medium flex items-center gap-1.5 bg-emerald-700/80 text-white hover:brightness-110"
                  >
                    <CheckCircle2 size={12} /> Mark fixed
                  </button>
                  <button
                    onClick={() => { findingsStore.setStatusMany(selectedIds, "snoozed"); clearSelection(); }}
                    className="h-8 px-3 rounded-full text-[11px] font-medium flex items-center gap-1.5 bg-slate-700/80 text-white hover:brightness-110"
                  >
                    <MoonStar size={12} /> Snooze
                  </button>
                  <div className={`h-5 w-px`} style={{ background: "var(--surface-border-2)" }} />
                  <span className={`text-[10px] uppercase tracking-widest ${T_MUTE} pl-1`}>Priority</span>
                  {(["fix now","plan next","watch"] as const).map((p) => (
                    <button
                      key={p}
                      onClick={() => { findingsStore.setPriorityMany(selectedIds, p); clearSelection(); }}
                      className={`h-8 px-3 rounded-full text-[11px] capitalize ${T_STRONG} border`}
                      style={{ borderColor: "var(--surface-border-2)" }}
                    >{p}</button>
                  ))}
                  <button
                    onClick={clearSelection}
                    className={`ml-auto h-8 px-3 rounded-full text-[11px] flex items-center gap-1.5 ${T_MUTE} hover:opacity-100`}
                  >
                    <X size={12} /> Clear
                  </button>
                </div>
              )}

              <div className="space-y-2">
                {filtered.length === 0 && (
                  <div className={`py-14 text-center ${T_MUTE} text-[13px]`}>No findings match this filter.</div>
                )}
                {filtered.map((f) => (
                  <FindingRow
                    key={f.id}
                    f={f}
                    checked={selected.has(f.id)}
                    onToggle={() => toggle(f.id)}
                  />
                ))}
              </div>
            </div>
          </div>

          <aside className="col-span-4 space-y-6">
            <div className={`${GLASS} p-5 overflow-hidden`}>
              <div className="absolute -top-10 -right-10 h-32 w-32 rounded-full opacity-60 blur-2xl" style={{ backgroundImage: "var(--orb-1)" }} />
              <div className="relative">
                <div className={`text-[10px] uppercase tracking-[0.22em] ${T_MUTE}`}>Active repo</div>
                <div className={`mt-2 font-display text-[22px] font-semibold ${T_STRONG}`}>patchhive/api</div>
                <div className={`text-[12px] ${T_MUTE}`}>main · TypeScript</div>
                <div className="mt-4 grid grid-cols-3 gap-2 text-center">
                  {[["Open","21"],["Fixed","104"],["SLA","98%"]].map(([l,v]) => (
                    <div key={l} className="surface-inset rounded-xl p-2">
                      <div className={`text-[10px] uppercase tracking-wider ${T_MUTE}`}>{l}</div>
                      <div className={`font-display text-[18px] font-semibold tabular-nums ${T_STRONG}`}>{v}</div>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            <div className={`${GLASS} p-5`}>
              <div className="flex items-center justify-between mb-3">
                <div className={`text-[10px] uppercase tracking-[0.22em] ${T_MUTE}`}>Feeds</div>
                <Zap size={13} style={{ color: "var(--accent-2)" }} />
              </div>
              {[
                { l: "Code scanning",   n: 18, dot: "bg-orange-600"   },
                { l: "Dependabot",      n: 35, dot: "bg-amber-500"    },
                { l: "Owner scoped",    n: 21, dot: "bg-slate-500"    },
                { l: "Runtime exposed", n: 4,  dot: "bg-red-700"      },
              ].map((f, i) => (
                <div key={f.l} className={`flex items-center justify-between py-2.5 ${i>0 ? "border-t" : ""}`} style={i>0 ? { borderColor: "var(--surface-border-2)" } : undefined}>
                  <span className={`flex items-center gap-2 text-[13px] ${T_BODY}`}>
                    <span className={`h-1.5 w-1.5 rounded-full ${f.dot}`} /> {f.l}
                  </span>
                  <span className={`font-display text-[15px] font-semibold tabular-nums ${T_STRONG}`}>{f.n}</span>
                </div>
              ))}
            </div>

            <div className={`${GLASS} p-5`}>
              <div className="flex items-center justify-between mb-3">
                <div className={`text-[10px] uppercase tracking-[0.22em] ${T_MUTE}`}>Recent scans</div>
                <Cpu size={13} className={T_MUTE} />
              </div>
              {[
                { r: "patchhive/api",     w: "2m",  n: 7,  d: "hold" },
                { r: "patchhive/edge",    w: "38m", n: 3,  d: "plan" },
                { r: "patchhive/web",     w: "2h",  n: 12, d: "hold" },
                { r: "patchhive/workers", w: "1d",  n: 1,  d: "ready"},
              ].map((s, i) => (
                <div key={s.r} className={`flex items-center justify-between py-2 ${i>0 ? "border-t" : ""}`} style={i>0 ? { borderColor: "var(--surface-border-2)" } : undefined}>
                  <div>
                    <div className={`text-[13px] ${T_STRONG}`}>{s.r}</div>
                    <div className={`text-[11px] ${T_MUTE}`}>{s.w} ago · {s.n} findings</div>
                  </div>
                  <span className={`text-[10px] uppercase tracking-wider px-2 py-0.5 rounded-full border ${
                    s.d === "hold"  ? "bg-red-950/10   text-red-800    border-red-900/30 dark:bg-red-500/10 dark:text-red-300 dark:border-red-400/25" :
                    s.d === "plan"  ? "bg-amber-900/10 text-amber-800  border-amber-900/30 dark:bg-amber-500/10 dark:text-amber-300 dark:border-amber-400/25" :
                                      "bg-stone-800/10 text-stone-800  border-stone-800/30 dark:bg-stone-500/10 dark:text-stone-300 dark:border-stone-400/25"
                  }`}>{s.d}</span>
                </div>
              ))}
            </div>
          </aside>
        </section>
      </div>

      <footer className="px-6 pb-8">
        <div className={`${GLASS_DIM} mx-auto max-w-[1440px] px-5 py-4 flex justify-between text-[11px] ${T_MUTE}`}>
          <span>PatchHive · VulnTriage</span>
          <span>Forged surface · sharp signal</span>
        </div>
      </footer>
    </main>
  );
}

function FindingRow({
  f,
  checked,
  onToggle,
}: {
  f: Finding;
  checked: boolean;
  onToggle: () => void;
}) {
  const sevMap: Record<string, string> = {
    critical: "from-red-800 to-orange-700 text-white",
    high:     "from-orange-700 to-amber-600 text-white",
    medium:   "from-amber-700 to-yellow-700 text-white",
    low:      "from-slate-600 to-slate-800 text-white",
  };
  const recMap: Record<string, string> = {
    "fix now":   "bg-red-900/10 text-red-800 border-red-900/30 dark:bg-red-500/10 dark:text-red-300 dark:border-red-400/25",
    "plan next": "bg-amber-900/10 text-amber-800 border-amber-900/30 dark:bg-amber-500/10 dark:text-amber-300 dark:border-amber-400/25",
    "watch":     "bg-stone-800/10 text-stone-800 border-stone-800/30 dark:bg-stone-500/10 dark:text-stone-300 dark:border-stone-400/25",
  };
  const statusChip =
    f.status === "fixed"
      ? "bg-emerald-500/15 text-emerald-700 border-emerald-500/30 dark:text-emerald-300"
      : f.status === "snoozed"
      ? "bg-slate-500/15 text-slate-600 border-slate-500/30 dark:text-slate-300"
      : "bg-transparent text-[color:var(--text-mute)] border-transparent";
  const stop = (e: React.MouseEvent | React.ChangeEvent) => e.stopPropagation();
  return (
    <div className={`surface-inset group rounded-xl p-4 hover:brightness-110 hover:shadow-[0_10px_30px_-15px_rgba(15,23,42,0.35)] ${f.status !== "open" ? "opacity-70" : ""}`}>
      <div className="grid grid-cols-[auto_auto_1fr_auto] gap-4 items-center">
        <input
          type="checkbox"
          checked={checked}
          onChange={(e) => { stop(e); onToggle(); }}
          onClick={stop}
          aria-label={`Select ${f.id}`}
          className="accent-[color:var(--accent-2)] h-4 w-4"
        />
        <div className={`h-12 w-12 rounded-xl bg-gradient-to-br ${sevMap[f.sev]} grid place-items-center shadow-inner`}>
          <span className="font-display font-semibold text-[15px] tabular-nums">{f.score.toFixed(1)}</span>
        </div>
        <Link
          to="/findings/$id"
          params={{ id: f.id }}
          className="min-w-0 block"
        >
          <div className="flex items-center gap-2 text-[10px] uppercase tracking-[0.2em] text-[color:var(--text-mute)]">
            <span>{f.id}</span><span className="opacity-40">·</span><span>{f.src}</span><span className="opacity-40">·</span><span>{f.age}</span>
            {f.status !== "open" && (
              <span className={`ml-1 px-2 py-0.5 rounded-full border text-[9px] tracking-widest ${statusChip}`}>{f.status}</span>
            )}
          </div>
          <div className="mt-1 font-display font-medium text-[16px] tracking-tight text-[color:var(--text-strong)] truncate">{f.title}</div>
          <div className="mt-1 text-[12px] text-[color:var(--text-mute)] font-mono truncate">
            {f.loc} <span className="text-[color:var(--text-dim)]">·</span> <span className="text-[color:var(--text-body)]">{f.pkg}</span>
          </div>
        </Link>
        <div className="flex items-center gap-2" onClick={stop}>
          <select
            value={f.owner ?? "unassigned"}
            onChange={(e) => { stop(e); findingsStore.setOwner(f.id, e.target.value); }}
            onClick={stop}
            className={`surface rounded-full h-8 px-2 text-[11px] ${T_BODY} outline-none border-0`}
            aria-label="Assign owner"
          >
            {OWNERS.map((o) => <option key={o} value={o}>{o}</option>)}
          </select>
          <select
            value={f.rec}
            onChange={(e) => { stop(e); findingsStore.setPriority(f.id, e.target.value as Rec); }}
            onClick={stop}
            className={`text-[10px] uppercase tracking-widest px-2.5 py-1 rounded-full border ${recMap[f.rec]} outline-none`}
            aria-label="Set priority"
          >
            <option value="fix now">fix now</option>
            <option value="plan next">plan next</option>
            <option value="watch">watch</option>
          </select>
          <button
            onClick={(e) => { stop(e); findingsStore.setStatus(f.id, f.status === "fixed" ? "open" : "fixed"); }}
            title={f.status === "fixed" ? "Reopen" : "Mark fixed"}
            className="surface h-8 w-8 rounded-full grid place-items-center hover:brightness-110"
          >
            <CheckCircle2 size={13} className={f.status === "fixed" ? "text-emerald-500" : T_MUTE} />
          </button>
          <button
            onClick={(e) => { stop(e); findingsStore.snooze(f.id, 7); }}
            title="Snooze 7d"
            className="surface h-8 w-8 rounded-full grid place-items-center hover:brightness-110"
          >
            <MoonStar size={13} className={T_MUTE} />
          </button>
          <Link
            to="/findings/$id"
            params={{ id: f.id }}
            className="surface h-8 w-8 rounded-full grid place-items-center hover:brightness-110"
            aria-label="Open detail"
          >
            <ChevronRight size={14} className="text-[color:var(--text-dim)] group-hover:translate-x-0.5 transition" />
          </Link>
        </div>
      </div>
    </div>
  );
}