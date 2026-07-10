import { createFileRoute, Link, useRouter } from "@tanstack/react-router";
import { useState } from "react";
import {
  ArrowLeft,
  ShieldAlert,
  Clock,
  User,
  CheckCircle2,
  MoonStar,
  MessageSquare,
  ExternalLink,
  Activity as ActivityIcon,
} from "lucide-react";
import { ThemeToggle } from "../components/theme-toggle";
import {
  useFinding,
  findingsStore,
  OWNERS,
  type Rec,
  type Status,
} from "../lib/findings-store";

export const Route = createFileRoute("/findings/$id")({
  component: FindingDetail,
  notFoundComponent: () => (
    <div className="min-h-screen grid place-items-center text-[color:var(--text-body)]">
      <div className="text-center">
        <div className="text-sm uppercase tracking-widest text-[color:var(--text-mute)]">404</div>
        <div className="mt-2 font-display text-3xl">Finding not found</div>
        <Link to="/" className="mt-4 inline-block text-sm underline">Back to queue</Link>
      </div>
    </div>
  ),
});

const T_STRONG = "text-[color:var(--text-strong)]";
const T_BODY = "text-[color:var(--text-body)]";
const T_MUTE = "text-[color:var(--text-mute)]";
const T_DIM = "text-[color:var(--text-dim)]";

const sevMap: Record<string, string> = {
  critical: "from-red-800 to-orange-700 text-white",
  high: "from-orange-700 to-amber-600 text-white",
  medium: "from-amber-700 to-yellow-700 text-white",
  low: "from-slate-600 to-slate-800 text-white",
};

