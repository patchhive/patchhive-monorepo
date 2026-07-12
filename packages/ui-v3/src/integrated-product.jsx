import { useEffect, useMemo, useRef, useState } from "react";
import {
  Activity,
  ArrowLeft,
  ArrowUpRight,
  Clock,
  Cpu,
  ExternalLink,
  Github,
  KeyRound,
  Search,
  Sparkles,
  Zap,
} from "lucide-react";
import { MetricCard, ProductHeader, ProductShell, ThemeToggle, V3_TEXT } from "./index.jsx";
import { DashboardControls, HistoryDashboard, ProgressiveList, StartupCheckList, useSavedDashboardViews } from "./workspace.jsx";

const STATUS_CLASSES = {
  hot: "bg-red-900/10 text-red-800 border-red-900/30 dark:bg-red-500/10 dark:text-red-300 dark:border-red-400/25",
  warn: "bg-amber-900/10 text-amber-800 border-amber-900/30 dark:bg-amber-500/10 dark:text-amber-300 dark:border-amber-400/25",
  ok: "bg-emerald-900/10 text-emerald-800 border-emerald-900/30 dark:bg-emerald-500/10 dark:text-emerald-300 dark:border-emerald-400/25",
  neutral: "bg-stone-800/10 text-stone-800 border-stone-800/30 dark:bg-stone-500/10 dark:text-stone-300 dark:border-stone-400/25",
};

const SCORE_CLASSES = {
  hot: "from-red-800 to-orange-700 text-white",
  warn: "from-orange-700 to-amber-600 text-white",
  ok: "from-emerald-700 to-teal-700 text-white",
  neutral: "from-slate-600 to-slate-800 text-white",
};

function parseError(data, fallback) {
  return data?.error || data?.message || fallback;
}

async function readJson(response) {
  const data = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(parseError(data, `Request failed: ${response.status}`));
  return data;
}

function timeAgo(value) {
  if (!value) return "never";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return String(value);
  const seconds = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 48) return `${hours}h`;
  return `${Math.floor(hours / 24)}d`;
}

function countLabel(value, noun, plural = `${noun}s`) {
  const count = Number(value || 0);
  return `${count} ${count === 1 ? noun : plural}`;
}

function tone(value) {
  return STATUS_CLASSES[value] ? value : "neutral";
}

