import { useEffect, useMemo, useState } from "react";
import { BookOpenCheck, BrainCircuit, Database, ExternalLink, GitCompareArrows, Pin, Search, Sparkles } from "lucide-react";
import {
  CopyMarkdownButton,
  DashboardControls,
  GuidanceNotice,
  ProgressiveList,
  V3_TEXT,
  useSavedDashboardViews,
} from "@patchhivehq/ui-v3";

async function readJson(response, fallback) {
  const data = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(data.error || data.message || fallback);
  return data;
}

function tone(disposition) {
  if (disposition === "policy") return "border-emerald-900/30 bg-emerald-900/10 text-emerald-800 dark:border-emerald-400/25 dark:bg-emerald-500/10 dark:text-emerald-300";
  if (disposition === "suppressed") return "border-stone-800/30 bg-stone-800/10 text-stone-700 dark:border-stone-400/20 dark:bg-stone-400/5 dark:text-stone-300";
  return "border-amber-900/30 bg-amber-900/10 text-amber-800 dark:border-amber-400/25 dark:bg-amber-500/10 dark:text-amber-300";
}

function Badge({ children, disposition = "signal" }) {
  return <span className={`inline-flex min-h-7 items-center rounded-full border px-2.5 text-[10px] uppercase tracking-wider ${tone(disposition)}`}>{children}</span>;
}

function frequencyBadge(entry) {
  if (entry.kind !== "hotspot") return `${entry.frequency || 0} hits`;
  return (entry.evidence || []).some((evidence) => evidence.url)
    ? `${entry.frequency || 0} merged PRs`
    : `${entry.frequency || 0} historical path touches`;
}

function Action({ children, disabled, onClick }) {
  return <button className={`surface-inset inline-flex h-9 items-center gap-2 rounded-full px-4 text-[11px] ${V3_TEXT.body} disabled:opacity-40`} disabled={disabled} onClick={onClick} type="button">{children}</button>;
}

function Field({ label, onChange, placeholder, rows, value }) {
  return <label className="block"><span className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>{label}</span><div className="surface-inset mt-2 rounded-xl px-3 py-2">{rows ? <textarea className={`w-full resize-y bg-transparent text-[12px] leading-relaxed outline-none ${V3_TEXT.strong}`} onChange={(event) => onChange(event.target.value)} placeholder={placeholder} rows={rows} value={value} /> : <input className={`h-8 w-full bg-transparent text-[12px] outline-none ${V3_TEXT.strong}`} onChange={(event) => onChange(event.target.value)} placeholder={placeholder} value={value} />}</div></label>;
}

function MemoryCard({ busy, entry, onCurate }) {
  return <article className="surface-inset rounded-xl p-4"><div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between"><div><div className={`font-display text-[17px] font-semibold ${V3_TEXT.strong}`}>{entry.title}</div><div className={`mt-1 text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>{entry.repo} · {String(entry.kind || "memory").replaceAll("_", " ")}</div></div><div className="flex flex-wrap gap-2"><Badge disposition={entry.disposition}>{entry.disposition || "signal"}</Badge>{entry.pinned ? <Badge disposition="policy"><Pin size={10} /> pinned</Badge> : null}<Badge>{Math.round(entry.confidence || 0)}%</Badge><Badge>{frequencyBadge(entry)}</Badge></div></div><p className={`mt-3 text-[12px] leading-relaxed ${V3_TEXT.body}`}>{entry.detail}</p><div className="surface mt-4 rounded-xl p-3"><div className={`text-[9px] uppercase tracking-wider ${V3_TEXT.mute}`}>Agent prompt line</div><p className={`mt-1 text-[12px] leading-relaxed ${V3_TEXT.strong}`}>{entry.prompt_line}</p></div>{entry.tags?.length ? <div className="mt-3 flex flex-wrap gap-2">{entry.tags.map((tag) => <span className={`surface rounded-full px-2.5 py-1 text-[10px] ${V3_TEXT.mute}`} key={`${entry.id}-${tag}`}>{tag}</span>)}</div> : null}<div className="mt-4 flex flex-wrap gap-2"><Action disabled={busy} onClick={() => onCurate(entry, "policy", true)}>Pin as policy</Action><Action disabled={busy} onClick={() => onCurate(entry, "signal", false)}>Keep as signal</Action><Action disabled={busy} onClick={() => onCurate(entry, "suppressed", false)}>Suppress</Action></div>{entry.evidence?.length ? <details className="surface mt-4 rounded-xl p-3"><summary className={`cursor-pointer text-[11px] font-semibold ${V3_TEXT.strong}`}>{entry.evidence.length} evidence item{entry.evidence.length === 1 ? "" : "s"}</summary><div className="mt-3 space-y-2">{entry.evidence.map((evidence, index) => <div className="surface-inset rounded-lg p-3" key={`${entry.id}-evidence-${index}`}><div className="flex flex-wrap items-center gap-2">{evidence.url ? <a className={`inline-flex items-center gap-1 text-[11px] ${V3_TEXT.strong}`} href={evidence.url} rel="noreferrer" target="_blank">{evidence.title || "Open evidence"}<ExternalLink size={11} /></a> : <span className={`text-[11px] ${V3_TEXT.strong}`}>{evidence.title}</span>}{evidence.path ? <Badge>{evidence.path}</Badge> : null}</div><p className={`mt-1 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>{evidence.excerpt}</p></div>)}</div></details> : null}</article>;
}

