import { useEffect, useMemo, useRef, useState } from "react";
import {
  Activity,
  ArrowLeft,
  ArrowUpRight,
  CheckCircle2,
  Clock,
  Cpu,
  ExternalLink,
  Github,
  KeyRound,
  Search,
  ShieldAlert,
  Sparkles,
  Zap,
} from "lucide-react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import {
  MetricCard,
  ProductHeader,
  ProductShell,
  ThemeToggle,
  V3_TEXT,
} from "@patchhivehq/ui-v3";
import { API } from "./config.js";

const TABS = [
  { id: "triage", label: "Triage" },
  { id: "history", label: "History" },
  { id: "checks", label: "Checks" },
  { id: "sources", label: "Sources" },
];

const METRIC_TONES = {
  fix_now: "from-orange-700/70 to-red-900/60",
  plan_next: "from-amber-600/70 to-yellow-800/50",
  watch: "from-slate-500/70 to-slate-800/60",
  runtime_exposed: "from-stone-500/70 to-stone-800/60",
};

const RECOMMENDATION_CLASSES = {
  "fix now": "bg-red-900/10 text-red-800 border-red-900/30 dark:bg-red-500/10 dark:text-red-300 dark:border-red-400/25",
  "plan next": "bg-amber-900/10 text-amber-800 border-amber-900/30 dark:bg-amber-500/10 dark:text-amber-300 dark:border-amber-400/25",
  watch: "bg-stone-800/10 text-stone-800 border-stone-800/30 dark:bg-stone-500/10 dark:text-stone-300 dark:border-stone-400/25",
};

