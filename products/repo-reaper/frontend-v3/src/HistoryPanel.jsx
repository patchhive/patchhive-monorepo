import { useMemo } from "react";
import { Ban, BrainCircuit, ExternalLink, ShieldCheck } from "lucide-react";
import {
  ActivityTimeline,
  CopyMarkdownButton,
  HistoryDashboard,
  ProgressiveList,
  V3_TEXT,
  useSavedDashboardViews,
} from "@patchhivehq/ui-v3";
import { Chip, Fact, formatDate, money, statusTone } from "./shared.jsx";
import ScoutReport, { scoutReportMarkdown } from "./ScoutReport.jsx";

function runMarkdown(run) {
  if (!run) return "";
  const lines = [`# RepoReaper run ${run.id}`, "", `- Status: ${run.status}`, `- Mode: ${run.dry_run ? "Dry Stalk" : "Patch mission"}`, `- Target: ${run.target_repo || "Autonomous discovery"}`, `- Fixed / attempted: ${run.total_fixed || 0} / ${run.total_attempted || 0}`, `- Cost: ${money(run.total_cost_usd)}`];
  if (run.attempts?.length) {
    lines.push("", "## Attempts", "");
    run.attempts.forEach((attempt) => lines.push(`- ${attempt.repo || "repository"}#${attempt.issue_number}: ${attempt.status} — ${attempt.skip_reason || attempt.error_msg || attempt.pr_url || "saved evidence"}`));
  }
  if (run.dry_stalk?.report) lines.push("", scoutReportMarkdown(run.dry_stalk.report));
  lines.push("", "*RepoReaper by [PatchHive](https://github.com/patchhive)*");
  return lines.join("\n");
}

