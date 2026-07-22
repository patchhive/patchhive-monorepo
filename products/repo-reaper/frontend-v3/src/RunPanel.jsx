import { useMemo } from "react";
import { Activity, Bot, ClipboardCopy, Crosshair, Radio, ShieldCheck, Target } from "lucide-react";
import {
  CopyMarkdownButton,
  DashboardControls,
  GuidanceNotice,
  ProgressiveList,
  V3_TEXT,
  useSavedDashboardViews,
} from "@patchhivehq/ui-v3";
import { Chip, Fact, money, normalizeCollection, serializeRunParams, statusTone } from "./shared.jsx";

const TARGET_MODES = [
  { value: "direct", label: "Target repo" },
  { value: "discovery", label: "Autonomous discovery" },
];

function Field({ help, label, onChange, type = "text", value, ...props }) {
  return <label className="block"><span className={`text-[10px] uppercase tracking-[0.2em] ${V3_TEXT.mute}`}>{label}</span><input className={`surface-inset mt-2 h-11 w-full rounded-xl bg-transparent px-4 text-[13px] outline-none ${V3_TEXT.strong}`} onChange={(event) => onChange(event.target.value)} type={type} value={value} {...props}/>{help ? <span className={`mt-1.5 block text-[10px] leading-relaxed ${V3_TEXT.mute}`}>{help}</span> : null}</label>;
}

function Select({ label, onChange, options, value }) {
  return <label className="block"><span className={`text-[10px] uppercase tracking-[0.2em] ${V3_TEXT.mute}`}>{label}</span><select className={`surface-inset mt-2 h-11 w-full rounded-xl bg-transparent px-4 text-[13px] outline-none ${V3_TEXT.strong}`} onChange={(event) => onChange(event.target.value)} value={value}>{options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}</select></label>;
}

function reportMarkdown({ dry, stream, targetSelectionMode }) {
  const lines = [
    `# RepoReaper ${dry ? "Dry Stalk" : "Mission"}`,
    "",
    `- Target mode: ${targetSelectionMode === "direct" ? "Target repo" : "Autonomous discovery"}`,
    `- Phase: ${stream.phase || "idle"}`,
    `- Candidates: ${normalizeCollection(stream.issues).length}`,
    `- Run cost: ${money(stream.runCost)}`,
  ];
  if (stream.done?.run_id) lines.push(`- Run: ${stream.done.run_id}`);
  if (stream.report) lines.push("", "## Scout report", "", "```json", JSON.stringify(stream.report, null, 2), "```");
  if (stream.logs.length) {
    lines.push("", "## Recent evidence", "");
    stream.logs.slice(-20).forEach((entry) => lines.push(`- ${entry.msg || JSON.stringify(entry)}`));
  }
  lines.push("", "*RepoReaper by [PatchHive](https://github.com/patchhive)*");
  return lines.join("\n");
}