export function ProductLoginScreen({ apiBase, auth, config }) {
  const [key, setKey] = useState("");
  const [error, setError] = useState(auth.authError || "");
  const [busy, setBusy] = useState(false);
  const Icon = config.icon;

  async function submit(event) {
    event.preventDefault();
    if (!key.trim()) return;
    setBusy(true);
    setError("");
    try {
      await readJson(await fetch(`${apiBase}/auth/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ api_key: key.trim() }),
      }));
      auth.login(key.trim());
    } catch (err) {
      setError(err.message || "Invalid API key.");
    } finally {
      setBusy(false);
    }
  }

  async function generate() {
    setBusy(true);
    setError("");
    try {
      setKey(await auth.generateKey({ autoLogin: false }));
    } catch (err) {
      setError(err.message || "Could not generate an API key.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <ProductShell productKey={config.productKey}>
      <div className="min-h-[calc(100vh-80px)] grid place-items-center px-6 py-20">
        <section className="surface w-full max-w-lg p-8 overflow-hidden">
          <div className="absolute -top-20 -right-16 h-56 w-56 rounded-full opacity-40 blur-2xl" style={{ backgroundImage: "var(--orb-1)" }} />
          <div className="relative">
            <div className="flex items-center justify-between">
              <div className="h-12 w-12 rounded-xl grid place-items-center text-white" style={{ backgroundImage: "linear-gradient(135deg, var(--accent-2), var(--accent-3))", boxShadow: "var(--accent-glow)" }}><Icon size={20} /></div>
              <ThemeToggle />
            </div>
            <div className={`mt-7 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>PatchHive · Secure product session</div>
            <h1 className={`font-display mt-2 text-[38px] leading-tight tracking-[-0.03em] font-semibold ${V3_TEXT.strong}`}>Open {config.name}.</h1>
            <p className={`mt-3 text-[14px] leading-relaxed ${V3_TEXT.body}`}>Connect this workspace to the unified PatchHive backend.</p>
            <form className="mt-7 space-y-3" onSubmit={submit}>
              <label className={`block text-[10px] uppercase tracking-[0.2em] ${V3_TEXT.mute}`} htmlFor="api-key">API key</label>
              <div className="surface-inset h-11 rounded-xl px-3 flex items-center gap-2"><KeyRound size={14} className={V3_TEXT.dim} /><input id="api-key" value={key} onChange={(event) => setKey(event.target.value)} className={`bg-transparent outline-none w-full text-[13px] ${V3_TEXT.strong}`} type="password" autoComplete="current-password" /></div>
              {error ? <p className="text-[12px] text-red-700 dark:text-red-300">{error}</p> : null}
              <div className="flex gap-2 pt-2">
                <button disabled={busy || !key.trim()} className="h-10 flex-1 rounded-full text-[12px] font-semibold text-white disabled:opacity-50" style={{ backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))" }} type="submit">{busy ? "Connecting…" : "Connect"}</button>
                {auth.bootstrapRequired ? <button disabled={busy} onClick={generate} className={`surface-inset h-10 px-4 rounded-full text-[12px] ${V3_TEXT.body}`} type="button">Generate key</button> : null}
              </div>
            </form>
          </div>
        </section>
      </div>
    </ProductShell>
  );
}

function ItemRow({ item, onOpen }) {
  const itemTone = tone(item.tone);
  return (
    <button type="button" onClick={() => onOpen(item)} className="surface-inset group rounded-xl p-4 hover:brightness-110 hover:shadow-[0_10px_30px_-15px_rgba(15,23,42,0.35)] w-full text-left">
      <div className="grid grid-cols-[auto_1fr] sm:grid-cols-[auto_1fr_auto] gap-4 items-center">
        <div className={`h-12 w-12 rounded-xl bg-gradient-to-br ${SCORE_CLASSES[itemTone]} grid place-items-center shadow-inner`}><span className="font-display font-semibold text-[15px] tabular-nums">{item.score ?? "—"}</span></div>
        <div className="min-w-0">
          <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.2em] ${V3_TEXT.mute}`}><span className="truncate">{item.id}</span>{item.source ? <><span className="opacity-40">·</span><span>{item.source}</span></> : null}</div>
          <div className={`mt-1 font-display font-medium text-[16px] tracking-tight ${V3_TEXT.strong} truncate`}>{item.title}</div>
          <div className={`mt-1 text-[12px] ${V3_TEXT.mute} font-mono truncate`}>{item.meta}</div>
        </div>
        <div className="col-span-2 sm:col-span-1 flex items-center justify-end gap-2"><span className={`text-[10px] uppercase tracking-widest px-2.5 py-1 rounded-full border ${STATUS_CLASSES[itemTone]}`}>{item.status}</span><ArrowUpRight size={14} className={V3_TEXT.dim} /></div>
      </div>
    </button>
  );
}

function Detail({ item, config, onBack }) {
  const itemTone = tone(item.tone);
  const links = item.links?.length ? item.links : item.link ? [{ label: "Open source evidence", url: item.link }] : [];
  const evidence = [...new Set((item.evidence || []).map((entry) => String(entry)))];
  return (
    <>
      <header className="px-3 sm:px-6 pt-3 sm:pt-6"><div className="surface mx-auto max-w-[1200px] px-5 min-h-16 flex items-center justify-between"><button onClick={onBack} className={`flex items-center gap-2 text-[12px] ${V3_TEXT.body}`} type="button"><ArrowLeft size={14} /> Back to queue</button><div className={`hidden sm:block text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>PatchHive · Evidence detail</div><ThemeToggle /></div></header>
      <div className="mx-auto max-w-[1200px] px-3 sm:px-6 pt-8 pb-24 grid grid-cols-12 gap-6">
        <section className="col-span-12 lg:col-span-8 space-y-6">
          <div className="surface p-6 sm:p-8"><div className="flex items-start gap-5"><div className={`h-16 w-16 rounded-2xl bg-gradient-to-br ${SCORE_CLASSES[itemTone]} grid place-items-center shadow-inner shrink-0`}><div className="font-display font-semibold text-[18px]">{item.score ?? "—"}</div></div><div><div className={`text-[10px] uppercase tracking-[0.2em] ${V3_TEXT.mute}`}>{item.id}</div><h1 className={`mt-2 font-display text-[32px] leading-tight tracking-tight font-semibold ${V3_TEXT.strong}`}>{item.title}</h1><div className={`mt-2 text-[13px] font-mono ${V3_TEXT.mute}`}>{item.meta}</div></div></div></div>
          <div className="surface p-6"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Context</div><p className={`mt-3 text-[14px] leading-relaxed ${V3_TEXT.body}`}>{item.summary || "No additional context was returned."}</p>{item.tags?.length ? <div className="mt-4 flex flex-wrap gap-2">{item.tags.map((tag) => <span className={`surface-inset rounded-full px-2.5 py-1 text-[10px] ${V3_TEXT.body}`} key={tag}>{tag}</span>)}</div> : null}{evidence.length ? <div className="mt-6 space-y-2">{evidence.map((value, index) => <div className={`surface-inset rounded-xl p-3 text-[12px] ${V3_TEXT.body}`} key={`${value}-${index}`}>{value}</div>)}</div> : null}</div>
        </section>
        <aside className="col-span-12 lg:col-span-4 space-y-6"><div className="surface p-6"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Decision</div><span className={`mt-4 inline-flex text-[10px] uppercase tracking-widest px-3 py-1.5 rounded-full border ${STATUS_CLASSES[itemTone]}`}>{item.status}</span><dl className="mt-6 space-y-3"><SideValue label="Product" value={config.name} /><SideValue label="Source" value={item.source || "product analysis"} /><SideValue label="Score" value={String(item.score ?? "—")} />{(item.facts || []).map((fact) => <SideValue key={fact.label} label={fact.label} value={String(fact.value ?? "—")} />)}</dl></div>{links.map((link) => <a className={`surface p-5 flex items-center justify-between text-[13px] ${V3_TEXT.body}`} href={link.url} key={`${link.label}-${link.url}`} rel="noreferrer" target="_blank">{link.label || "Open source evidence"} <ExternalLink size={13} /></a>)}</aside>
      </div>
    </>
  );
}

function SideValue({ label, value }) {
  return <div className="flex items-center justify-between gap-4"><dt className={`text-[11px] ${V3_TEXT.mute}`}>{label}</dt><dd className={`text-[12px] ${V3_TEXT.strong} text-right`}>{value}</dd></div>;
}

function githubStatusLabel(health, prefix = false) {
  if (health.github_ready || health.github?.token_verified) return prefix ? "GitHub verified" : "verified";
  if (health.github?.token_configured) return prefix ? "GitHub unverified" : "verification failed";
  return prefix ? "Token missing" : "token missing";
}

function FormField({ field, form, setForm, inputRef }) {
  if (field.type === "checkbox") {
    return <label className={`surface-inset rounded-xl p-4 flex items-start gap-3 text-[13px] ${field.disabled ? "opacity-60" : ""} ${V3_TEXT.body}`}><input checked={!field.disabled && Boolean(form[field.key])} disabled={field.disabled} onChange={(event) => setForm((current) => ({ ...current, [field.key]: event.target.checked }))} type="checkbox" className="mt-0.5 accent-[color:var(--accent-2)]" /><span><span className="block">{field.label}</span>{field.help ? <span className={`mt-1 block text-[10px] leading-relaxed ${V3_TEXT.mute}`}>{field.help}</span> : null}</span></label>;
  }
  if (field.type === "select") {
    return <label className="block"><span className={`text-[10px] uppercase tracking-[0.2em] ${V3_TEXT.mute}`}>{field.label}</span><div className="surface-inset mt-2 rounded-xl h-12 px-4 flex items-center"><select value={form[field.key] ?? ""} onChange={(event) => setForm((current) => ({ ...current, [field.key]: event.target.value }))} className={`bg-transparent outline-none w-full text-[13px] ${V3_TEXT.strong}`}>{(field.options || []).map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}</select></div>{field.help ? <span className={`mt-1 block text-[10px] leading-relaxed ${V3_TEXT.mute}`}>{field.help}</span> : null}</label>;
  }
  if (field.type === "textarea") {
    return <label className={field.fullWidth ? "sm:col-span-2" : "block"}><span className={`text-[10px] uppercase tracking-[0.2em] ${V3_TEXT.mute}`}>{field.label}</span><div className="surface-inset mt-2 rounded-xl p-4"><textarea ref={field.primary ? inputRef : undefined} value={form[field.key] ?? ""} onChange={(event) => setForm((current) => ({ ...current, [field.key]: event.target.value }))} placeholder={field.placeholder || ""} rows={field.rows || 10} className={`bg-transparent outline-none w-full resize-y font-mono text-[12px] leading-relaxed ${V3_TEXT.strong}`} /></div>{field.help ? <span className={`mt-1 block text-[10px] leading-relaxed ${V3_TEXT.mute}`}>{field.help}</span> : null}</label>;
  }
  return <label className="block"><span className={`text-[10px] uppercase tracking-[0.2em] ${V3_TEXT.mute}`}>{field.label}</span><div className="surface-inset mt-2 rounded-xl h-12 px-4 flex items-center gap-2">{field.icon === "github" ? <Github size={14} className={V3_TEXT.dim} /> : null}<input ref={field.primary ? inputRef : undefined} value={form[field.key] ?? ""} onChange={(event) => setForm((current) => ({ ...current, [field.key]: event.target.value }))} placeholder={field.placeholder || ""} type={field.type || "text"} min={field.min} max={field.max} className={`bg-transparent outline-none w-full text-[13px] ${V3_TEXT.strong}`} /></div>{field.help ? <span className={`mt-1 block text-[10px] leading-relaxed ${V3_TEXT.mute}`}>{field.help}</span> : null}</label>;
}

function HistoryRow({ config, entry, onLoad }) {
  const badges = config.historyBadges?.(entry) || [];
  const identity = config.historyIdentity?.(entry) || (entry.id ? `run ${String(entry.id).slice(0, 8)}` : "saved run");
  return (
    <button className="surface-inset rounded-xl p-4 w-full text-left flex items-center justify-between gap-4" onClick={() => onLoad(entry.id)} type="button">
      <div className="min-w-0 flex-1">
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <div className={`font-display text-[16px] ${V3_TEXT.strong}`}>{config.historyTitle(entry)}</div>
          {badges.length ? <div className="flex shrink-0 flex-wrap gap-2">{badges.map((badge) => <span className={`inline-flex min-h-7 items-center justify-center rounded-full border px-2.5 text-[10px] uppercase tracking-wider ${STATUS_CLASSES[tone(badge.tone)]}`} key={`${badge.label}-${badge.tone}`}>{badge.label}</span>)}</div> : null}
        </div>
        {config.historySummary?.(entry) ? <div className={`mt-1 line-clamp-2 text-[12px] ${V3_TEXT.body}`}>{config.historySummary(entry)}</div> : null}
        <div className={`mt-2 flex flex-wrap gap-x-2 gap-y-1 text-[11px] ${V3_TEXT.mute}`}>
          <span>{new Date(entry.created_at).toLocaleString()}</span>
          <span>· {identity}</span>
          {config.historyMeta?.(entry) ? <span>· {config.historyMeta(entry)}</span> : null}
        </div>
      </div>
      <ArrowUpRight size={14} className={`${V3_TEXT.dim} shrink-0`} />
    </button>
  );
}

export function IntegratedProductApp({ apiBase, auth, config, fetcher }) {
  const storagePrefix = `${config.productKey}.v3`;
  const inputRef = useRef(null);
  const initialRunId = useRef(typeof window === "undefined" ? "" : new URLSearchParams(window.location.search).get("run") || "");
  const tabs = [{ id: "workspace", label: config.workspaceLabel }, { id: "history", label: "History" }, ...(config.extraTabs || []).map(({ id, label }) => ({ id, label })), { id: "checks", label: "Checks" }, { id: "sources", label: "Sources" }];
  const [activeTab, setActiveTab] = useState(() => {
    const requested = new URLSearchParams(window.location.search).get("tab");
    return tabs.some((tab) => tab.id === requested) ? requested : localStorage.getItem(`${storagePrefix}.tab`) || "workspace";
  });
  const [form, setForm] = useState(() => ({ ...config.defaultForm, repo: localStorage.getItem(`${storagePrefix}.repo`) || config.defaultForm.repo || "" }));
  const [health, setHealth] = useState({});
  const [checks, setChecks] = useState([]);
  const [overview, setOverview] = useState({ counts: {} });
  const [history, setHistory] = useState([]);
  const [result, setResult] = useState(null);
  const [selected, setSelected] = useState(null);
  const [query, setQuery] = useState("");
  const [historyQuery, setHistoryQuery] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [running, setRunning] = useState(false);
  const dashboard = useSavedDashboardViews({
    storageKey: `${storagePrefix}.dashboard`,
    defaultView: config.dashboard?.defaultView || { sort: "default" },
  });
  const historyDashboard = useSavedDashboardViews({
    storageKey: `${storagePrefix}.history-dashboard`,
    defaultView: config.historyDashboard?.defaultView || { sort: "newest" },
  });

  async function refresh(loadLatest = false) {
    setLoading(true);
    setError("");
    try {
      const [nextHealth, nextChecks, nextOverview, nextHistory] = await Promise.all([
        readJson(await fetcher(`${apiBase}/health`)),
        readJson(await fetcher(`${apiBase}/startup/checks`)),
        readJson(await fetcher(`${apiBase}/overview`)),
        readJson(await fetcher(`${apiBase}/history`)),
      ]);
      const list = config.historyItems ? config.historyItems(nextHistory) : Array.isArray(nextHistory) ? nextHistory : [];
      setHealth(nextHealth);
      setChecks(nextChecks.checks || []);
      setOverview(nextOverview);
      setHistory(list);
      const requestedRunId = initialRunId.current;
      const resultId = requestedRunId || ((loadLatest || !result) ? list[0]?.id : "");
      if (resultId) {
        const loaded = await readJson(await fetcher(`${apiBase}/history/${encodeURIComponent(resultId)}`));
        setResult(loaded);
        setSelected(null);
        setForm((current) => ({ ...current, ...config.formFromResult?.(loaded) }));
        initialRunId.current = "";
      }
    } catch (err) {
      setError(err.message || `${config.name} could not load.`);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => { refresh(true); }, []);
  useEffect(() => { localStorage.setItem(`${storagePrefix}.tab`, activeTab); }, [activeTab]);
  useEffect(() => {
    const url = new URL(window.location.href);
    if (activeTab === "workspace") url.searchParams.delete("tab");
    else url.searchParams.set("tab", activeTab);
    window.history.replaceState({}, "", url.toString());
  }, [activeTab]);
  useEffect(() => { localStorage.setItem(`${storagePrefix}.repo`, form.repo || ""); }, [form.repo]);
  useEffect(() => {
    if (typeof window === "undefined") return;
    const url = new URL(window.location.href);
    if (result?.id) url.searchParams.set("run", result.id);
    else url.searchParams.delete("run");
    window.history.replaceState({}, "", url.toString());
  }, [result?.id]);

  async function run() {
    const validationError = config.validate?.(form) || (!form.repo?.trim() ? "Enter a repository in owner/name format before running." : "");
    if (validationError) {
      setError(validationError);
      setActiveTab("sources");
      setTimeout(() => inputRef.current?.focus(), 0);
      return;
    }
    setRunning(true);
    setError("");
    try {
      const actionPath = typeof config.actionPath === "function" ? config.actionPath(form, health) : config.actionPath;
      const next = await readJson(await fetcher(`${apiBase}${actionPath}`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(config.serialize(form, health)) }));
      setResult(next);
      setSelected(null);
      if (activeTab !== "history") setActiveTab("workspace");
      setForm((current) => ({ ...current, ...config.formFromResult?.(next) }));
      await refresh(false);
    } catch (err) {
      setError(err.message || `${config.name} could not run.`);
    } finally {
      setRunning(false);
    }
  }

  async function load(id) {
    try {
      const loaded = await readJson(await fetcher(`${apiBase}/history/${encodeURIComponent(id)}`));
      setResult(loaded);
      setSelected(null);
      setForm((current) => ({ ...current, ...config.formFromResult?.(loaded) }));
      setActiveTab("workspace");
    } catch (err) {
      setError(err.message || "Could not load that run.");
    }
  }

  const rawItems = config.items(result);
  const items = useMemo(() => rawItems.map(config.mapItem), [rawItems, config]);
  const filtered = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    const matchesQuery = (item) => !normalizedQuery || `${item.id} ${item.title} ${item.meta} ${item.summary} ${(item.evidence || []).join(" ")}`.toLowerCase().includes(normalizedQuery);
    const next = items.filter((item) => matchesQuery(item) && (!config.dashboard?.filterItem || config.dashboard.filterItem(item, dashboard.view)));
    return config.dashboard?.sortItems ? [...next].sort((left, right) => config.dashboard.sortItems(left, right, dashboard.view.sort)) : next;
  }, [dashboard.view, items, query, config]);
  const dashboardFilters = config.dashboard?.filters?.(items, dashboard.view) || [];
  const filteredHistory = useMemo(() => {
    const normalizedQuery = historyQuery.trim().toLowerCase();
    const matchesQuery = (entry) => !normalizedQuery || `${entry.id} ${entry.repo} ${entry.pr_number} ${entry.pr_title} ${entry.readiness} ${entry.decision} ${entry.branch} ${entry.target_version} ${entry.target_tag} ${entry.summary} ${config.historySearchText?.(entry) || ""}`.toLowerCase().includes(normalizedQuery);
    const next = history.filter((entry) => matchesQuery(entry) && (!config.historyDashboard?.filterEntry || config.historyDashboard.filterEntry(entry, historyDashboard.view)));
    return config.historyDashboard?.sortEntries ? [...next].sort((left, right) => config.historyDashboard.sortEntries(left, right, historyDashboard.view.sort)) : next;
  }, [config, history, historyDashboard.view, historyQuery]);
  const historyFilters = config.historyDashboard?.filters?.(history, historyDashboard.view) || [];
  const metrics = config.metrics(result, overview, health);
  const hero = config.hero(result, overview);
  const status = config.status(result, overview);
  const WorkspaceDetails = config.WorkspaceDetails;
  const ChecksDetails = config.ChecksDetails;
  const SourcesDetails = config.SourcesDetails;
  const formFields = typeof config.fields === "function" ? config.fields(health, form) : config.fields;

  if (selected) return <ProductShell productKey={config.productKey}><Detail config={config} item={selected} onBack={() => setSelected(null)} /></ProductShell>;

  return (
    <ProductShell productKey={config.productKey}>
      <ProductHeader activeTab={activeTab} githubLabel={githubStatusLabel(health, true)} icon={config.icon} onRun={run} onSignOut={auth.logout} onTabChange={setActiveTab} productName={config.name} runDisabled={running} runLabel={running ? config.runningLabel : config.runLabel} subtitle={config.subtitle} tabs={tabs} />
      <div className="mx-auto max-w-[1440px] px-3 sm:px-6 pt-6 sm:pt-10 pb-24">
        {error ? <div className="surface mb-6 px-5 py-4 text-[12px] text-red-800 dark:text-red-300">{error}</div> : null}
        {activeTab === "workspace" ? <>
          <section className="grid grid-cols-12 gap-6 items-stretch">
            <div className="surface col-span-12 lg:col-span-8 p-6 sm:p-10"><div className={`flex items-center gap-2 text-[11px] tracking-[0.2em] uppercase ${V3_TEXT.mute}`}><Sparkles size={12} style={{ color: "var(--accent-2)" }} />{config.eyebrow} · {result?.repo || form.repo || "no repository selected"}</div><h1 className={`font-display mt-4 text-[44px] sm:text-[68px] leading-[0.95] tracking-[-0.03em] font-semibold ${V3_TEXT.strong}`}>{hero.lead}<br />{hero.middle} <span className="bg-clip-text text-transparent" style={{ backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2), #cbd5e1)" }}>{hero.highlight}</span></h1><p className={`mt-6 max-w-xl text-[15px] ${V3_TEXT.body} leading-relaxed`}>{config.description}</p><div className="mt-8 flex flex-wrap gap-2">{config.chips(result, health).map((value, index) => <span key={`${value}-${index}`} className={`surface-inset px-3 h-8 rounded-full text-[12px] flex items-center gap-2 ${index ? V3_TEXT.mute : V3_TEXT.strong}`}><span className="h-1.5 w-1.5 rounded-full" style={{ background: index ? "var(--text-dim)" : "var(--accent-2)" }} />{value}</span>)}</div></div>
            <div className="surface col-span-12 lg:col-span-4 p-6 overflow-hidden"><div className="absolute -top-20 -right-16 h-56 w-56 rounded-full opacity-40 blur-2xl" style={{ backgroundImage: "var(--orb-1)" }} /><div className="relative"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute} flex items-center gap-1.5`}><Clock size={11} />Current decision</div><div className={`mt-3 font-display text-[46px] font-semibold ${V3_TEXT.strong} leading-none`}>{status.label}</div><div className={`mt-2 text-[12px] ${V3_TEXT.mute}`}>{status.detail}</div><div className="mt-6 h-2 rounded-full overflow-hidden" style={{ background: "var(--surface-border)" }}><div className="h-full rounded-full" style={{ width: status.progress || "70%", backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))" }} /></div><div className="mt-6 grid grid-cols-3 gap-2">{status.stats.map(([label, value]) => <div key={label} className="surface-inset rounded-xl p-2.5"><div className={`text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>{label}</div><div className={`font-display text-[16px] font-semibold tabular-nums ${V3_TEXT.strong}`}>{value}</div></div>)}</div></div></div>
          </section>
          <section className="mt-8 grid grid-cols-2 lg:grid-cols-4 gap-4">{metrics.slice(0, 4).map((metric) => <MetricCard icon={Activity} key={metric.label} {...metric} />)}</section>
          <section className="mt-8 grid grid-cols-12 gap-6"><div className="col-span-12 lg:col-span-8"><div className="surface p-5"><div className="mb-4"><div><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>{config.queueLabel}</div><div className={`font-display text-2xl mt-0.5 ${V3_TEXT.strong}`}>{filtered.length} in view <span className={`${V3_TEXT.dim} font-normal`}>/ {items.length} tracked</span></div></div></div>{config.dashboard ? <DashboardControls filters={dashboardFilters} onApplySavedView={dashboard.applyView} onDeleteSavedView={dashboard.deleteView} onFilterChange={dashboard.updateView} onQueryChange={setQuery} onReset={() => { dashboard.resetView(); setQuery(""); }} onSaveView={dashboard.saveView} onSortChange={(nextSort) => dashboard.updateView("sort", nextSort)} query={query} savedViews={dashboard.savedViews} searchPlaceholder={config.searchPlaceholder} sort={dashboard.view.sort} sortOptions={config.dashboard.sortOptions} /> : <div className="surface-inset flex items-center gap-2 rounded-full px-3 h-9 w-full sm:w-[260px]"><Search size={13} className={V3_TEXT.dim} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder={config.searchPlaceholder} className={`bg-transparent outline-none text-[12px] w-full ${V3_TEXT.strong}`} /></div>}<div className="mt-5">{loading ? <div className={`py-14 text-center ${V3_TEXT.mute}`}>Loading…</div> : <ProgressiveList empty={<div className={`py-14 text-center text-[13px] ${V3_TEXT.mute}`}>{config.emptyLabel}</div>} initialCount={config.dashboard?.initialCount || 6} itemLabel={config.dashboard?.itemLabel || "items"} items={filtered} renderItem={(item) => <ItemRow item={item} key={item.id} onOpen={setSelected} />} />}</div></div></div><aside className="col-span-12 lg:col-span-4 space-y-6"><div className="surface p-5 overflow-hidden"><div className="absolute -top-10 -right-10 h-32 w-32 rounded-full opacity-60 blur-2xl" style={{ backgroundImage: "var(--orb-1)" }} /><div className="relative"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Active target</div><div className={`mt-2 font-display text-[22px] font-semibold ${V3_TEXT.strong}`}>{result?.repo || form.repo || "No repository selected"}</div><div className={`text-[12px] ${V3_TEXT.mute}`}>{config.targetSubtitle(result)}</div></div></div><div className="surface p-5"><div className="flex items-center justify-between mb-3"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Evidence</div><Zap size={13} style={{ color: "var(--accent-2)" }} /></div>{metrics.slice(0, 4).map((metric, index) => <div key={metric.label} className={`flex items-center justify-between py-2.5 ${index ? "border-t" : ""}`} style={index ? { borderColor: "var(--surface-border-2)" } : undefined}><span className={`text-[13px] ${V3_TEXT.body}`}>{metric.label}</span><span className={`font-display text-[15px] font-semibold ${V3_TEXT.strong}`}>{metric.value}</span></div>)}</div><div className="surface p-5"><div className="flex items-center justify-between mb-3"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Recent runs</div><Cpu size={13} className={V3_TEXT.mute} /></div>{history.slice(0, 4).map((entry, index) => <button key={entry.id} onClick={() => load(entry.id)} className={`w-full flex items-center justify-between py-2 text-left ${index ? "border-t" : ""}`} style={index ? { borderColor: "var(--surface-border-2)" } : undefined} type="button"><div><div className={`text-[13px] ${V3_TEXT.strong}`}>{config.historyTitle(entry)}</div><div className={`text-[11px] ${V3_TEXT.mute}`}>{timeAgo(entry.created_at)} ago</div></div><ArrowUpRight size={13} className={V3_TEXT.dim} /></button>)}</div></aside></section>
          {WorkspaceDetails ? <WorkspaceDetails health={health} onError={setError} onLoad={load} result={result} /> : null}
        </> : null}
        {activeTab === "history" ? <HistoryDashboard dashboard={historyDashboard} filters={historyFilters} initialCount={config.historyDashboard?.initialCount || 6} items={filteredHistory} loading={loading} onQueryChange={setHistoryQuery} onRefresh={() => refresh(false)} query={historyQuery} renderItem={(entry) => <HistoryRow config={config} entry={entry} key={entry.id} onLoad={load} />} searchPlaceholder={config.historyDashboard?.searchPlaceholder || "Search run, repository, PR…"} sortOptions={config.historyDashboard?.sortOptions || [{ value: "newest", label: "Newest first" }]} totalCount={history.length} /> : null}
        {(config.extraTabs || []).map((tab) => activeTab === tab.id ? <div key={tab.id}>{tab.render({ apiBase, auth, fetcher, form, health, history, loading, onError: setError, onLoad: load, onRefresh: () => refresh(false), result, setForm })}</div> : null)}
        {activeTab === "checks" ? <><div className="grid grid-cols-12 gap-6"><section className="surface col-span-12 lg:col-span-8 p-6"><div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between"><div><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Startup evidence</div><h1 className={`font-display mt-2 text-[42px] font-semibold ${V3_TEXT.strong}`}>System checks.</h1></div><button className={`surface-inset h-9 rounded-full px-4 text-[11px] ${V3_TEXT.body}`} disabled={loading} onClick={() => refresh(false)} type="button">{loading ? "Refreshing…" : "Refresh checks"}</button></div><div className="mt-7"><StartupCheckList checks={checks} /></div></section><aside className="surface col-span-12 lg:col-span-4 p-6"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Unified backend</div><div className={`mt-3 font-display text-[46px] font-semibold ${V3_TEXT.strong}`}>{health.status || "unknown"}</div><dl className="mt-6 space-y-3"><SideValue label="Database" value={health.db_ok ? "ready" : "unavailable"} /><SideValue label="GitHub" value={githubStatusLabel(health)} /><SideValue label="Runs" value={String(history.length)} /></dl></aside></div>{ChecksDetails ? <ChecksDetails checks={checks} health={health} history={history} /> : null}</> : null}
        {activeTab === "sources" ? <><div className="grid grid-cols-12 gap-6"><section className="surface col-span-12 lg:col-span-8 p-6 sm:p-8"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Product intake</div><h1 className={`font-display mt-2 text-[42px] font-semibold ${V3_TEXT.strong}`}>{config.formTitle}</h1><div className="mt-7 grid grid-cols-1 sm:grid-cols-2 gap-4">{formFields.map((field) => <FormField field={field} form={form} inputRef={inputRef} key={field.key} setForm={setForm} />)}</div><button disabled={running || !form.repo?.trim()} onClick={run} className="mt-6 h-11 px-5 rounded-full text-[12px] font-semibold text-white disabled:opacity-50" style={{ backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))" }} type="button">{running ? config.runningLabel : config.runLabel}</button></section><aside className="surface col-span-12 lg:col-span-4 p-6"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Connection</div><div className={`mt-3 font-display text-[36px] font-semibold ${V3_TEXT.strong}`}>{githubStatusLabel(health, true)}</div><p className={`mt-3 text-[13px] ${V3_TEXT.body}`}>{config.sourceHelp}</p></aside></div>{SourcesDetails ? <SourcesDetails health={health} /> : null}</> : null}
      </div>
    </ProductShell>
  );
}

export { countLabel, readJson, timeAgo };
