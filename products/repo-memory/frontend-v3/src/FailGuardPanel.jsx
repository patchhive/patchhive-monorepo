import { useEffect, useMemo, useState } from "react";
import { ArchiveX, CheckCircle2, ShieldAlert, Sparkles } from "lucide-react";
import { DashboardControls, GuidanceNotice, ProgressiveList, V3_TEXT, useSavedDashboardViews } from "@patchhivehq/ui-v3";

const EMPTY_CANDIDATE = { repo: "", source_type: "operator", source_ref: "", title: "", outcome: "", lesson: "", prevention: "", affected_paths: "", evidence: "" };
const EMPTY_LESSON = { repo: "", title: "", outcome: "", lesson: "", prevention: "", affected_paths: "", evidence: "", disposition: "policy", pinned: true };

async function readJson(response, fallback) {
  const data = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(data.error || data.message || fallback);
  return data;
}

function lines(value) {
  return String(value || "").split("\n").map((item) => item.trim()).filter(Boolean);
}

function Field({ full = false, label, onChange, placeholder, rows, value }) {
  return <label className={full ? "block sm:col-span-2" : "block"}><span className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>{label}</span><div className="surface-inset mt-2 rounded-xl px-3 py-2">{rows ? <textarea className={`w-full resize-y bg-transparent text-[12px] leading-relaxed outline-none ${V3_TEXT.strong}`} onChange={(event) => onChange(event.target.value)} placeholder={placeholder} rows={rows} value={value} /> : <input className={`h-8 w-full bg-transparent text-[12px] outline-none ${V3_TEXT.strong}`} onChange={(event) => onChange(event.target.value)} placeholder={placeholder} value={value} />}</div></label>;
}

function Action({ children, danger = false, disabled, primary = false, onClick }) {
  return <button className={`inline-flex h-9 items-center gap-2 rounded-full px-4 text-[11px] font-medium disabled:opacity-40 ${primary ? "text-white" : `surface-inset ${danger ? "text-red-700 dark:text-red-300" : V3_TEXT.body}`}`} disabled={disabled} onClick={onClick} style={primary ? { backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))" } : undefined} type="button">{children}</button>;
}

function Badge({ children, status = "open" }) {
  const classes = status === "promoted" ? "border-emerald-900/30 bg-emerald-900/10 text-emerald-800 dark:border-emerald-400/25 dark:bg-emerald-500/10 dark:text-emerald-300" : status === "dismissed" ? "border-stone-800/30 bg-stone-800/10 text-stone-700 dark:border-stone-400/20 dark:bg-stone-400/5 dark:text-stone-300" : "border-amber-900/30 bg-amber-900/10 text-amber-800 dark:border-amber-400/25 dark:bg-amber-500/10 dark:text-amber-300";
  return <span className={`inline-flex min-h-7 items-center rounded-full border px-2.5 text-[10px] uppercase tracking-wider ${classes}`}>{children}</span>;
}

function reviewFrom(candidate) {
  return { title: candidate.title || "", outcome: candidate.outcome || "", lesson: candidate.lesson || "", prevention: candidate.prevention || "", affected_paths: (candidate.affected_paths || []).join("\n"), evidence: (candidate.evidence || []).join("\n"), disposition: "policy", pinned: true };
}