export default function RunPanel({
  agents,
  dry = false,
  health,
  onParamsChange,
  onStart,
  params,
  stream,
  targetSelectionMode,
  onTargetSelectionModeChange,
}) {
  const items = normalizeCollection(stream.issues);
  const views = useSavedDashboardViews({
    storageKey: `repo-reaper.${dry ? "dry" : "mission"}.queue`,
    defaultView: { query: "", status: "all", sort: "score" },
  });
  const filtered = useMemo(() => {
    const query = views.view.query.toLowerCase();
    return [...items].filter((item) => {
      const text = [item.title, item.issue_title, item.repo, item.reason, item.feedback].filter(Boolean).join(" ").toLowerCase();
      return (!query || text.includes(query)) && (views.view.status === "all" || String(item.status || "queued") === views.view.status);
    }).sort((left, right) => {
      if (views.view.sort === "repo") return String(left.repo || "").localeCompare(String(right.repo || ""));
      if (views.view.sort === "status") return String(left.status || "").localeCompare(String(right.status || ""));
      return Number(right.fixability_score || right.score || 0) - Number(left.fixability_score || left.score || 0);
    });
  }, [items, views.view]);
  const roles = new Set(agents.map((agent) => agent.role));
  const ready = dry ? roles.has("scout") : ["scout", "judge", "reaper", "smith", "gatekeeper"].every((role) => roles.has(role));
  const validTarget = targetSelectionMode === "direct" ? /^[^/\s]+\/[^/\s]+$/.test(params.target_repo.trim()) : true;
  const set = (key) => (value) => onParamsChange((current) => ({ ...current, [key]: value }));
  const priority = items[0];

  return <div className="mx-auto max-w-[1440px] space-y-6 px-3 py-6 sm:px-6">
    <section className="surface grid gap-6 p-6 lg:grid-cols-[1.6fr_0.9fr] lg:p-8">
      <div>
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>{dry ? <Crosshair size={12}/> : <Radio size={12}/>} {dry ? "No-write reconnaissance" : "Autonomous patch mission"}</div>
        <h1 className={`mt-3 max-w-4xl font-display text-[42px] font-semibold leading-[1.05] ${V3_TEXT.strong}`}>{stream.running ? `${dry ? "Dry Stalk" : "RepoReaper"} is ${stream.phase || "working"}.` : dry ? "Find the work before touching it." : "Find, validate, and deliver bounded fixes."}</h1>
        <p className={`mt-4 max-w-3xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>{dry ? "Discovers and scores candidate issues with the Scout, saves the evidence, and performs no repository writes." : "Runs the complete Scout → Judge → Reaper → Smith → Gatekeeper pipeline. Every PR remains policy-, validation-, capacity-, and attribution-gated."}</p>
        <div className="mt-5 flex flex-wrap gap-2"><Chip tone={health.github_ready ? "ok" : "hot"}>GitHub {health.github_ready ? "verified" : "not ready"}</Chip><Chip tone={ready ? "ok" : "warn"}>{agents.length} agents · {ready ? "roles ready" : "setup needed"}</Chip><Chip tone={health.run_active ? "warn" : "ok"}>{health.run_active ? "operation active" : "engine idle"}</Chip><Chip>{targetSelectionMode === "direct" ? "target repo" : "autonomous discovery"}</Chip></div>
      </div>
      <aside className="surface-inset rounded-2xl p-5">
        <div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Current assessment</div>
        <div className={`mt-3 font-display text-[30px] font-semibold ${V3_TEXT.strong}`}>{stream.running ? stream.phase : priority ? "candidate evidence" : ready ? "ready to scope" : "squad setup"}</div>
        <p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.body}`}>{priority ? `${priority.repo || "Repository"} · ${priority.title || priority.issue_title || "candidate issue"}` : ready ? "Choose an explicit target mode and bounded scope." : "Configure the required agent roles before starting."}</p>
        <div className="mt-5 grid grid-cols-3 gap-2"><Fact label="Candidates" value={items.length}/><Fact label="Cost" value={money(stream.runCost)}/><Fact label="Phase" value={stream.phase || "idle"}/></div>
      </aside>
    </section>

    <section className="surface p-6">
      <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Target size={12}/> Mission scope</div>
      <div className="mt-5 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <Select label="Target mode" onChange={onTargetSelectionModeChange} options={TARGET_MODES} value={targetSelectionMode}/>
        {targetSelectionMode === "direct" ? <Field help="Required in Target repo mode." label="Repository" onChange={set("target_repo")} placeholder="owner/repository" value={params.target_repo}/> : <Field help="Optional GitHub search override." label="Discovery query" onChange={set("search_query")} placeholder="topic:rust is:public" value={params.search_query}/>} 
        <Field label="Language" onChange={set("language")} value={params.language}/>
        <Field label="Issue labels" onChange={set("labels")} value={params.labels}/>
        <Field label="Minimum stars" min="0" onChange={set("min_stars")} type="number" value={params.min_stars}/>
        <Field label="Repository cap" min="1" max="100" onChange={set("max_repos")} type="number" value={params.max_repos}/>
        <Field label="Issue cap" min="1" max="100" onChange={set("max_issues")} type="number" value={params.max_issues}/>
        {!dry ? <Field label="Concurrent fixes" min="1" max="32" onChange={set("concurrency")} type="number" value={params.concurrency}/> : null}
        <Field label="Retry count" min="0" max="10" onChange={set("retry_count")} type="number" value={params.retry_count}/>
        <Field label="Cost budget USD" min="0" step="0.01" onChange={set("cost_budget_usd")} type="number" value={params.cost_budget_usd}/>
      </div>
      <div className="mt-5 flex flex-wrap items-center gap-3">
        <button className="h-11 rounded-full px-5 text-[12px] font-semibold text-white disabled:opacity-40" disabled={stream.running || !ready || !validTarget} onClick={() => onStart(serializeRunParams(params, targetSelectionMode))} style={{ backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))", boxShadow: "var(--accent-glow)" }} type="button">{stream.running ? `${stream.phase || "Operation"}…` : dry ? "Run Dry Stalk" : "Launch mission"}</button>
        <CopyMarkdownButton content={reportMarkdown({ dry, stream, targetSelectionMode })} label="Copy evidence Markdown"/>
      </div>
      {!ready ? <GuidanceNotice label="Squad not ready">{dry ? "Dry Stalk needs at least one Scout agent." : "A write mission needs Scout, Judge, Reaper, Smith, and Gatekeeper roles."} Configure them in Squad.</GuidanceNotice> : null}
      {!validTarget ? <GuidanceNotice label="Target required">Enter a repository in owner/repository format. RepoReaper will not fall through to autonomous discovery.</GuidanceNotice> : null}
      {!dry ? <GuidanceNotice label="Write boundary">Launching authorizes this bounded mission, not unrestricted GitHub writes. Repo policy, existing-PR detection, trusted-test policy, Smith confidence, validation status, and HiveCore PR budgets still decide whether a PR may be opened.</GuidanceNotice> : null}
    </section>

    <section className="surface p-6">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-end sm:justify-between"><div><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Activity size={12}/> Live candidate queue</div><h2 className={`mt-2 font-display text-[28px] font-semibold ${V3_TEXT.strong}`}>{filtered.length} in view / {items.length} tracked</h2></div></div>
      <div className="mt-5"><DashboardControls filters={[{ key: "status", label: "Status", value: views.view.status, options: [{ value: "all", label: "All" }, ...[...new Set(items.map((item) => String(item.status || "queued")))].map((value) => ({ value, label: value }))] }]} onApplySavedView={views.applyView} onDeleteSavedView={views.deleteView} onFilterChange={views.updateView} onQueryChange={(value) => views.updateView("query", value)} onReset={views.resetView} onSaveView={views.saveView} onSortChange={(value) => views.updateView("sort", value)} query={views.view.query} savedViews={views.savedViews} searchPlaceholder="Search repository, issue, status, feedback…" sort={views.view.sort} sortOptions={[{ value: "score", label: "Highest score" }, { value: "status", label: "Status" }, { value: "repo", label: "Repository" }]}/></div>
      <div className="mt-5"><ProgressiveList initialCount={6} batchCount={24} itemLabel="candidates" items={filtered} empty={<div className={`surface-inset rounded-xl p-8 text-center text-[12px] ${V3_TEXT.mute}`}>No candidate evidence in this view.</div>} renderItem={(item, index) => <article className="surface-inset rounded-xl p-4" key={item.id || item.issue_url || index}><div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between"><div><h3 className={`font-display text-[17px] font-semibold ${V3_TEXT.strong}`}>{item.title || item.issue_title || "Candidate issue"}</h3><p className={`mt-1 text-[11px] ${V3_TEXT.mute}`}>{item.repo || "repository pending"}{item.issue_number ? ` · issue #${item.issue_number}` : ""}</p><p className={`mt-2 text-[12px] leading-relaxed ${V3_TEXT.body}`}>{item.reason || item.feedback || item.fixability_reason || "Candidate is waiting for the next pipeline decision."}</p></div><div className="flex shrink-0 gap-2"><Chip tone={statusTone(item.status)}>{item.status || "queued"}</Chip><Chip>{item.fixability_score ?? item.score ?? 0} score</Chip></div></div>{item.pr_url ? <a className="mt-3 inline-block text-[11px] underline" href={item.pr_url} rel="noreferrer" target="_blank">Open PR #{item.pr_number || ""}</a> : null}</article>}/></div>
    </section>

    <section className="surface p-6">
      <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Bot size={12}/> Live agent evidence</div>
      <div className="mt-4 grid gap-3 lg:grid-cols-[0.8fr_1.2fr]"><div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-1">{agents.map((agent) => { const live = stream.agentStatuses[agent.id] || {}; return <div className="surface-inset rounded-xl p-3" key={agent.id}><div className="flex items-center justify-between gap-2"><span className={`font-display text-[14px] ${V3_TEXT.strong}`}>{agent.name}</span><Chip tone={statusTone(live.status || agent.status)}>{live.status || agent.status || "idle"}</Chip></div><div className={`mt-1 text-[10px] ${V3_TEXT.mute}`}>{agent.role} · {agent.provider}/{agent.model}</div>{live.task ? <div className={`mt-2 text-[11px] ${V3_TEXT.body}`}>{live.task}</div> : null}</div>; })}</div><div><ProgressiveList initialCount={8} batchCount={30} itemLabel="events" items={stream.logs} empty={<div className={`surface-inset rounded-xl p-8 text-center text-[12px] ${V3_TEXT.mute}`}>Live logs appear here after the operation starts.</div>} renderItem={(entry, index) => <div className="surface-inset rounded-xl p-3" key={`${entry.ts || "log"}-${index}`}><div className="flex items-start gap-3"><ClipboardCopy className={V3_TEXT.dim} size={13}/><div><div className={`text-[11px] leading-relaxed ${V3_TEXT.body}`}>{entry.msg || JSON.stringify(entry)}</div><div className={`mt-1 text-[9px] uppercase tracking-wider ${V3_TEXT.mute}`}>{entry.agent || entry.role || entry.type || "RepoReaper"}</div></div></div></div>}/></div></div>
      {stream.report ? <div className="surface-inset mt-4 rounded-xl p-4"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}><ShieldCheck size={12}/> Scout report</div><pre className={`mt-3 overflow-x-auto whitespace-pre-wrap text-[11px] leading-relaxed ${V3_TEXT.body}`}>{JSON.stringify(stream.report, null, 2)}</pre></div> : null}
    </section>
  </div>;
}
