import { useEffect, useState } from "react";
import { Activity, Bookmark, Check, ChevronDown, ChevronUp, Copy, ListFilter, Search, ShieldAlert, SlidersHorizontal, Trash2 } from "lucide-react";
import { V3_TEXT } from "./tokens.js";

function readStoredJson(key, fallback) {
  try {
    const value = JSON.parse(localStorage.getItem(key) || "null");
    return value ?? fallback;
  } catch {
    return fallback;
  }
}

export function useSavedDashboardViews({ storageKey, defaultView }) {
  const currentKey = `${storageKey}.current`;
  const savedKey = `${storageKey}.saved`;
  const [view, setView] = useState(() => ({ ...defaultView, ...readStoredJson(currentKey, {}) }));
  const [savedViews, setSavedViews] = useState(() => readStoredJson(savedKey, []));

  useEffect(() => { localStorage.setItem(currentKey, JSON.stringify(view)); }, [currentKey, view]);
  useEffect(() => { localStorage.setItem(savedKey, JSON.stringify(savedViews)); }, [savedKey, savedViews]);

  function updateView(key, value) {
    setView((current) => ({ ...current, [key]: value }));
  }

  function saveView(name) {
    const trimmed = name.trim();
    if (!trimmed) return false;
    setSavedViews((current) => [...current.filter((item) => item.name !== trimmed), { name: trimmed, view }]);
    return true;
  }

  function applyView(name) {
    const saved = savedViews.find((item) => item.name === name);
    if (saved) setView({ ...defaultView, ...saved.view });
  }

  function deleteView(name) {
    setSavedViews((current) => current.filter((item) => item.name !== name));
  }

  function resetView() {
    setView({ ...defaultView });
  }

  return { applyView, deleteView, resetView, saveView, savedViews, setView, updateView, view };
}