const SEVERITY_CLASSES = {
  critical: "from-red-800 to-orange-700 text-white",
  high: "from-orange-700 to-amber-600 text-white",
  medium: "from-amber-700 to-yellow-700 text-white",
  moderate: "from-amber-700 to-yellow-700 text-white",
  low: "from-slate-600 to-slate-800 text-white",
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

function findingId(finding) {
  return finding.identifiers?.[0] || finding.key || finding.title || "finding";
}

function findingPackage(finding) {
  const parts = [finding.package_name, finding.ecosystem].filter(Boolean);
  return parts.join(" · ") || "repository code";
}

function countLabel(value, singular, plural = `${singular}s`) {
  return `${value || 0} ${(value || 0) === 1 ? singular : plural}`;
}

function recommendation(value) {
  const normalized = String(value || "watch").replaceAll("_", " ").toLowerCase();
  if (normalized.includes("fix")) return "fix now";
  if (normalized.includes("plan")) return "plan next";
  return "watch";
}

function LoginScreen({ auth }) {
  const [key, setKey] = useState("");
  const [error, setError] = useState(auth.authError || "");
  const [busy, setBusy] = useState(false);
  const [generatedKey, setGeneratedKey] = useState("");

  async function submit(event) {
    event.preventDefault();
    if (!key.trim()) return;
    setBusy(true);
    setError("");
    try {
      await readJson(await fetch(`${API}/auth/login`, {
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
      const value = await auth.generateKey({ autoLogin: false });
      setGeneratedKey(value);
      setKey(value);
    } catch (err) {
      setError(err.message || "Could not generate an API key.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <ProductShell productKey="vuln-triage" footerLeft="PatchHive · VulnTriage" footerRight="Secure local session">
      <div className="min-h-[calc(100vh-80px)] grid place-items-center px-6 py-20">
        <section className="surface w-full max-w-lg p-8 overflow-hidden">
          <div className="absolute -top-20 -right-16 h-56 w-56 rounded-full opacity-40 blur-2xl" style={{ backgroundImage: "var(--orb-1)" }} />
          <div className="relative">
            <div className="flex items-center justify-between">
              <div className="h-12 w-12 rounded-xl grid place-items-center text-white" style={{ backgroundImage: "linear-gradient(135deg, var(--accent-2), var(--accent-3))", boxShadow: "var(--accent-glow)" }}>
                <ShieldAlert size={20} />
              </div>
              <ThemeToggle />
            </div>
            <div className={`mt-7 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>PatchHive · Secure product session</div>
            <h1 className={`font-display mt-2 text-[38px] leading-tight tracking-[-0.03em] font-semibold ${V3_TEXT.strong}`}>Open VulnTriage.</h1>
            <p className={`mt-3 text-[14px] leading-relaxed ${V3_TEXT.body}`}>Use the local API key to connect this security queue to the VulnTriage backend.</p>
            <form className="mt-7 space-y-3" onSubmit={submit}>
              <label className={`block text-[10px] uppercase tracking-[0.2em] ${V3_TEXT.mute}`} htmlFor="api-key">API key</label>
              <div className="surface-inset h-11 rounded-xl px-3 flex items-center gap-2">
                <KeyRound size={14} className={V3_TEXT.dim} />
                <input id="api-key" value={key} onChange={(event) => setKey(event.target.value)} className={`bg-transparent outline-none w-full text-[13px] ${V3_TEXT.strong}`} type="password" autoComplete="current-password" />
              </div>
              {error ? <p className="text-[12px] text-red-700 dark:text-red-300">{error}</p> : null}
              {generatedKey ? <p className={`text-[11px] ${V3_TEXT.mute}`}>A new local key was generated. Save it before leaving this screen.</p> : null}
              <div className="flex gap-2 pt-2">
                <button disabled={busy || !key.trim()} className="h-10 flex-1 rounded-full text-[12px] font-semibold text-white disabled:opacity-50" style={{ backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))", boxShadow: "var(--accent-glow)" }} type="submit">
                  {busy ? "Connecting…" : "Connect"}
                </button>
                {auth.bootstrapRequired ? <button disabled={busy} onClick={generate} className={`surface-inset h-10 px-4 rounded-full text-[12px] ${V3_TEXT.body}`} type="button">Generate key</button> : null}
              </div>
            </form>
          </div>
        </section>
      </div>
    </ProductShell>
  );
}

function LoadingScreen() {
  return (
    <ProductShell productKey="vuln-triage" footerLeft="PatchHive · VulnTriage">
      <div className={`min-h-[calc(100vh-80px)] grid place-items-center text-[13px] ${V3_TEXT.mute}`}>Connecting to VulnTriage…</div>
    </ProductShell>
  );
}

function FindingRow({ finding, onOpen }) {
  const severity = String(finding.severity || "low").toLowerCase();
  const rec = recommendation(finding.recommendation);
  return (
    <button type="button" onClick={() => onOpen(finding)} className="surface-inset group rounded-xl p-4 hover:brightness-110 hover:shadow-[0_10px_30px_-15px_rgba(15,23,42,0.35)] w-full text-left">
      <div className="grid grid-cols-[auto_1fr] sm:grid-cols-[auto_1fr_auto] gap-4 items-center">
        <div className={`h-12 w-12 rounded-xl bg-gradient-to-br ${SEVERITY_CLASSES[severity] || SEVERITY_CLASSES.low} grid place-items-center shadow-inner`}>
          <span className="font-display font-semibold text-[15px] tabular-nums">{Number(finding.score || 0).toFixed(1)}</span>
        </div>
        <div className="min-w-0">
          <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.2em] ${V3_TEXT.mute}`}>
            <span className="truncate">{findingId(finding)}</span><span className="opacity-40">·</span><span>{String(finding.source || "scan").replaceAll("_", " ")}</span><span className="opacity-40">·</span><span>{timeAgo(finding.created_at)}</span>
          </div>
          <div className={`mt-1 font-display font-medium text-[16px] tracking-tight ${V3_TEXT.strong} truncate`}>{finding.title || finding.summary}</div>
          <div className={`mt-1 text-[12px] ${V3_TEXT.mute} font-mono truncate`}>
            {finding.location || "repository"} <span className={V3_TEXT.dim}>·</span> <span className={V3_TEXT.body}>{findingPackage(finding)}</span>
          </div>
        </div>
        <div className="col-span-2 sm:col-span-1 flex items-center justify-end gap-2">
          {finding.owner_hint ? <span className={`hidden xl:inline surface-inset rounded-full px-2.5 py-1 text-[10px] ${V3_TEXT.body}`}>{finding.owner_hint}</span> : null}
          <span className={`text-[10px] uppercase tracking-widest px-2.5 py-1 rounded-full border ${RECOMMENDATION_CLASSES[rec]}`}>{rec}</span>
          <ArrowUpRight size={14} className={`${V3_TEXT.dim} group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition`} />
        </div>
      </div>
    </button>
  );
}

function FindingDetail({ finding, onBack }) {
  const severity = String(finding.severity || "low").toLowerCase();
  const rec = recommendation(finding.recommendation);
  const references = [...new Set([finding.html_url, ...(finding.references || [])].filter(Boolean))];
  return (
    <>
      <header className="px-3 sm:px-6 pt-3 sm:pt-6">
        <div className="surface mx-auto max-w-[1200px] px-5 h-16 flex items-center justify-between">
          <button onClick={onBack} className={`flex items-center gap-2 text-[12px] ${V3_TEXT.body}`} type="button"><ArrowLeft size={14} /> Back to queue</button>
          <div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>PatchHive · Finding detail</div>
          <ThemeToggle />
        </div>
      </header>
      <div className="mx-auto max-w-[1200px] px-3 sm:px-6 pt-8 pb-24 grid grid-cols-12 gap-6">
        <section className="col-span-12 lg:col-span-8 space-y-6">
          <div className="surface p-8">
            <div className="flex items-start gap-5">
              <div className={`h-16 w-16 rounded-2xl bg-gradient-to-br ${SEVERITY_CLASSES[severity] || SEVERITY_CLASSES.low} grid place-items-center shadow-inner shrink-0`}>
                <div className="text-center"><div className="font-display font-semibold text-[18px] tabular-nums leading-none">{Number(finding.score || 0).toFixed(1)}</div><div className="mt-1 text-[9px] uppercase tracking-widest opacity-80">{severity}</div></div>
              </div>
              <div className="min-w-0 flex-1">
                <div className={`flex flex-wrap items-center gap-2 text-[10px] uppercase tracking-[0.2em] ${V3_TEXT.mute}`}><span>{findingId(finding)}</span><span className="opacity-40">·</span><span>{String(finding.source || "scan").replaceAll("_", " ")}</span></div>
                <h1 className={`mt-2 font-display text-[32px] leading-tight tracking-tight font-semibold ${V3_TEXT.strong}`}>{finding.title || finding.summary}</h1>
                <div className={`mt-2 text-[13px] font-mono ${V3_TEXT.mute}`}>{finding.location || "repository"} <span className={V3_TEXT.dim}>·</span> <span className={V3_TEXT.body}>{findingPackage(finding)}</span></div>
              </div>
            </div>
            <div className="mt-8 grid grid-cols-2 sm:grid-cols-4 gap-3 text-[12px]">
              <Meta label="Detected" value={finding.created_at ? new Date(finding.created_at).toLocaleDateString() : "scan time"} />
              <Meta label="Owner" value={finding.owner_hint || "repo maintainers"} />
              <Meta label="Priority" value={rec} />
              <Meta label="Reachability" value={finding.reachability || "unknown"} />
            </div>
          </div>
          <div className="surface p-6">
            <div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Context</div>
            <p className={`mt-3 text-[14px] leading-relaxed ${V3_TEXT.body}`}>{finding.summary || "No summary was returned by this security feed."}</p>
            <div className={`mt-6 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Remediation</div>
            <p className={`mt-3 text-[14px] leading-relaxed ${V3_TEXT.body}`}>{finding.next_action || "Review the finding and plan the smallest safe remediation."}</p>
            {finding.evidence?.length ? <ul className={`mt-6 space-y-2 text-[12px] ${V3_TEXT.body}`}>{finding.evidence.map((item) => <li className="surface-inset rounded-xl p-3" key={item}>{item}</li>)}</ul> : null}
          </div>
        </section>
        <aside className="col-span-12 lg:col-span-4 space-y-6">
          <div className="surface p-6">
            <div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Finding status</div>
            <div className={`mt-4 inline-flex text-[10px] uppercase tracking-widest px-3 py-1.5 rounded-full border ${RECOMMENDATION_CLASSES[rec]}`}>{rec}</div>
            <dl className="mt-5 space-y-3">
              <SideValue label="Severity" value={severity} />
              <SideValue label="Tool" value={finding.tool_name || finding.source || "GitHub"} />
              <SideValue label="Package" value={findingPackage(finding)} />
            </dl>
          </div>
          {references.length ? <div className="surface p-6"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>References</div><div className="mt-4 space-y-2">{references.map((href) => <a className={`surface-inset rounded-xl p-3 flex items-center justify-between text-[12px] ${V3_TEXT.body}`} href={href} key={href} rel="noreferrer" target="_blank"><span className="truncate">Open source</span><ExternalLink size={12} /></a>)}</div></div> : null}
        </aside>
      </div>
    </>
  );
}

function Meta({ label, value }) {
  return <div className="surface-inset rounded-xl p-3"><div className={`text-[10px] uppercase tracking-widest ${V3_TEXT.mute}`}>{label}</div><div className={`mt-1 text-[13px] ${V3_TEXT.strong} truncate`}>{value}</div></div>;
}

function SideValue({ label, value }) {
  return <div className="flex items-center justify-between gap-4"><dt className={`text-[11px] ${V3_TEXT.mute}`}>{label}</dt><dd className={`text-[12px] ${V3_TEXT.strong} text-right`}>{value}</dd></div>;
}

function MainProduct({ auth }) {
  const fetcher = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const repoInput = useRef(null);
  const [activeTab, setActiveTab] = useState(() => localStorage.getItem("vuln-triage.v3.tab") || "triage");
  const [form, setForm] = useState(() => ({
    repo: localStorage.getItem("vuln-triage.v3.repo") || "",
    include_code_scanning: true,
    include_dependency_alerts: true,
  }));
  const [health, setHealth] = useState({});
  const [checks, setChecks] = useState([]);
  const [overview, setOverview] = useState({ counts: {}, recent_scans: [] });
  const [history, setHistory] = useState([]);
  const [scan, setScan] = useState(null);
  const [selectedFinding, setSelectedFinding] = useState(null);
  const [bucket, setBucket] = useState("all");
  const [query, setQuery] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [running, setRunning] = useState(false);

  async function refresh({ loadLatest = false } = {}) {
    setError("");
    setLoading(true);
    try {
      const [nextHealth, nextChecks, nextOverview, nextHistory] = await Promise.all([
        readJson(await fetcher(`${API}/health`)),
        readJson(await fetcher(`${API}/startup/checks`)),
        readJson(await fetcher(`${API}/overview`)),
        readJson(await fetcher(`${API}/history`)),
      ]);
      setHealth(nextHealth);
      setChecks(nextChecks.checks || []);
      setOverview(nextOverview);
      setHistory(Array.isArray(nextHistory) ? nextHistory : []);
      const latest = Array.isArray(nextHistory) ? nextHistory[0] : null;
      if ((loadLatest || !scan) && latest?.id) {
        setScan(await readJson(await fetcher(`${API}/history/${encodeURIComponent(latest.id)}`)));
      }
    } catch (err) {
      setError(err.message || "Could not load VulnTriage.");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => { refresh({ loadLatest: true }); }, []);
  useEffect(() => { localStorage.setItem("vuln-triage.v3.tab", activeTab); }, [activeTab]);
  useEffect(() => { localStorage.setItem("vuln-triage.v3.repo", form.repo); }, [form.repo]);

  async function runScan() {
    if (!form.repo.trim()) {
      setError("Enter a repository in owner/name format before running a scan.");
      setActiveTab("sources");
      window.setTimeout(() => repoInput.current?.focus(), 0);
      return;
    }
    setRunning(true);
    setError("");
    try {
      const result = await readJson(await fetcher(`${API}/scan/github/findings`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(form),
      }));
      setScan(result);
      setSelectedFinding(null);
      setActiveTab("triage");
      await refresh();
    } catch (err) {
      setError(err.message || "The security scan failed.");
    } finally {
      setRunning(false);
    }
  }

  async function loadScan(id) {
    setError("");
    try {
      setScan(await readJson(await fetcher(`${API}/history/${encodeURIComponent(id)}`)));
      setActiveTab("triage");
    } catch (err) {
      setError(err.message || "Could not load that scan.");
    }
  }

  const findings = scan?.findings || [];
  const filtered = useMemo(() => findings.filter((finding) => {
    if (bucket !== "all" && recommendation(finding.recommendation) !== bucket) return false;
    const text = `${findingId(finding)} ${finding.title} ${finding.summary} ${finding.location} ${finding.package_name}`.toLowerCase();
    return !query.trim() || text.includes(query.trim().toLowerCase());
  }), [bucket, findings, query]);
  const metrics = scan?.metrics || {};
  const activeRepo = scan?.repo || form.repo || "No repository selected";

  if (selectedFinding) {
    return <ProductShell productKey="vuln-triage" footerLeft="PatchHive · VulnTriage"><FindingDetail finding={selectedFinding} onBack={() => setSelectedFinding(null)} /></ProductShell>;
  }

  return (
    <ProductShell productKey="vuln-triage" footerLeft="PatchHive · VulnTriage">
      <ProductHeader
        activeTab={activeTab}
        githubLabel={health.github_ready ? "GitHub ready" : "Token missing"}
        icon={ShieldAlert}
        onRun={runScan}
        onTabChange={setActiveTab}
        productName="VulnTriage"
        runDisabled={running}
        runLabel={running ? "Scanning…" : "Run scan"}
        subtitle="security queue"
        tabs={TABS}
      />
      <div className="mx-auto max-w-[1440px] px-3 sm:px-6 pt-6 sm:pt-10 pb-24">
        {error ? <div className="surface mb-6 px-5 py-4 text-[12px] text-red-800 dark:text-red-300">{error}</div> : null}
        {activeTab === "triage" ? (
          <>
            <section className="grid grid-cols-12 gap-6 items-stretch">
              <div className="surface col-span-12 lg:col-span-8 p-6 sm:p-10">
                <div className={`flex items-center gap-2 text-[11px] tracking-[0.2em] uppercase ${V3_TEXT.mute}`}><Sparkles size={12} style={{ color: "var(--accent-2)" }} /> Security queue · {activeRepo}</div>
                <h1 className={`font-display mt-4 text-[44px] sm:text-[68px] leading-[0.95] tracking-[-0.03em] font-semibold ${V3_TEXT.strong}`}>
                  {findings.length || "No"} findings <br />
                  need a decision{" "}<span className="bg-clip-text text-transparent" style={{ backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2), #cbd5e1)" }}>today.</span>
                </h1>
                <p className={`mt-6 max-w-xl text-[15px] ${V3_TEXT.body} leading-relaxed`}>Reads GitHub code scanning and Dependabot alerts, then sorts them into fix-now, plan-next, and watch—all in one calm, luminous surface.</p>
                <div className="mt-8 flex flex-wrap gap-2">
                  {[activeRepo, countLabel(metrics.code_scanning_alerts, "code finding"), countLabel(metrics.dependency_alerts, "dependency alert")].map((value, index) => <span key={`${value}-${index}`} className={`surface-inset px-3 h-8 rounded-full text-[12px] flex items-center gap-2 ${index === 0 ? V3_TEXT.strong : V3_TEXT.mute}`}><span className="h-1.5 w-1.5 rounded-full" style={{ background: index === 0 ? "var(--accent-2)" : "var(--text-dim)" }} />{value}</span>)}
                </div>
              </div>
              <div className="surface col-span-12 lg:col-span-4 p-6 overflow-hidden">
                <div className="absolute -top-20 -right-16 h-56 w-56 rounded-full opacity-40 blur-2xl" style={{ backgroundImage: "var(--orb-1)" }} />
                <div className="relative">
                  <div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute} flex items-center gap-1.5`}><Clock size={11} /> Current scan</div>
                  <div className={`mt-3 font-display text-[46px] font-semibold tabular-nums ${V3_TEXT.strong} leading-none`}>{scan ? timeAgo(scan.created_at) : "—"}</div>
                  <div className={`mt-2 text-[12px] ${V3_TEXT.mute}`}>{scan ? `${scan.repo} · ${countLabel(metrics.tracked_findings, "finding")}` : "Run a repository scan to begin"}</div>
                  <div className="mt-6 h-2 rounded-full overflow-hidden" style={{ background: "var(--surface-border)" }}><div className="h-full rounded-full" style={{ width: scan ? "100%" : "8%", backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))" }} /></div>
                  <div className="mt-6 grid grid-cols-3 gap-2">
                    {[["Scans", overview.counts?.scans || 0], ["Repos", overview.counts?.repos || 0], ["Feeds", health.github_ready ? "live" : "off"]].map(([label, value]) => <div key={label} className="surface-inset rounded-xl p-2.5"><div className={`text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>{label}</div><div className={`font-display text-[16px] font-semibold tabular-nums ${V3_TEXT.strong}`}>{value}</div></div>)}
                  </div>
                </div>
              </div>
            </section>
            <section className="mt-8 grid grid-cols-2 lg:grid-cols-4 gap-4">
              <MetricCard icon={Activity} label="Fix now" value={metrics.fix_now || 0} footerLeft="live" footerRight="highest urgency" tone={METRIC_TONES.fix_now} />
              <MetricCard icon={Activity} label="Plan next" value={metrics.plan_next || 0} footerLeft="live" footerRight="owner follow-up" tone={METRIC_TONES.plan_next} />
              <MetricCard icon={Activity} label="Watch" value={metrics.watch || 0} footerLeft="live" footerRight="low pressure" tone={METRIC_TONES.watch} />
              <MetricCard icon={Activity} label="Runtime" value={metrics.runtime_exposed || 0} footerLeft="exposed" footerRight="review first" tone={METRIC_TONES.runtime_exposed} />
            </section>
            <section className="mt-8 grid grid-cols-12 gap-6">
              <div className="col-span-12 lg:col-span-8 space-y-6">
                <div className="surface p-5">
                  <div className="flex flex-col xl:flex-row xl:items-end justify-between gap-3 mb-4">
                    <div><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Findings</div><div className={`font-display text-2xl mt-0.5 tracking-tight ${V3_TEXT.strong}`}>{filtered.length} in view <span className={`${V3_TEXT.dim} font-normal`}>/ {findings.length} tracked</span></div></div>
                    <div className="flex flex-col sm:flex-row items-stretch sm:items-center gap-2">
                      <div className="surface-inset flex items-center gap-2 rounded-full px-3 h-9 w-full sm:w-[240px]"><Search size={13} className={V3_TEXT.dim} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search CVE, file, package…" className={`bg-transparent outline-none text-[12px] w-full ${V3_TEXT.strong} placeholder:text-[color:var(--text-dim)]`} /></div>
                      <div className="surface-inset flex rounded-full p-1 overflow-x-auto">{["all", "fix now", "plan next", "watch"].map((value) => <button key={value} onClick={() => setBucket(value)} className={`px-3 h-7 rounded-full text-[11px] capitalize transition whitespace-nowrap ${bucket === value ? `bg-white shadow ${V3_TEXT.strong} dark:bg-white/15` : V3_TEXT.mute}`} type="button">{value}</button>)}</div>
                    </div>
                  </div>
                  <div className="space-y-2">{loading ? <div className={`py-14 text-center text-[13px] ${V3_TEXT.mute}`}>Loading findings…</div> : filtered.length ? filtered.map((finding) => <FindingRow finding={finding} key={finding.key || findingId(finding)} onOpen={setSelectedFinding} />) : <div className={`py-14 text-center text-[13px] ${V3_TEXT.mute}`}>No findings match this view.</div>}</div>
                </div>
              </div>
              <aside className="col-span-12 lg:col-span-4 space-y-6">
                <div className="surface p-5 overflow-hidden">
                  <div className="absolute -top-10 -right-10 h-32 w-32 rounded-full opacity-60 blur-2xl" style={{ backgroundImage: "var(--orb-1)" }} />
                  <div className="relative"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Active repo</div><div className={`mt-2 font-display text-[22px] font-semibold ${V3_TEXT.strong}`}>{activeRepo}</div><div className={`text-[12px] ${V3_TEXT.mute}`}>GitHub security feeds</div><div className="mt-4 grid grid-cols-3 gap-2 text-center">{[["Tracked", metrics.tracked_findings || 0], ["Owners", metrics.owner_scoped || 0], ["Status", health.status || "unknown"]].map(([label, value]) => <div className="surface-inset rounded-xl p-2" key={label}><div className={`text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>{label}</div><div className={`font-display text-[18px] font-semibold tabular-nums ${V3_TEXT.strong}`}>{value}</div></div>)}</div></div>
                </div>
                <div className="surface p-5">
                  <div className="flex items-center justify-between mb-3"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Feeds</div><Zap size={13} style={{ color: "var(--accent-2)" }} /></div>
                  {[["Code scanning", metrics.code_scanning_alerts || 0, "bg-orange-600"], ["Dependabot", metrics.dependency_alerts || 0, "bg-amber-500"], ["Owner scoped", metrics.owner_scoped || 0, "bg-slate-500"], ["Runtime exposed", metrics.runtime_exposed || 0, "bg-red-700"]].map(([label, value, dot], index) => <div key={label} className={`flex items-center justify-between py-2.5 ${index ? "border-t" : ""}`} style={index ? { borderColor: "var(--surface-border-2)" } : undefined}><span className={`flex items-center gap-2 text-[13px] ${V3_TEXT.body}`}><span className={`h-1.5 w-1.5 rounded-full ${dot}`} />{label}</span><span className={`font-display text-[15px] font-semibold tabular-nums ${V3_TEXT.strong}`}>{value}</span></div>)}
                </div>
                <div className="surface p-5">
                  <div className="flex items-center justify-between mb-3"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Recent scans</div><Cpu size={13} className={V3_TEXT.mute} /></div>
                  {history.slice(0, 4).map((item, index) => <button key={item.id} onClick={() => loadScan(item.id)} className={`w-full flex items-center justify-between py-2 text-left ${index ? "border-t" : ""}`} style={index ? { borderColor: "var(--surface-border-2)" } : undefined} type="button"><div><div className={`text-[13px] ${V3_TEXT.strong}`}>{item.repo}</div><div className={`text-[11px] ${V3_TEXT.mute}`}>{timeAgo(item.created_at)} ago · {countLabel(item.tracked_findings, "finding")}</div></div><span className={`text-[10px] uppercase tracking-wider px-2 py-0.5 rounded-full border ${item.fix_now ? RECOMMENDATION_CLASSES["fix now"] : item.plan_next ? RECOMMENDATION_CLASSES["plan next"] : RECOMMENDATION_CLASSES.watch}`}>{item.fix_now ? "hold" : item.plan_next ? "plan" : "watch"}</span></button>)}
                  {!history.length ? <div className={`py-6 text-center text-[12px] ${V3_TEXT.mute}`}>No saved scans yet.</div> : null}
                </div>
              </aside>
            </section>
          </>
        ) : null}
        {activeTab === "history" ? <HistoryTab history={history} loadScan={loadScan} /> : null}
        {activeTab === "checks" ? <ChecksTab checks={checks} health={health} onRefresh={() => refresh({ loadLatest: false })} /> : null}
        {activeTab === "sources" ? <SourcesTab form={form} health={health} onChange={setForm} onRun={runScan} repoInput={repoInput} running={running} /> : null}
      </div>
    </ProductShell>
  );
}

function HistoryTab({ history, loadScan }) {
  return <section className="surface p-6"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Saved evidence</div><h1 className={`font-display mt-2 text-[42px] tracking-tight font-semibold ${V3_TEXT.strong}`}>Scan history.</h1><div className="mt-7 space-y-2">{history.map((item) => <button className="surface-inset rounded-xl p-4 w-full text-left flex items-center justify-between" key={item.id} onClick={() => loadScan(item.id)} type="button"><div><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>{item.repo}</div><div className={`mt-1 text-[11px] ${V3_TEXT.mute}`}>{new Date(item.created_at).toLocaleString()} · {countLabel(item.tracked_findings, "finding")}</div></div><div className="flex gap-2"><span className={`text-[10px] rounded-full border px-2.5 py-1 ${RECOMMENDATION_CLASSES["fix now"]}`}>{item.fix_now || 0} fix now</span><ArrowUpRight size={14} className={V3_TEXT.dim} /></div></button>)}</div></section>;
}

function ChecksTab({ checks, health, onRefresh }) {
  return <div className="grid grid-cols-12 gap-6"><section className="surface col-span-8 p-6"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Startup evidence</div><h1 className={`font-display mt-2 text-[42px] tracking-tight font-semibold ${V3_TEXT.strong}`}>System checks.</h1><div className="mt-7 space-y-2">{checks.map((check, index) => <div className="surface-inset rounded-xl p-4 flex items-start justify-between gap-4" key={check.name || index}><div><div className={`text-[13px] ${V3_TEXT.strong}`}>{check.name || check.label || "Startup check"}</div><div className={`mt-1 text-[12px] ${V3_TEXT.mute}`}>{check.message || check.detail}</div></div><span className={`text-[10px] uppercase tracking-wider ${check.level === "error" ? "text-red-700 dark:text-red-300" : check.level === "warn" ? "text-amber-700 dark:text-amber-300" : "text-emerald-700 dark:text-emerald-300"}`}>{check.level || "ok"}</span></div>)}</div></section><aside className="surface col-span-4 p-6"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Backend</div><div className={`mt-3 font-display text-[46px] font-semibold ${V3_TEXT.strong}`}>{health.status || "unknown"}</div><dl className="mt-6 space-y-3"><SideValue label="Database" value={health.db_ok ? "ready" : "unavailable"} /><SideValue label="GitHub" value={health.github_ready ? "ready" : "token missing"} /><SideValue label="Saved scans" value={String(health.scan_count || 0)} /></dl><button onClick={onRefresh} className={`surface-inset mt-6 h-10 w-full rounded-full text-[12px] ${V3_TEXT.body}`} type="button">Refresh checks</button></aside></div>;
}

function SourcesTab({ form, health, onChange, onRun, repoInput, running }) {
  return <div className="grid grid-cols-12 gap-6"><section className="surface col-span-8 p-8"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>GitHub intake</div><h1 className={`font-display mt-2 text-[42px] tracking-tight font-semibold ${V3_TEXT.strong}`}>Choose a repository.</h1><p className={`mt-3 max-w-xl text-[14px] ${V3_TEXT.body}`}>VulnTriage reads security feeds without changing repository code.</p><div className="mt-7"><label className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`} htmlFor="repo">Repository</label><div className="surface-inset mt-2 rounded-xl h-12 px-4 flex items-center gap-2"><Github size={14} className={V3_TEXT.dim} /><input id="repo" ref={repoInput} value={form.repo} onChange={(event) => onChange((current) => ({ ...current, repo: event.target.value }))} placeholder="owner/repository" className={`bg-transparent outline-none w-full text-[13px] ${V3_TEXT.strong}`} /></div></div><div className="mt-5 grid grid-cols-2 gap-3">{[["include_code_scanning", "Code scanning alerts"], ["include_dependency_alerts", "Dependabot alerts"]].map(([key, label]) => <label className={`surface-inset rounded-xl p-4 flex items-center gap-3 text-[13px] ${V3_TEXT.body}`} key={key}><input checked={form[key]} onChange={(event) => onChange((current) => ({ ...current, [key]: event.target.checked }))} type="checkbox" className="accent-[color:var(--accent-2)]" />{label}</label>)}</div><button disabled={running || !form.repo.trim()} onClick={onRun} className="mt-6 h-11 px-5 rounded-full text-[12px] font-semibold text-white disabled:opacity-50" style={{ backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))", boxShadow: "var(--accent-glow)" }} type="button">{running ? "Scanning…" : "Run security scan"}</button></section><aside className="surface col-span-4 p-6 overflow-hidden"><div className="absolute -top-16 -right-12 h-48 w-48 rounded-full opacity-40 blur-2xl" style={{ backgroundImage: "var(--orb-1)" }} /><div className="relative"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Connection</div><div className={`mt-3 font-display text-[36px] font-semibold ${V3_TEXT.strong}`}>{health.github_ready ? "Ready" : "Needs token"}</div><p className={`mt-3 text-[13px] leading-relaxed ${V3_TEXT.body}`}>{health.github_ready ? "GitHub security feeds are available to this product." : "Configure a GitHub token with security-alert read access before scanning."}</p></div></aside></div>;
}

export default function App() {
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "vuln-triage_api_key" });
  if (!auth.checked) return <LoadingScreen />;
  if (auth.needsAuth) return <LoginScreen auth={auth} />;
  return <MainProduct auth={auth} />;
}