export default function HistoryPanel({ history, leaderboard, loading, onLoadRun, onRefresh, rejected, selectedRun }) {
  const dashboard = useSavedDashboardViews({ storageKey: "repo-reaper.history", defaultView: { query: "", mode: "all", status: "all", sort: "newest" } });
  const items = useMemo(() => {
    const query = dashboard.view.query.toLowerCase();
    return [...history].filter((run) => {
      const text = [run.id, run.status, run.target_repo, ...(run.attempts || []).flatMap((attempt) => [attempt.repo, attempt.issue_title, attempt.error_msg])].filter(Boolean).join(" ").toLowerCase();
      const mode = run.dry_run ? "dry" : "write";
      return (!query || text.includes(query)) && (dashboard.view.mode === "all" || dashboard.view.mode === mode) && (dashboard.view.status === "all" || run.status === dashboard.view.status);
    }).sort((left, right) => {
      if (dashboard.view.sort === "oldest") return new Date(left.started_at) - new Date(right.started_at);
      if (dashboard.view.sort === "cost") return Number(right.total_cost_usd || 0) - Number(left.total_cost_usd || 0);
      if (dashboard.view.sort === "attempts") return Number(right.total_attempted || 0) - Number(left.total_attempted || 0);
      return new Date(right.started_at) - new Date(left.started_at);
    });
  }, [dashboard.view, history]);
  const statuses = [...new Set(history.map((run) => run.status).filter(Boolean))];
  const events = (selectedRun?.events || []).map((event, index) => ({ id: event.id || `${event.source}-${index}`, kind: event.phase || event.source || "status", at: event.occurred_at || event.created_at || selectedRun.started_at, message: event.message || event.summary || "Saved run event", actor: event.actor || "RepoReaper" }));
  const eventTypes = [...new Set(events.map((event) => event.kind))];

  return <div className="mx-auto max-w-[1440px] space-y-6 px-3 py-6 sm:px-6">
    <HistoryDashboard dashboard={dashboard} filters={[{ key: "mode", label: "Mode", value: dashboard.view.mode, options: [{ value: "all", label: "All" }, { value: "write", label: "Patch missions" }, { value: "dry", label: "Dry Stalk" }] }, { key: "status", label: "Status", value: dashboard.view.status, options: [{ value: "all", label: "All" }, ...statuses.map((status) => ({ value: status, label: status }))] }]} items={items} loading={loading} onQueryChange={(value) => dashboard.updateView("query", value)} onRefresh={onRefresh} query={dashboard.view.query} renderItem={(run) => <button className="surface-inset w-full rounded-xl p-4 text-left" key={run.id} onClick={() => onLoadRun(run.id)} type="button"><div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between"><div><div className={`font-display text-[17px] font-semibold ${V3_TEXT.strong}`}>{run.target_repo || (run.dry_run ? "Autonomous Dry Stalk" : "Autonomous patch mission")}</div><p className={`mt-1 text-[11px] ${V3_TEXT.mute}`}>{formatDate(run.started_at)} · run {run.id} · {run.total_fixed || 0}/{run.total_attempted || 0} fixed · {money(run.total_cost_usd)}</p></div><div className="flex gap-2"><Chip tone={statusTone(run.status)}>{run.status}</Chip><Chip>{run.dry_run ? "dry stalk" : "write"}</Chip></div></div></button>} searchPlaceholder="Search repository, issue, run, failure…" sortOptions={[{ value: "newest", label: "Newest first" }, { value: "oldest", label: "Oldest first" }, { value: "cost", label: "Highest cost" }, { value: "attempts", label: "Most attempts" }]} totalCount={history.length}/>

    {selectedRun ? <>
      <section className="surface p-6">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between"><div><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Run dossier</div><h2 className={`mt-2 font-display text-[30px] font-semibold ${V3_TEXT.strong}`}>{selectedRun.target_repo || "Autonomous discovery"}</h2><p className={`mt-2 text-[12px] ${V3_TEXT.body}`}>Run {selectedRun.id} · {selectedRun.dry_run ? "Dry Stalk / no writes" : "Guarded patch mission"} · {formatDate(selectedRun.started_at)}</p></div><CopyMarkdownButton content={runMarkdown(selectedRun)} label="Copy run Markdown"/></div>
        <div className="mt-5 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6"><Fact label="Status" value={selectedRun.status}/><Fact label="Fixed" value={selectedRun.total_fixed || 0}/><Fact label="Attempted" value={selectedRun.total_attempted || 0}/><Fact label="Cost" value={money(selectedRun.total_cost_usd)}/><Fact label="Started" value={formatDate(selectedRun.started_at)}/><Fact label="Finished" value={formatDate(selectedRun.finished_at)}/></div>
      </section>
      <ActivityTimeline caption={`Run ${selectedRun.id}`} eventTypes={eventTypes} events={events}/>
      <ScoutReport report={selectedRun.dry_stalk?.report}/>
      <section className="surface p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><ShieldCheck size={12}/> Issue attempts</div><div className="mt-5"><ProgressiveList initialCount={6} batchCount={30} itemLabel="attempts" items={selectedRun.attempts || []} empty={<div className={`surface-inset rounded-xl p-8 text-center text-[12px] ${V3_TEXT.mute}`}>No write attempts were recorded for this run.</div>} renderItem={(attempt) => <article className="surface-inset rounded-xl p-4" key={attempt.id}><div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between"><div><h3 className={`font-display text-[17px] font-semibold ${V3_TEXT.strong}`}>{attempt.issue_title || `Issue #${attempt.issue_number}`}</h3><p className={`mt-1 text-[11px] ${V3_TEXT.mute}`}>{attempt.repo} · #{attempt.issue_number} · {attempt.reaper_agent} / {attempt.smith_agent || "no Smith"} / {attempt.gatekeeper_agent}</p><p className={`mt-2 text-[12px] leading-relaxed ${V3_TEXT.body}`}>{attempt.skip_reason || attempt.error_msg || (attempt.pr_url ? "Pull request delivered." : "Saved attempt evidence.")}</p></div><div className="flex shrink-0 gap-2"><Chip tone={statusTone(attempt.status)}>{attempt.status}</Chip><Chip>{attempt.confidence || 0}% confidence</Chip><Chip>{money(attempt.cost_usd)}</Chip></div></div><div className="mt-3 flex flex-wrap gap-3">{attempt.issue_url ? <a className="inline-flex items-center gap-1 text-[11px] underline" href={attempt.issue_url} rel="noreferrer" target="_blank">Issue <ExternalLink size={11}/></a> : null}{attempt.pr_url ? <a className="inline-flex items-center gap-1 text-[11px] underline" href={attempt.pr_url} rel="noreferrer" target="_blank">PR #{attempt.pr_number} <ExternalLink size={11}/></a> : null}{attempt.patch_diff ? <details className="w-full"><summary className={`cursor-pointer text-[11px] ${V3_TEXT.body}`}>Patch diff</summary><pre className={`surface mt-3 max-h-96 overflow-auto whitespace-pre-wrap p-4 text-[10px] ${V3_TEXT.body}`}>{attempt.patch_diff}</pre></details> : null}</div></article>}/></div></section>
    </> : null}

    <section className="grid gap-6 lg:grid-cols-2"><article className="surface p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Ban size={12}/> Rejected patches</div><div className="mt-4"><ProgressiveList initialCount={4} batchCount={20} itemLabel="rejections" items={rejected} empty={<div className={`surface-inset rounded-xl p-6 text-center text-[12px] ${V3_TEXT.mute}`}>No rejected patches are stored.</div>} renderItem={(item) => <div className="surface-inset rounded-xl p-3" key={item.id}><div className="flex items-center justify-between gap-2"><span className={`font-display text-[14px] ${V3_TEXT.strong}`}>{item.repo} #{item.issue_number}</span><Chip tone="warn">{item.confidence}%</Chip></div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.body}`}>{item.smith_feedback || item.reason}</p></div>}/></div></article><article className="surface p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><BrainCircuit size={12}/> Agent outcomes</div><div className="mt-4"><ProgressiveList initialCount={5} batchCount={20} itemLabel="agents" items={leaderboard} empty={<div className={`surface-inset rounded-xl p-6 text-center text-[12px] ${V3_TEXT.mute}`}>Agent outcome evidence appears after write attempts.</div>} renderItem={(item, index) => <div className="surface-inset rounded-xl p-3" key={`${item.agent_name}-${item.model}-${index}`}><div className="flex items-center justify-between gap-2"><span className={`font-display text-[14px] ${V3_TEXT.strong}`}>{item.agent_name}</span><Chip tone={item.total_errors ? "warn" : "ok"}>{item.fix_rate}% fixed</Chip></div><p className={`mt-1 text-[10px] ${V3_TEXT.mute}`}>{item.role} · {item.provider}/{item.model} · {money(item.total_cost_usd)}</p></div>}/></div></article></section>
  </div>;
}