export function DashboardControls({
  filters,
  onApplySavedView,
  onDeleteSavedView,
  onFilterChange,
  onQueryChange,
  onReset,
  onSaveView,
  onSortChange,
  query,
  searchPlaceholder = "Search CVE, file, package…",
  savedViews,
  sort,
  sortOptions,
}) {
  const [viewName, setViewName] = useState("");
  const [selectedView, setSelectedView] = useState("");

  function save() {
    if (onSaveView(viewName)) {
      setSelectedView(viewName.trim());
      setViewName("");
    }
  }

  return (
    <div className="space-y-3">
      <div className="flex flex-col gap-2 xl:flex-row xl:items-center">
        <div className="surface-inset flex h-9 min-w-0 flex-1 items-center gap-2 rounded-full px-3">
          <Search size={13} className={V3_TEXT.dim} />
          <input value={query} onChange={(event) => onQueryChange(event.target.value)} placeholder={searchPlaceholder} className={`w-full bg-transparent text-[12px] outline-none ${V3_TEXT.strong} placeholder:text-[color:var(--text-dim)]`} />
        </div>
        <div className="flex items-center gap-2 overflow-x-auto pb-1 xl:pb-0">
          <ListFilter size={13} className={`${V3_TEXT.dim} shrink-0`} />
          {filters.map((filter) => <label className={`surface-inset flex h-9 shrink-0 items-center gap-1.5 rounded-full px-3 text-[11px] ${V3_TEXT.body}`} key={filter.key}>
            <span className={V3_TEXT.mute}>{filter.label}</span>
            <select className="max-w-32 bg-transparent outline-none" value={filter.value} onChange={(event) => onFilterChange(filter.key, event.target.value)}>
              {filter.options.map((option) => <option value={option.value} key={option.value}>{option.label}</option>)}
            </select>
          </label>)}
          <label className={`surface-inset flex h-9 shrink-0 items-center gap-1.5 rounded-full px-3 text-[11px] ${V3_TEXT.body}`}>
            <SlidersHorizontal size={12} className={V3_TEXT.dim} />
            <select className="bg-transparent outline-none" value={sort} onChange={(event) => onSortChange(event.target.value)}>
              {sortOptions.map((option) => <option value={option.value} key={option.value}>{option.label}</option>)}
            </select>
          </label>
          <button className={`h-9 shrink-0 px-3 text-[11px] ${V3_TEXT.mute}`} onClick={onReset} type="button">Reset</button>
        </div>
      </div>
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex min-w-0 items-center gap-2">
          <Bookmark size={13} className={V3_TEXT.dim} />
          <select className={`surface-inset h-9 min-w-40 rounded-full bg-transparent px-3 text-[11px] outline-none ${V3_TEXT.body}`} value={selectedView} onChange={(event) => { setSelectedView(event.target.value); if (event.target.value) onApplySavedView(event.target.value); }}>
            <option value="">Saved views</option>
            {savedViews.map((item) => <option value={item.name} key={item.name}>{item.name}</option>)}
          </select>
          {selectedView ? <button aria-label={`Delete ${selectedView} saved view`} className={`surface-inset grid h-9 w-9 place-items-center rounded-full ${V3_TEXT.mute}`} onClick={() => { onDeleteSavedView(selectedView); setSelectedView(""); }} type="button"><Trash2 size={12} /></button> : null}
        </div>
        <div className="flex min-w-0 items-center gap-2">
          <input value={viewName} onChange={(event) => setViewName(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") save(); }} placeholder="Name this view" className={`surface-inset h-9 min-w-0 rounded-full bg-transparent px-3 text-[11px] outline-none ${V3_TEXT.strong}`} />
          <button disabled={!viewName.trim()} onClick={save} className="h-9 shrink-0 rounded-full px-4 text-[11px] font-semibold text-white disabled:opacity-40" style={{ backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))" }} type="button">Save view</button>
        </div>
      </div>
    </div>
  );
}

export function ProgressiveList({ empty, initialCount = 6, itemLabel = "findings", items, renderItem }) {
  const [expanded, setExpanded] = useState(false);
  const visible = expanded ? items : items.slice(0, initialCount);

  useEffect(() => { setExpanded(false); }, [items]);

  if (!items.length) return empty;

  return (
    <>
      <div className="space-y-2">{visible.map(renderItem)}</div>
      {items.length > initialCount ? <div className="mt-4 flex justify-center">
        <button type="button" onClick={() => setExpanded((value) => !value)} className={`surface-inset flex h-10 items-center gap-2 rounded-full px-5 text-[12px] ${V3_TEXT.body}`}>
          {expanded ? <ChevronUp size={13} /> : <ChevronDown size={13} />}
          {expanded ? `Show first ${initialCount}` : `Show all ${items.length} ${itemLabel}`}
        </button>
      </div> : null}
    </>
  );
}

export function HistoryDashboard({
  dashboard,
  emptyLabel = "No saved runs match this view.",
  eyebrow = "Saved evidence",
  filters,
  initialCount = 6,
  itemLabel = "runs",
  items,
  loading,
  onQueryChange,
  onRefresh,
  query,
  renderItem,
  searchPlaceholder = "Search repository, summary, run ID…",
  sortOptions,
  title = "Run history.",
  totalCount,
}) {
  return (
    <section className="surface p-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>{eyebrow}</div>
          <h1 className={`font-display mt-2 text-[42px] tracking-tight font-semibold ${V3_TEXT.strong}`}>{title}</h1>
          <div className={`mt-1 text-[12px] ${V3_TEXT.mute}`}>{items.length} in view / {totalCount} saved</div>
        </div>
        <button className={`surface-inset h-9 rounded-full px-4 text-[11px] ${V3_TEXT.body}`} disabled={loading} onClick={onRefresh} type="button">{loading ? "Refreshing…" : "Refresh history"}</button>
      </div>
      <div className="mt-6">
        <DashboardControls
          filters={filters}
          onApplySavedView={dashboard.applyView}
          onDeleteSavedView={dashboard.deleteView}
          onFilterChange={dashboard.updateView}
          onQueryChange={onQueryChange}
          onReset={() => { dashboard.resetView(); onQueryChange(""); }}
          onSaveView={dashboard.saveView}
          onSortChange={(sort) => dashboard.updateView("sort", sort)}
          query={query}
          savedViews={dashboard.savedViews}
          searchPlaceholder={searchPlaceholder}
          sort={dashboard.view.sort}
          sortOptions={sortOptions}
        />
      </div>
      <div className="mt-7">
        <ProgressiveList
          empty={<div className={`py-14 text-center text-[13px] ${V3_TEXT.mute}`}>{emptyLabel}</div>}
          initialCount={initialCount}
          itemLabel={itemLabel}
          items={items}
          renderItem={renderItem}
        />
      </div>
    </section>
  );
}

function startupCheckTitle(check) {
  if (check.name || check.label) return check.name || check.label;
  const message = String(check.message || check.detail || check.msg || "").toLowerCase();
  if (message.includes("sqlite") || message.includes("database")) return "Database";
  if (message.includes("api-key") || message.includes("auth")) return "Authentication";
  if (message.includes("reviewbee")) return "ReviewBee";
  if (message.includes("trustgate")) return "TrustGate";
  if (message.includes("repomemory")) return "RepoMemory";
  if (message.includes("webhook")) return "GitHub webhook";
  if (message.includes("public url")) return "Public URL";
  if (message.includes("github token") || message.includes("github access")) return "GitHub access";
  if (message.includes("approval policy")) return "Approval policy";
  if (message.includes("reads ") || message.includes("returns a simple")) return "Product mode";
  return "Startup configuration";
}

function startupLevelClass(level) {
  if (level === "error" || level === "failed") return "border-red-900/30 bg-red-900/10 text-red-800 dark:border-red-400/25 dark:bg-red-500/10 dark:text-red-300";
  if (level === "warn" || level === "missing") return "border-amber-900/30 bg-amber-900/10 text-amber-800 dark:border-amber-400/25 dark:bg-amber-500/10 dark:text-amber-300";
  if (level === "ok" || level === "verified") return "border-emerald-900/30 bg-emerald-900/10 text-emerald-800 dark:border-emerald-400/25 dark:bg-emerald-500/10 dark:text-emerald-300";
  return `border-[var(--line)] bg-[var(--surface-soft)] ${V3_TEXT.mute}`;
}

export function StartupCheckList({ checks = [] }) {
  if (!checks.length) return <div className={`py-14 text-center text-[13px] ${V3_TEXT.mute}`}>No startup checks were returned.</div>;
  return <div className="space-y-2">{checks.map((check, index) => {
    const status = String(check.status || check.level || "info").toLowerCase();
    return <div className="surface-inset rounded-xl p-4" key={`${startupCheckTitle(check)}-${check.msg || check.message || index}`}><div className="flex items-start justify-between gap-3"><div><div className={`text-[13px] font-semibold ${V3_TEXT.strong}`}>{startupCheckTitle(check)}</div><div className={`mt-1 text-[12px] leading-relaxed ${V3_TEXT.mute}`}>{check.message || check.detail || check.msg || "No detail returned."}</div></div><span className={`inline-flex min-h-7 shrink-0 items-center justify-center rounded-full border px-2.5 text-[10px] uppercase tracking-wider ${startupLevelClass(status)}`}>{status}</span></div></div>;
  })}</div>;
}

export function ActivityTimeline({ caption, eventTypes, events }) {
  const [activeType, setActiveType] = useState("all");
  const visible = activeType === "all" ? events : events.filter((event) => event.kind === activeType);

  function jumpTo(kind) {
    setActiveType(kind);
    window.setTimeout(() => {
      document.getElementById(`activity-${kind}`)?.scrollIntoView({ behavior: "smooth", block: "center" });
    }, 0);
  }

  return (
    <div className="surface p-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <div className={`flex items-center gap-1.5 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Activity size={11} /> Activity timeline</div>
          <div className={`mt-1 text-[11px] ${V3_TEXT.dim}`}>{visible.length} of {events.length} events{caption ? ` · ${caption}` : ""}</div>
        </div>
        <label className={`surface-inset flex h-9 items-center gap-2 rounded-full px-3 text-[11px] ${V3_TEXT.body}`}>
          Jump to
          <select className="bg-transparent outline-none" value={activeType} onChange={(event) => jumpTo(event.target.value)}>
            <option value="all">All activity</option>
            {eventTypes.map((kind) => <option value={kind} key={kind}>{kind}</option>)}
          </select>
        </label>
      </div>
      <div className="mt-4 flex flex-wrap gap-2" aria-label="Filter activity timeline">
        {["all", ...eventTypes].map((kind) => {
          const count = kind === "all" ? events.length : events.filter((event) => event.kind === kind).length;
          return <button type="button" key={kind} onClick={() => setActiveType(kind)} className={`h-8 rounded-full px-3 text-[11px] capitalize transition ${activeType === kind ? `bg-white shadow ${V3_TEXT.strong} dark:bg-white/15` : `surface-inset ${V3_TEXT.mute}`}`}>{kind} · {count}</button>;
        })}
      </div>
      <ol className="relative mt-6 border-l pl-5" style={{ borderColor: "var(--surface-border-2)" }}>
        {visible.map((event, index) => <li id={index === 0 ? `activity-${event.kind}` : undefined} className="relative mb-5 last:mb-0" key={event.id}>
          <span className="absolute -left-[25px] top-1 h-2.5 w-2.5 rounded-full" style={{ background: "var(--accent-2)" }} />
          <div className={`text-[10px] uppercase tracking-widest ${V3_TEXT.mute}`}>{event.kind} · {new Date(event.at).toLocaleString()}</div>
          <div className={`mt-1 text-[13px] ${V3_TEXT.strong}`}>{event.message}</div>
          <div className={`mt-0.5 text-[11px] ${V3_TEXT.dim}`}>by {event.actor}</div>
        </li>)}
        {!visible.length ? <li className={`text-[12px] ${V3_TEXT.mute}`}>No {activeType} events are recorded.</li> : null}
      </ol>
    </div>
  );
}

export function ScanWarnings({ formatWarning = (warning) => String(warning), warnings = [] }) {
  if (!warnings.length) return null;
  return (
    <section className="surface mt-6 border border-amber-700/20 p-5 dark:border-amber-300/15">
      <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><ShieldAlert size={13} className="text-amber-700 dark:text-amber-300" /> Scan warnings</div>
      <div className="mt-3 space-y-2">{warnings.map((warning, index) => <div className={`surface-inset rounded-xl p-3 text-[12px] leading-relaxed ${V3_TEXT.body}`} key={`${warning}-${index}`}>{formatWarning(warning)}</div>)}</div>
    </section>
  );
}

export function GitHubPermissionGuidance({ children }) {
  return <div className={`surface-inset mt-5 rounded-xl p-3 text-[11px] leading-relaxed ${V3_TEXT.mute}`}><span className={`font-semibold ${V3_TEXT.body}`}>GitHub access:</span> {children}</div>;
}

export function CopyMarkdownButton({ content, label = "Copy summary", onError }) {
  const [copied, setCopied] = useState(false);

  async function copy() {
    if (!content) return;
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1800);
    } catch (error) {
      onError?.(error);
    }
  }

  return <button disabled={!content} onClick={copy} className={`surface-inset flex h-9 items-center gap-2 rounded-full px-3 text-[11px] disabled:opacity-40 ${V3_TEXT.body}`} type="button">{copied ? <Check size={12} /> : <Copy size={12} />}{copied ? "Copied" : label}</button>;
}