export default function FailGuardPanel({ apiBase, fetcher, form, onError, onRefresh }) {
  const [candidates, setCandidates] = useState([]);
  const [guardrails, setGuardrails] = useState([]);
  const [candidateForm, setCandidateForm] = useState(() => ({ ...EMPTY_CANDIDATE, repo: form.repo || "" }));
  const [lessonForm, setLessonForm] = useState(() => ({ ...EMPTY_LESSON, repo: form.repo || "" }));
  const [selected, setSelected] = useState(null);
  const [review, setReview] = useState(null);
  const [dismissReason, setDismissReason] = useState("");
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const dashboard = useSavedDashboardViews({ storageKey: "repo-memory.v3.failguard", defaultView: { status: "open", source: "all", sort: "newest" } });

  async function load() {
    try {
      const status = dashboard.view.status || "open";
      const [candidateData, guardrailData] = await Promise.all([
        readJson(await fetcher(`${apiBase}/failguard/candidates?status=${encodeURIComponent(status)}`), "Could not load FailGuard candidates."),
        readJson(await fetcher(`${apiBase}/failguard/guardrails?status=active`), "Could not load FailGuard guardrails."),
      ]);
      setCandidates(candidateData.candidates || []);
      setGuardrails(guardrailData.guardrails || []);
    } catch (error) {
      onError(error.message || "RepoMemory could not load FailGuard candidates.");
    }
  }

  useEffect(() => { load(); }, [dashboard.view.status]);
  useEffect(() => { if (!form.repo) return; setCandidateForm((current) => current.repo ? current : { ...current, repo: form.repo }); setLessonForm((current) => current.repo ? current : { ...current, repo: form.repo }); }, [form.repo]);

  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const next = candidates.filter((candidate) => (dashboard.view.source === "all" || candidate.source_type === dashboard.view.source) && (!needle || `${candidate.repo} ${candidate.title} ${candidate.outcome} ${candidate.lesson} ${candidate.source_ref}`.toLowerCase().includes(needle)));
    return [...next].sort((left, right) => dashboard.view.sort === "confidence" ? (right.confidence || 0) - (left.confidence || 0) : dashboard.view.sort === "repo" ? left.repo.localeCompare(right.repo) : new Date(right.updated_at) - new Date(left.updated_at));
  }, [candidates, dashboard.view, query]);
  const sources = [...new Set(candidates.map((candidate) => candidate.source_type).filter(Boolean))].sort();
  const filters = [
    { key: "status", label: "Status", value: dashboard.view.status, options: [{ value: "open", label: "Open" }, { value: "promoted", label: "Promoted" }, { value: "dismissed", label: "Dismissed" }, { value: "all", label: "All" }] },
    { key: "source", label: "Source", value: dashboard.view.source, options: [{ value: "all", label: "All" }, ...sources.map((source) => ({ value: source, label: source }))] },
  ];

  function updateCandidate(key, value) { setCandidateForm((current) => ({ ...current, [key]: value })); }
  function updateLesson(key, value) { setLessonForm((current) => ({ ...current, [key]: value })); }
  function updateReview(key, value) { setReview((current) => ({ ...current, [key]: value })); }

  async function suggest() {
    setBusy(true); setMessage("");
    try {
      const data = await readJson(await fetcher(`${apiBase}/failguard/candidates`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ ...candidateForm, affected_paths: lines(candidateForm.affected_paths), evidence: lines(candidateForm.evidence) }) }), "Could not queue FailGuard candidate.");
      setMessage(data.message); setCandidateForm({ ...EMPTY_CANDIDATE, repo: candidateForm.repo }); await load(); onRefresh();
    } catch (error) { onError(error.message || "RepoMemory could not queue the candidate."); } finally { setBusy(false); }
  }

  async function promote() {
    if (!selected || !review) return;
    setBusy(true); setMessage("");
    try {
      const data = await readJson(await fetcher(`${apiBase}/failguard/candidates/${encodeURIComponent(selected.id)}/promote`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ ...review, affected_paths: lines(review.affected_paths), evidence: lines(review.evidence) }) }), "Could not promote FailGuard candidate.");
      setMessage(data.message); setSelected(null); setReview(null); await load(); onRefresh();
    } catch (error) { onError(error.message || "RepoMemory could not promote the candidate."); } finally { setBusy(false); }
  }

  async function dismiss() {
    if (!selected) return;
    setBusy(true); setMessage("");
    try {
      const data = await readJson(await fetcher(`${apiBase}/failguard/candidates/${encodeURIComponent(selected.id)}/dismiss`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ reason: dismissReason }) }), "Could not dismiss FailGuard candidate.");
      setMessage(data.message); setSelected(null); setReview(null); setDismissReason(""); await load(); onRefresh();
    } catch (error) { onError(error.message || "RepoMemory could not dismiss the candidate."); } finally { setBusy(false); }
  }

  async function captureLesson() {
    setBusy(true); setMessage("");
    try {
      const data = await readJson(await fetcher(`${apiBase}/failguard/lessons`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ ...lessonForm, affected_paths: lines(lessonForm.affected_paths), evidence: lines(lessonForm.evidence) }) }), "Could not capture FailGuard lesson.");
      setMessage(data.message); setLessonForm({ ...EMPTY_LESSON, repo: lessonForm.repo }); await load(); onRefresh();
    } catch (error) { onError(error.message || "RepoMemory could not capture the lesson."); } finally { setBusy(false); }
  }

  return <div className="space-y-6"><section className="surface p-6 sm:p-8"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><ShieldAlert size={12} /> Reviewable failure memory</div><h1 className={`mt-2 font-display text-[42px] font-semibold ${V3_TEXT.strong}`}>FailGuard.</h1><p className={`mt-3 max-w-3xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>Queue painful outcomes, review the proposed lesson, and promote only the guardrails that deserve durable policy weight.</p>{message ? <div className={`surface-inset mt-5 rounded-xl p-3 text-[12px] ${V3_TEXT.body}`}>{message}</div> : null}</section>
    <section className="surface p-5 sm:p-6"><details><summary className={`cursor-pointer font-display text-[22px] ${V3_TEXT.strong}`}>Suggest a candidate</summary><div className="mt-5 grid gap-4 sm:grid-cols-2"><Field label="Repository" onChange={(value) => updateCandidate("repo", value)} placeholder="owner/repo" value={candidateForm.repo} /><label className="block"><span className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>Source</span><div className="surface-inset mt-2 rounded-xl px-3"><select className={`h-12 w-full bg-transparent text-[12px] ${V3_TEXT.strong}`} onChange={(event) => updateCandidate("source_type", event.target.value)} value={candidateForm.source_type}><option value="operator">Operator</option><option value="trust-gate-block">TrustGate block</option><option value="trust-gate-warn">TrustGate warning</option><option value="repo-reaper-rejection">RepoReaper rejection</option><option value="reviewbee-thread">ReviewBee thread</option><option value="reverted-pr">Reverted PR</option></select></div></label><Field label="Source reference" onChange={(value) => updateCandidate("source_ref", value)} placeholder="run ID or PR URL" value={candidateForm.source_ref} /><Field label="Title" onChange={(value) => updateCandidate("title", value)} placeholder="What failed?" value={candidateForm.title} /><Field label="Bad outcome" onChange={(value) => updateCandidate("outcome", value)} rows={4} value={candidateForm.outcome} /><Field label="Draft lesson · optional" onChange={(value) => updateCandidate("lesson", value)} rows={4} value={candidateForm.lesson} /><Field label="Draft prevention · optional" onChange={(value) => updateCandidate("prevention", value)} rows={4} value={candidateForm.prevention} /><Field label="Affected paths · one per line" onChange={(value) => updateCandidate("affected_paths", value)} rows={4} value={candidateForm.affected_paths} /><Field full label="Evidence · one item per line" onChange={(value) => updateCandidate("evidence", value)} rows={4} value={candidateForm.evidence} /></div><div className="mt-4"><Action disabled={busy} onClick={suggest} primary><Sparkles size={12} /> Queue candidate</Action></div></details></section>
    <section className="surface p-5 sm:p-6"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Candidate queue</div><div className={`mt-1 font-display text-[24px] ${V3_TEXT.strong}`}>{visible.length} in view <span className={`${V3_TEXT.dim} font-normal`}>/ {candidates.length} loaded</span></div><div className="mt-5"><DashboardControls filters={filters} onApplySavedView={dashboard.applyView} onDeleteSavedView={dashboard.deleteView} onFilterChange={dashboard.updateView} onQueryChange={setQuery} onReset={() => { dashboard.resetView(); setQuery(""); }} onSaveView={dashboard.saveView} onSortChange={(sort) => dashboard.updateView("sort", sort)} query={query} savedViews={dashboard.savedViews} searchPlaceholder="Search repo, outcome, lesson, source…" sort={dashboard.view.sort} sortOptions={[{ value: "newest", label: "Newest first" }, { value: "confidence", label: "Highest confidence" }, { value: "repo", label: "Repository" }]} /></div><div className="mt-6"><ProgressiveList empty={<div className={`py-14 text-center text-[13px] ${V3_TEXT.mute}`}>No FailGuard candidates match this view.</div>} initialCount={6} itemLabel="candidates" items={visible} renderItem={(candidate) => <article className="surface-inset rounded-xl p-4" key={candidate.id}><div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between"><div><div className={`font-display text-[17px] ${V3_TEXT.strong}`}>{candidate.title}</div><div className={`mt-1 text-[10px] ${V3_TEXT.mute}`}>{candidate.repo} · {candidate.source_type}</div></div><div className="flex flex-wrap gap-2"><Badge status={candidate.status}>{candidate.status}</Badge><Badge>{Math.round(candidate.confidence || 0)}%</Badge>{(candidate.occurrence_count || 1) > 1 ? <Badge>{candidate.occurrence_count} occurrences</Badge> : null}</div></div><p className={`mt-3 text-[12px] leading-relaxed ${V3_TEXT.body}`}>{candidate.outcome}</p>{candidate.resolution_note ? <p className={`mt-2 text-[11px] ${V3_TEXT.mute}`}>{candidate.resolution_note}</p> : null}{candidate.status === "open" ? <div className="mt-4"><Action onClick={() => { setSelected(candidate); setReview(reviewFrom(candidate)); setDismissReason(""); }}>Review candidate</Action></div> : null}</article>} /></div></section>
    {selected && review ? <section className="surface p-5 sm:p-6"><div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between"><div><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Operator review</div><h2 className={`mt-1 font-display text-[28px] ${V3_TEXT.strong}`}>{selected.title}</h2></div><Badge>{selected.source_type}</Badge></div><div className="mt-5 grid gap-4 sm:grid-cols-2"><Field label="Title" onChange={(value) => updateReview("title", value)} value={review.title} /><Field label="Bad outcome" onChange={(value) => updateReview("outcome", value)} rows={4} value={review.outcome} /><Field label="Durable lesson" onChange={(value) => updateReview("lesson", value)} rows={4} value={review.lesson} /><Field label="Future prevention" onChange={(value) => updateReview("prevention", value)} rows={4} value={review.prevention} /><Field label="Affected paths" onChange={(value) => updateReview("affected_paths", value)} rows={4} value={review.affected_paths} /><Field label="Evidence" onChange={(value) => updateReview("evidence", value)} rows={4} value={review.evidence} /><label className="block"><span className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>Disposition</span><div className="surface-inset mt-2 rounded-xl px-3"><select className={`h-12 w-full bg-transparent text-[12px] ${V3_TEXT.strong}`} onChange={(event) => updateReview("disposition", event.target.value)} value={review.disposition}><option value="policy">Policy</option><option value="signal">Signal</option></select></div></label><label className={`surface-inset mt-5 flex h-12 items-center gap-3 rounded-xl px-4 text-[12px] ${V3_TEXT.body}`}><input checked={review.pinned} onChange={(event) => updateReview("pinned", event.target.checked)} type="checkbox" /> Pin promoted lesson</label><Field full label="Dismissal reason · optional" onChange={setDismissReason} value={dismissReason} /></div><div className="mt-5 flex flex-wrap gap-2"><Action disabled={busy} onClick={promote} primary><CheckCircle2 size={12} /> Promote and compile guardrails</Action><Action danger disabled={busy} onClick={dismiss}><ArchiveX size={12} /> Dismiss candidate</Action><Action onClick={() => { setSelected(null); setReview(null); }}>Cancel</Action></div></section> : null}
    <section className="surface p-5 sm:p-6"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Active guardrails</div><div className={`mt-1 font-display text-[24px] ${V3_TEXT.strong}`}>{guardrails.length} compiled</div><div className="mt-5"><ProgressiveList empty={<div className={`py-10 text-center text-[13px] ${V3_TEXT.mute}`}>Promoted lessons will compile into product-specific guardrails here.</div>} initialCount={4} itemLabel="guardrails" items={guardrails} renderItem={(guardrail) => <article className="surface-inset rounded-xl p-4" key={guardrail.id}><div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between"><div><div className={`font-display text-[17px] ${V3_TEXT.strong}`}>{guardrail.title}</div><div className={`mt-1 text-[10px] ${V3_TEXT.mute}`}>{guardrail.repo} · {guardrail.affected_paths?.length ? guardrail.affected_paths.join(", ") : "repo-wide"}</div></div><div className="flex gap-2"><Badge status="promoted">active</Badge><Badge>{guardrail.match_count || 0} {(guardrail.match_count || 0) === 1 ? "match" : "matches"}</Badge></div></div><p className={`mt-3 text-[12px] leading-relaxed ${V3_TEXT.body}`}>{guardrail.prevention}</p><div className="mt-3 flex flex-wrap gap-2">{(guardrail.suggestions || []).map((suggestion) => <Badge key={`${guardrail.id}-${suggestion.consumer}`}>{suggestion.consumer}</Badge>)}</div></article>} /></div></section>
    <section className="surface p-5 sm:p-6"><details><summary className={`cursor-pointer font-display text-[22px] ${V3_TEXT.strong}`}>Capture a reviewed lesson directly</summary><p className={`mt-2 text-[11px] ${V3_TEXT.mute}`}>Use this path only when the lesson has already been reviewed and should bypass the candidate queue.</p><div className="mt-5 grid gap-4 sm:grid-cols-2"><Field label="Repository" onChange={(value) => updateLesson("repo", value)} placeholder="owner/repo" value={lessonForm.repo} /><Field label="Title" onChange={(value) => updateLesson("title", value)} value={lessonForm.title} /><Field label="Bad outcome" onChange={(value) => updateLesson("outcome", value)} rows={4} value={lessonForm.outcome} /><Field label="Durable lesson" onChange={(value) => updateLesson("lesson", value)} rows={4} value={lessonForm.lesson} /><Field label="Future prevention" onChange={(value) => updateLesson("prevention", value)} rows={4} value={lessonForm.prevention} /><Field label="Affected paths" onChange={(value) => updateLesson("affected_paths", value)} rows={4} value={lessonForm.affected_paths} /><Field full label="Evidence" onChange={(value) => updateLesson("evidence", value)} rows={4} value={lessonForm.evidence} /><label className="block"><span className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>Disposition</span><div className="surface-inset mt-2 rounded-xl px-3"><select className={`h-12 w-full bg-transparent text-[12px] ${V3_TEXT.strong}`} onChange={(event) => updateLesson("disposition", event.target.value)} value={lessonForm.disposition}><option value="policy">Policy</option><option value="signal">Signal</option></select></div></label><label className={`surface-inset mt-5 flex h-12 items-center gap-3 rounded-xl px-4 text-[12px] ${V3_TEXT.body}`}><input checked={lessonForm.pinned} onChange={(event) => updateLesson("pinned", event.target.checked)} type="checkbox" /> Pin captured lesson</label></div><div className="mt-4"><Action disabled={busy} onClick={captureLesson} primary>Capture reviewed memory</Action></div></details></section>
    <GuidanceNotice label="Promotion boundary">Products submit bad outcomes automatically when configured. FailGuard correlates repeated candidates, but only an operator-edited promotion creates durable memory and active TrustGate, RepoReaper, MergeKeeper, and ReleaseSentry suggestions. Every later match is recorded.</GuidanceNotice>
  </div>;
}