export function MemoryLibraryPanel({ apiBase, fetcher, form, onError, onRefresh }) {
  const [repos, setRepos] = useState([]);
  const [memories, setMemories] = useState([]);
  const [query, setQuery] = useState("");
  const [repo, setRepo] = useState(form.repo || "");
  const [busy, setBusy] = useState(false);
  const [preview, setPreview] = useState(null);
  const [previewForm, setPreviewForm] = useState({ consumer: "trust-gate", task_summary: "", changed_paths: "", diff_summary: "" });
  const dashboard = useSavedDashboardViews({ storageKey: "repo-memory.v3.memory-library", defaultView: { kind: "all", disposition: "all", sort: "confidence" } });

  async function load() {
    try {
      const [repoData, memoryData] = await Promise.all([readJson(await fetcher(`${apiBase}/repos`), "Could not load repositories."), readJson(await fetcher(`${apiBase}/memories`), "Could not load memories.")]);
      setRepos(repoData.repos || []);
      setMemories(memoryData.memories || []);
      if (!repo && repoData.repos?.[0]?.repo) setRepo(repoData.repos[0].repo);
    } catch (error) {
      onError(error.message || "RepoMemory could not load durable memories.");
    }
  }

  useEffect(() => { load(); }, []);
  useEffect(() => { if (form.repo) setRepo(form.repo); }, [form.repo]);

  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const next = memories.filter((entry) => (!repo || entry.repo === repo)
      && (dashboard.view.kind === "all" || entry.kind === dashboard.view.kind)
      && (dashboard.view.disposition === "all" || entry.disposition === dashboard.view.disposition)
      && (!needle || `${entry.title} ${entry.detail} ${entry.prompt_line} ${(entry.tags || []).join(" ")}`.toLowerCase().includes(needle)));
    return [...next].sort((left, right) => dashboard.view.sort === "frequency" ? (right.frequency || 0) - (left.frequency || 0) : dashboard.view.sort === "title" ? left.title.localeCompare(right.title) : (right.confidence || 0) - (left.confidence || 0));
  }, [dashboard.view, memories, query, repo]);

  const kinds = [...new Set(memories.filter((entry) => !repo || entry.repo === repo).map((entry) => entry.kind).filter(Boolean))].sort();
  const filters = [
    { key: "kind", label: "Kind", value: dashboard.view.kind, options: [{ value: "all", label: "All" }, ...kinds.map((kind) => ({ value: kind, label: kind.replaceAll("_", " ") }))] },
    { key: "disposition", label: "Disposition", value: dashboard.view.disposition, options: [{ value: "all", label: "All" }, { value: "policy", label: "Policy" }, { value: "signal", label: "Signal" }, { value: "suppressed", label: "Suppressed" }] },
  ];

  async function curate(entry, disposition, pinned) {
    setBusy(true);
    try {
      await readJson(await fetcher(`${apiBase}/memories/curation`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ repo: entry.repo, memory_ref: entry.memory_ref, disposition, pinned }) }), "Could not save memory curation.");
      await load();
      onRefresh();
    } catch (error) {
      onError(error.message || "RepoMemory could not curate that memory.");
    } finally {
      setBusy(false);
    }
  }

  async function runPreview() {
    if (!repo) return onError("Choose a repository before previewing context.");
    setBusy(true);
    try {
      const data = await readJson(await fetcher(`${apiBase}/context`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ repo, consumer: previewForm.consumer, task_summary: previewForm.task_summary, diff_summary: previewForm.diff_summary, changed_paths: previewForm.changed_paths.split("\n").map((item) => item.trim()).filter(Boolean), limit: 6 }) }), "Could not preview context.");
      setPreview(data);
    } catch (error) {
      onError(error.message || "RepoMemory could not preview context.");
    } finally {
      setBusy(false);
    }
  }

  return <div className="space-y-6"><section className="surface p-6 sm:p-8"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><BrainCircuit size={12} /> Durable context</div><h1 className={`mt-2 font-display text-[42px] font-semibold ${V3_TEXT.strong}`}>Memory library.</h1><p className={`mt-3 max-w-3xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>Search, inspect, pin, soften, or suppress the repo knowledge that downstream PatchHive products retrieve.</p><label className="mt-6 block max-w-xl"><span className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>Repository</span><div className="surface-inset mt-2 rounded-xl px-3"><select className={`h-11 w-full bg-transparent text-[12px] outline-none ${V3_TEXT.strong}`} onChange={(event) => setRepo(event.target.value)} value={repo}><option value="">All repositories</option>{repos.map((item) => <option key={item.repo} value={item.repo}>{item.repo}</option>)}</select></div></label></section>
    <section className="surface p-5 sm:p-6"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Memory queue</div><div className={`mt-1 font-display text-[24px] ${V3_TEXT.strong}`}>{visible.length} in view <span className={`${V3_TEXT.dim} font-normal`}>/ {memories.length} stored</span></div><div className="mt-5"><DashboardControls filters={filters} onApplySavedView={dashboard.applyView} onDeleteSavedView={dashboard.deleteView} onFilterChange={dashboard.updateView} onQueryChange={setQuery} onReset={() => { dashboard.resetView(); setQuery(""); }} onSaveView={dashboard.saveView} onSortChange={(sort) => dashboard.updateView("sort", sort)} query={query} savedViews={dashboard.savedViews} searchPlaceholder="Search title, lesson, prompt, tag…" sort={dashboard.view.sort} sortOptions={[{ value: "confidence", label: "Highest confidence" }, { value: "frequency", label: "Most repeated" }, { value: "title", label: "Title · A to Z" }]} /></div><div className="mt-6"><ProgressiveList empty={<div className={`py-14 text-center text-[13px] ${V3_TEXT.mute}`}>No memories match this view.</div>} initialCount={6} itemLabel="memories" items={visible} renderItem={(entry) => <MemoryCard busy={busy} entry={entry} key={entry.id} onCurate={curate} />} /></div></section>
    <section className="surface p-5 sm:p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Search size={12} /> Consumer context preview</div><h2 className={`mt-1 font-display text-[28px] ${V3_TEXT.strong}`}>Test retrieval before handoff.</h2><div className="mt-5 grid gap-4 sm:grid-cols-2"><Field label="Consumer" onChange={(value) => setPreviewForm((current) => ({ ...current, consumer: value }))} placeholder="trust-gate, repo-reaper, merge-keeper" value={previewForm.consumer} /><Field label="Task summary" onChange={(value) => setPreviewForm((current) => ({ ...current, task_summary: value }))} placeholder="Fix flaky worker timeout" value={previewForm.task_summary} /><Field label="Changed paths · one per line" onChange={(value) => setPreviewForm((current) => ({ ...current, changed_paths: value }))} rows={4} value={previewForm.changed_paths} /><Field label="Diff summary" onChange={(value) => setPreviewForm((current) => ({ ...current, diff_summary: value }))} rows={4} value={previewForm.diff_summary} /></div><div className="mt-4"><Action disabled={busy || !repo} onClick={runPreview}><Sparkles size={12} /> Preview context</Action></div>{preview ? <div className="surface-inset mt-5 rounded-xl p-4"><div className="flex flex-wrap items-center justify-between gap-2"><div className={`font-display text-[17px] ${V3_TEXT.strong}`}>{preview.consumer || "generic"} context</div><div className="flex gap-2"><Badge disposition="policy">{preview.entries?.length || 0} entries</Badge><CopyMarkdownButton content={(preview.prompt_lines || []).join("\n")} label="Copy prompt lines" onError={() => onError("Could not copy context.")} /></div></div><p className={`mt-2 text-[12px] leading-relaxed ${V3_TEXT.body}`}>{preview.summary}</p><div className="mt-4 space-y-2">{(preview.entries || []).map((entry) => <div className="surface rounded-xl p-3" key={entry.id}><div className={`font-semibold ${V3_TEXT.strong}`}>{entry.title}</div><p className={`mt-1 text-[11px] ${V3_TEXT.body}`}>{entry.prompt_line}</p></div>)}</div></div> : null}</section>
  </div>;
}

export function PromptPackPanel({ apiBase, fetcher, history, onError, result }) {
  const [runId, setRunId] = useState(result?.id || history[0]?.id || "");
  const [pack, setPack] = useState(null);
  const [diff, setDiff] = useState(null);
  const [loading, setLoading] = useState(false);

  async function load(id) {
    if (!id) return;
    setLoading(true);
    try {
      const [nextPack, nextDiff] = await Promise.all([readJson(await fetcher(`${apiBase}/history/${encodeURIComponent(id)}/prompt-pack`), "Could not load prompt pack."), readJson(await fetcher(`${apiBase}/history/${encodeURIComponent(id)}/diff`), "Could not load run diff.")]);
      setPack(nextPack);
      setDiff(nextDiff);
    } catch (error) {
      onError(error.message || "RepoMemory could not load prompt-pack evidence.");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => { const id = result?.id || history[0]?.id || ""; if (id) { setRunId(id); load(id); } }, [result?.id, history[0]?.id]);

  return <div className="space-y-6"><section className="surface p-6 sm:p-8"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><BookOpenCheck size={12} /> Agent handoff</div><h1 className={`mt-2 font-display text-[42px] font-semibold ${V3_TEXT.strong}`}>Prompt packs.</h1><p className={`mt-3 max-w-3xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>Load the exact durable context bundle produced by a saved ingest and inspect how memory moved since the prior run.</p><div className="mt-6 flex flex-col gap-3 sm:flex-row"><select className={`surface-inset h-11 min-w-0 flex-1 rounded-xl bg-transparent px-3 text-[12px] ${V3_TEXT.strong}`} onChange={(event) => { setRunId(event.target.value); load(event.target.value); }} value={runId}><option value="">Choose a saved run</option>{history.map((entry) => <option key={entry.id} value={entry.id}>{entry.repo} · {new Date(entry.created_at).toLocaleString()}</option>)}</select><Action disabled={loading || !runId} onClick={() => load(runId)}>{loading ? "Loading…" : "Refresh pack"}</Action></div></section>
    {diff ? <section className="surface p-5 sm:p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><GitCompareArrows size={12} /> Memory movement</div><h2 className={`mt-1 font-display text-[26px] ${V3_TEXT.strong}`}>{diff.previous_run_id ? "Compared with prior ingest." : "Baseline ingest."}</h2><p className={`mt-2 text-[12px] ${V3_TEXT.body}`}>{diff.summary}</p><div className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-4">{[["New", diff.counts?.new_entries], ["Strengthened", diff.counts?.strengthened_entries], ["Faded", diff.counts?.faded_entries], ["Retired", diff.counts?.retired_entries]].map(([label, value]) => <div className="surface-inset rounded-xl p-3" key={label}><div className={`text-[9px] uppercase tracking-wider ${V3_TEXT.mute}`}>{label}</div><div className={`mt-1 font-display text-[24px] ${V3_TEXT.strong}`}>{value || 0}</div></div>)}</div></section> : null}
    <section className="surface p-5 sm:p-6"><div className="flex flex-wrap items-center justify-between gap-3"><div><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Database size={12} /> Saved prompt-pack evidence</div><div className={`mt-1 font-display text-[24px] ${V3_TEXT.strong}`}>{pack?.repo || "No run selected"}</div></div>{pack?.prompt_pack ? <CopyMarkdownButton content={pack.prompt_pack} label="Copy prompt pack" onError={() => onError("Could not copy the prompt pack.")} /> : null}</div>{pack?.prompt_pack ? <pre className={`surface-inset mt-5 max-h-[620px] overflow-auto whitespace-pre-wrap rounded-xl p-4 text-[11px] leading-relaxed ${V3_TEXT.body}`}>{pack.prompt_pack}</pre> : <div className={`py-14 text-center text-[13px] ${V3_TEXT.mute}`}>Choose a saved ingest to load its prompt pack.</div>}</section>
    <GuidanceNotice label="Handoff boundary">Prompt packs are read-only context bundles. Loading or copying one does not rerun an ingest or perform repository writes.</GuidanceNotice>
  </div>;
}