function fmt(iso: string) {
  return new Date(iso).toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

function FindingDetail() {
  const { id } = Route.useParams();
  const router = useRouter();
  const f = useFinding(id);
  const [note, setNote] = useState("");

  if (!f) {
    return (
      <main className="theme-transition min-h-screen" style={{ background: "var(--page-bg)", color: "var(--text-body)" }}>
        <div className="mx-auto max-w-3xl px-6 py-24 text-center">
          <div className="text-xs uppercase tracking-widest text-[color:var(--text-mute)]">Missing</div>
          <h1 className={`mt-2 font-display text-4xl ${T_STRONG}`}>Finding not found</h1>
          <Link to="/" className="mt-6 inline-block text-sm underline">Back to queue</Link>
        </div>
      </main>
    );
  }

  return (
    <main
      className="theme-transition min-h-screen relative"
      style={{ background: "var(--page-bg)", color: "var(--text-body)" }}
    >
      <div aria-hidden className="pointer-events-none absolute inset-0 -z-10">
        <div className="absolute top-[8%] left-[6%] h-72 w-72 rounded-full opacity-50 blur-3xl" style={{ backgroundImage: "var(--orb-1)" }} />
        <div className="absolute top-[30%] right-[4%] h-96 w-96 rounded-full opacity-40 blur-3xl" style={{ backgroundImage: "var(--orb-2)" }} />
      </div>

      <header className="px-6 pt-6">
        <div className="surface mx-auto max-w-[1200px] px-5 h-16 flex items-center justify-between">
          <button
            onClick={() => router.history.back()}
            className={`flex items-center gap-2 text-[12px] ${T_BODY} hover:opacity-100`}
          >
            <ArrowLeft size={14} /> Back to queue
          </button>
          <div className={`text-[10px] uppercase tracking-[0.22em] ${T_MUTE}`}>PatchHive · Finding detail</div>
          <ThemeToggle />
        </div>
      </header>

      <div className="mx-auto max-w-[1200px] px-6 pt-8 pb-24 grid grid-cols-12 gap-6">
        <section className="col-span-8 space-y-6">
          <div className="surface p-8">
            <div className="flex items-start gap-5">
              <div className={`h-16 w-16 rounded-2xl bg-gradient-to-br ${sevMap[f.sev]} grid place-items-center shadow-inner shrink-0`}>
                <div className="text-center">
                  <div className="font-display font-semibold text-[18px] tabular-nums leading-none">{f.score.toFixed(1)}</div>
                  <div className="mt-1 text-[9px] uppercase tracking-widest opacity-80">{f.sev}</div>
                </div>
              </div>
              <div className="min-w-0 flex-1">
                <div className={`flex flex-wrap items-center gap-2 text-[10px] uppercase tracking-[0.2em] ${T_MUTE}`}>
                  <span>{f.id}</span><span className="opacity-40">·</span>
                  <span>{f.src}</span><span className="opacity-40">·</span>
                  <span className="flex items-center gap-1"><ShieldAlert size={11}/> {f.status}</span>
                </div>
                <h1 className={`mt-2 font-display text-[32px] leading-tight tracking-tight font-semibold ${T_STRONG}`}>
                  {f.title}
                </h1>
                <div className={`mt-2 text-[13px] font-mono ${T_MUTE}`}>
                  {f.loc} <span className={T_DIM}>·</span> <span className={T_BODY}>{f.pkg}</span>
                </div>
              </div>
            </div>

            <div className="mt-8 grid grid-cols-4 gap-3 text-[12px]">
              <Meta label="Detected" value={fmt(f.detectedAt)} />
              <Meta label="Updated" value={fmt(f.updatedAt)} />
              <Meta label="Owner" value={f.owner ?? "unassigned"} />
              <Meta label="Priority" value={f.rec} />
            </div>
          </div>

          <div className="surface p-6">
            <div className={`text-[10px] uppercase tracking-[0.22em] ${T_MUTE}`}>Context</div>
            <p className={`mt-3 text-[14px] leading-relaxed ${T_BODY}`}>{f.description}</p>
            <div className={`mt-6 text-[10px] uppercase tracking-[0.22em] ${T_MUTE}`}>Remediation</div>
            <p className={`mt-3 text-[14px] leading-relaxed ${T_BODY}`}>{f.remediation}</p>
            {f.references.length > 0 && (
              <>
                <div className={`mt-6 text-[10px] uppercase tracking-[0.22em] ${T_MUTE}`}>References</div>
                <ul className="mt-2 space-y-1">
                  {f.references.map((r) => (
                    <li key={r.href}>
                      <a href={r.href} target="_blank" rel="noreferrer" className={`text-[13px] ${T_BODY} inline-flex items-center gap-1 hover:opacity-80`}>
                        {r.label} <ExternalLink size={11} />
                      </a>
                    </li>
                  ))}
                </ul>
              </>
            )}
          </div>

          <div className="surface p-6">
            <div className="flex items-center justify-between">
              <div className={`text-[10px] uppercase tracking-[0.22em] ${T_MUTE} flex items-center gap-1.5`}>
                <ActivityIcon size={11} /> Activity timeline
              </div>
              <span className={`text-[11px] ${T_DIM}`}>{f.activity.length} events</span>
            </div>
            <ol className="mt-5 relative border-l pl-5" style={{ borderColor: "var(--surface-border-2)" }}>
              {f.activity.map((e) => (
                <li key={e.id} className="mb-5 last:mb-0">
                  <div
                    className="absolute -left-[5px] h-2.5 w-2.5 rounded-full"
                    style={{ background: "var(--accent-2)" }}
                  />
                  <div className={`text-[11px] uppercase tracking-widest ${T_MUTE}`}>
                    {e.kind} · {fmt(e.at)}
                  </div>
                  <div className={`mt-1 text-[13px] ${T_STRONG}`}>{e.message}</div>
                  <div className={`text-[11px] ${T_DIM}`}>by {e.actor}</div>
                </li>
              ))}
            </ol>

            <form
              onSubmit={(ev) => {
                ev.preventDefault();
                if (!note.trim()) return;
                findingsStore.addNote(f.id, note.trim());
                setNote("");
              }}
              className="mt-6 flex gap-2"
            >
              <input
                value={note}
                onChange={(e) => setNote(e.target.value)}
                placeholder="Add a note to the timeline…"
                className={`surface-inset flex-1 rounded-full px-4 h-10 text-[13px] outline-none ${T_STRONG} placeholder:text-[color:var(--text-dim)]`}
              />
              <button
                type="submit"
                className="h-10 px-4 rounded-full text-[12px] font-semibold text-white flex items-center gap-2"
                style={{ backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))" }}
              >
                <MessageSquare size={13} /> Post
              </button>
            </form>
          </div>
        </section>

        <aside className="col-span-4 space-y-6">
          <div className="surface p-6 space-y-4">
            <div className={`text-[10px] uppercase tracking-[0.22em] ${T_MUTE}`}>Actions</div>
            <div className="grid grid-cols-2 gap-2">
              <button
                onClick={() => findingsStore.setStatus(f.id, "fixed")}
                disabled={f.status === "fixed"}
                className="surface-inset h-10 rounded-lg text-[12px] flex items-center justify-center gap-2 hover:brightness-110 disabled:opacity-50"
              >
                <CheckCircle2 size={13} /> Mark fixed
              </button>
              <button
                onClick={() => findingsStore.snooze(f.id, 7)}
                disabled={f.status === "snoozed"}
                className="surface-inset h-10 rounded-lg text-[12px] flex items-center justify-center gap-2 hover:brightness-110 disabled:opacity-50"
              >
                <MoonStar size={13} /> Snooze 7d
              </button>
              <button
                onClick={() => findingsStore.setStatus(f.id, "open")}
                disabled={f.status === "open"}
                className="surface-inset h-10 rounded-lg text-[12px] flex items-center justify-center gap-2 hover:brightness-110 disabled:opacity-50 col-span-2"
              >
                Reopen
              </button>
            </div>

            <div>
              <label className={`text-[10px] uppercase tracking-[0.22em] ${T_MUTE}`}>Priority</label>
              <select
                value={f.rec}
                onChange={(e) => findingsStore.setPriority(f.id, e.target.value as Rec)}
                className={`mt-1 w-full surface-inset h-10 rounded-lg px-3 text-[13px] ${T_STRONG} outline-none`}
              >
                <option value="fix now">fix now</option>
                <option value="plan next">plan next</option>
                <option value="watch">watch</option>
              </select>
            </div>

            <div>
              <label className={`text-[10px] uppercase tracking-[0.22em] ${T_MUTE} flex items-center gap-1.5`}>
                <User size={11} /> Owner
              </label>
              <select
                value={f.owner ?? "unassigned"}
                onChange={(e) => findingsStore.setOwner(f.id, e.target.value)}
                className={`mt-1 w-full surface-inset h-10 rounded-lg px-3 text-[13px] ${T_STRONG} outline-none`}
              >
                {OWNERS.map((o) => (
                  <option key={o} value={o}>{o}</option>
                ))}
              </select>
            </div>

            <div>
              <label className={`text-[10px] uppercase tracking-[0.22em] ${T_MUTE}`}>Status</label>
              <select
                value={f.status}
                onChange={(e) => findingsStore.setStatus(f.id, e.target.value as Status)}
                className={`mt-1 w-full surface-inset h-10 rounded-lg px-3 text-[13px] ${T_STRONG} outline-none`}
              >
                <option value="open">open</option>
                <option value="fixed">fixed</option>
                <option value="snoozed">snoozed</option>
              </select>
            </div>
          </div>

          <div className="surface p-6">
            <div className={`text-[10px] uppercase tracking-[0.22em] ${T_MUTE} flex items-center gap-1.5`}>
              <Clock size={11} /> Snooze status
            </div>
            <div className={`mt-3 text-[13px] ${T_BODY}`}>
              {f.snoozedUntil
                ? <>Snoozed until <span className={T_STRONG}>{fmt(f.snoozedUntil)}</span></>
                : <span className={T_MUTE}>Not snoozed</span>}
            </div>
          </div>
        </aside>
      </div>
    </main>
  );
}

function Meta({ label, value }: { label: string; value: string }) {
  return (
    <div className="surface-inset rounded-xl p-3">
      <div className={`text-[10px] uppercase tracking-widest ${T_MUTE}`}>{label}</div>
      <div className={`mt-1 text-[13px] ${T_STRONG} truncate`}>{value}</div>
    </div>
  );
}