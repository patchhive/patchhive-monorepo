import { useCallback, useEffect, useState } from "react";
import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  Database,
  Loader2,
  Plus,
  RefreshCw,
  Save,
  Settings,
  ShieldCheck,
  Trash2,
  XCircle,
} from "lucide-react";
import { toast } from "sonner";

import {
  fetchDiagnostics,
  fetchFleetRuntime,
  fetchPrBudgets,
  fetchRepositoryPolicies,
  fetchSettings,
  releasePrReservation,
  savePrBudgets,
  saveRepositoryPolicies,
  saveSettings,
  type PrBudgetStatus,
  type RepositoryPoliciesResponse,
  type SettingsResponse,
  type ProductRuntimeDetail,
  type RuntimeObservation,
} from "@/lib/control-plane";

/**
 * The v1 settings surface, fitted into the v3 deck without becoming a second app.
 * Sections stay collapsed until the operator needs them; the saved policy is still
 * loaded together so a single refresh reports whether every control-plane store is
 * readable. Secret replacement inputs are write-only and clear after a save.
 */
export function ControlCenter({ syncVersion = 0 }: { syncVersion?: number }) {
  const [settings, setSettings] = useState<SettingsResponse | null>(null);
  const [policies, setPolicies] = useState<RepositoryPoliciesResponse | null>(null);
  const [budgets, setBudgets] = useState<PrBudgetStatus | null>(null);
  const [diagnostics, setDiagnostics] = useState<Awaited<ReturnType<typeof fetchDiagnostics>> | null>(null);
  const [fleet, setFleet] = useState<ProductRuntimeDetail[]>([]);
  const [newRepository, setNewRepository] = useState("");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");

  const load = useCallback((signal?: AbortSignal) => {
    setLoading(true);
    Promise.all([
      fetchSettings(signal),
      fetchRepositoryPolicies(signal),
      fetchPrBudgets(signal),
      fetchDiagnostics(signal),
      fetchFleetRuntime(signal),
    ])
      .then(([nextSettings, nextPolicies, nextBudgets, nextDiagnostics, nextFleet]) => {
        setSettings(nextSettings);
        setPolicies(nextPolicies);
        setBudgets(nextBudgets);
        setDiagnostics(nextDiagnostics);
        setFleet(nextFleet);
        setError("");
      })
      .catch((cause) => {
        if (cause instanceof DOMException && cause.name === "AbortError") return;
        setError(cause instanceof Error ? cause.message : "Could not read HiveCore controls.");
      })
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    load(controller.signal);
    return () => controller.abort();
  }, [load, syncVersion]);

  async function persistAll() {
    if (!settings || !policies || !budgets || busy) return;
    setBusy("save");
    setError("");
    const completed: string[] = [];
    try {
      // The legacy allow/deny text and the structured policy editor are views over
      // the same SQLite rows. Keep structured rows canonical and write sequentially
      // so the two endpoints cannot race and replace each other's policy kinds.
      const settingsWithCanonicalLists = {
        ...settings,
        suite_settings: {
          ...settings.suite_settings,
          repo_allowlist: repositoryListText(policies, "allowlisted"),
          repo_denylist: repositoryListText(policies, "operator_excluded"),
        },
      };
      const nextSettings = await saveSettings(settingsWithCanonicalLists);
      completed.push("suite settings");
      const nextPolicies = await saveRepositoryPolicies(policies);
      completed.push("repository policy");
      const nextBudgets = await savePrBudgets(budgets);
      completed.push("PR budgets");
      setSettings(nextSettings);
      setPolicies(nextPolicies);
      setBudgets(nextBudgets);
      toast.success("Control-plane policy saved", {
        description: "Suite defaults, repository policy, product overrides, and PR budgets are durable.",
      });
    } catch (cause) {
      const reason = cause instanceof Error ? cause.message : "HiveCore could not save policy.";
      const message = completed.length > 0
        ? `${completed.join(" and ")} saved; the remaining save stopped: ${reason}`
        : reason;
      setError(message);
      toast.error("Policy save failed", { description: message });
      load();
    } finally {
      setBusy("");
    }
  }

  async function releaseReservation(id: string) {
    if (busy) return;
    setBusy(id);
    try {
      await releasePrReservation(id);
      setBudgets(await fetchPrBudgets());
      toast.success("PR slot released");
    } catch (cause) {
      toast.error("Could not release PR slot", {
        description: cause instanceof Error ? cause.message : String(cause),
      });
    } finally {
      setBusy("");
    }
  }

  function addRepository() {
    const repository = newRepository.trim().toLowerCase();
    if (!repository || !policies || policies.policies.some((item) => item.repository === repository)) return;
    setPolicies({
      ...policies,
      policies: [
        ...policies.policies,
        {
          repository,
          trusted: false,
          operator_excluded: false,
          allowlisted: false,
          public_opt_out: false,
          source: "operator",
          notes: "",
          updated_at: "",
        },
      ],
    });
    setNewRepository("");
  }

  const startupErrors = diagnostics?.startup.filter((check) => check.level === "error").length ?? 0;
  const startupWarnings = diagnostics?.startup.filter((check) => check.level === "warn").length ?? 0;

  return (
    <section id="controls" className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <Settings className="h-4 w-4 text-[var(--honey)]" /> Control Center
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Durable suite policy, repository safety, product endpoints, and outbound limits.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={() => load()} disabled={loading || Boolean(busy)} className={buttonClass}>
            <RefreshCw className={`h-3 w-3 ${loading ? "animate-spin" : ""}`} /> Reload
          </button>
          <button onClick={persistAll} disabled={!settings || !policies || !budgets || Boolean(busy)} className={primaryButtonClass}>
            {busy === "save" ? <Loader2 className="h-3 w-3 animate-spin" /> : <Save className="h-3 w-3" />} Save all
          </button>
        </div>
      </div>

      {error && <p className="mt-3 rounded border border-[var(--crit)]/40 bg-[var(--crit)]/[0.06] p-2 text-xs text-[var(--crit)]">{error}</p>}

      <details open className="group mt-4 rounded-lg border border-border bg-background/40">
        <Summary icon={Activity} title="HiveCore diagnostics" detail="Health, database, and startup evidence" />
        <div className="grid gap-2 border-t border-border/60 p-3 sm:grid-cols-3">
          <Diagnostic label="Suite backend" ok={diagnostics?.suite.status === "ok"} value={diagnostics ? `${diagnostics.suite.status} · ${diagnostics.suite.enabled_products} enabled` : "not observed"} />
          <Diagnostic label="HiveCore engine" ok={diagnostics?.hiveCore.status === "ok"} value={diagnostics ? `${diagnostics.hiveCore.status} · v${diagnostics.hiveCore.version}` : "not observed"} />
          <Diagnostic label="SQLite" ok={diagnostics?.hiveCore.db_ok === true} value={diagnostics ? (diagnostics.hiveCore.db_ok ? diagnostics.hiveCore.db_path : "failed") : "not observed"} />
          <Diagnostic label="Startup checks" ok={startupErrors === 0} value={diagnostics ? `${startupErrors} errors · ${startupWarnings} warnings` : "not observed"} />
          <Diagnostic label="Repository policy" ok={Boolean(diagnostics)} value={diagnostics ? `${diagnostics.hiveCore.repository_policy_count} records` : "not observed"} />
          <Diagnostic label="Suite PR ceiling" ok={diagnostics?.hiveCore.suite_pr_limit !== null && diagnostics?.hiveCore.suite_pr_limit !== undefined} value={diagnostics?.hiveCore.suite_pr_limit == null ? "not observed" : String(diagnostics.hiveCore.suite_pr_limit)} />
        </div>
        {diagnostics && diagnostics.startup.length > 0 && (
          <div className="border-t border-border/60 px-3 py-2">
            {diagnostics.startup.map((check, index) => (
              <div key={`${check.code ?? check.msg}-${index}`} className="flex flex-wrap items-center gap-2 py-1 text-[11px]">
                <span className={`rounded border px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider ${check.level === "error" ? "border-[var(--crit)]/40 text-[var(--crit)]" : check.level === "warn" ? "border-[var(--warn)]/40 text-[var(--warn)]" : "border-[var(--ok)]/40 text-[var(--ok)]"}`}>{check.level}</span>
                {check.code && <code className="text-[10px] text-muted-foreground">{check.code}:{check.status ?? "unknown"}</code>}
                <span className="text-muted-foreground">{check.msg}</span>
              </div>
            ))}
          </div>
        )}
      </details>

      <details className="group mt-2 rounded-lg border border-border bg-background/40">
        <Summary icon={Database} title="Fleet contract evidence" detail={`${fleet.length} products · health, startup, auth, and route checks`} />
        <div className="grid gap-2 border-t border-border/60 p-3 md:grid-cols-2">
          {fleet.length === 0 && <p className="text-xs text-muted-foreground">Fleet contract evidence has not been observed.</p>}
          {fleet.map((product) => (
            <details key={product.slug} className="rounded border border-border/70 bg-background/40 p-2">
              <summary className="cursor-pointer list-none">
                <div className="flex flex-wrap items-center gap-2">
                  <span className={`h-1.5 w-1.5 rounded-full ${product.status === "online" ? "bg-[var(--ok)]" : product.status === "degraded" ? "bg-[var(--warn)]" : "bg-muted-foreground"}`} />
                  <strong className="font-display text-xs">{product.title}</strong>
                  <span className="rounded border border-border px-1.5 py-0.5 font-display text-[8px] uppercase text-muted-foreground">{product.status}</span>
                  <span className="ml-auto text-[9px] text-muted-foreground">{product.auth_mode} · {product.machine_auth_configured ? "machine auth ready" : "machine auth missing"}</span>
                </div>
              </summary>
              <div className="mt-2 grid gap-1 sm:grid-cols-2">
                <Evidence label="health" evidence={product.health.health_endpoint} format={(value) => `${observationText(value.reported_status)} · ${value.latency_ms}ms`} />
                <Evidence label="version" evidence={product.health.version} />
                <Evidence label="database" evidence={product.health.database_ok} format={(value) => value ? "ok" : "failed"} />
                <Evidence label="startup" evidence={product.health.startup_checks} format={(value) => `${value.errors} errors · ${value.warnings} warnings · ${value.infos} info`} />
                <Evidence label="actions" evidence={product.health.capabilities} format={(value) => String(value.action_count)} />
                <Evidence label="runs" evidence={product.health.runs} format={(value) => String(value.run_count)} />
              </div>
              <div className="mt-2 space-y-1">
                {product.contract_checks.map((check) => (
                  <div key={check.id} className="flex flex-wrap items-center gap-2 rounded border border-border/60 px-2 py-1 text-[9px]">
                    <span className={check.ok ? "text-[var(--ok)]" : "text-[var(--warn)]"}>{check.ok ? "pass" : check.status}</span>
                    <code>{check.path}</code>
                    <span className="flex-1 text-muted-foreground">{check.error || check.label}</span>
                  </div>
                ))}
              </div>
              <div className="mt-2 flex flex-wrap gap-3 font-mono text-[9px] text-muted-foreground">
                {product.frontend_url && <a href={product.frontend_url} target="_blank" rel="noreferrer" className="hover:text-[var(--honey)]">frontend</a>}
                {product.api_url && <a href={`${product.api_url.replace(/\/$/, "")}/health`} target="_blank" rel="noreferrer" className="hover:text-[var(--honey)]">health endpoint</a>}
                <span>checked {new Date(product.health.checked_at).toLocaleString()}</span>
              </div>
            </details>
          ))}
        </div>
      </details>

      {settings && (
        <details className="group mt-2 rounded-lg border border-border bg-background/40">
          <Summary icon={Settings} title="Suite defaults" detail={`Updated ${new Date(settings.suite_settings.updated_at).toLocaleString()}`} />
          <div className="grid gap-3 border-t border-border/60 p-3 sm:grid-cols-2">
            <Field label="Operator label" value={settings.suite_settings.operator_label} onChange={(value) => setSettings({ ...settings, suite_settings: { ...settings.suite_settings, operator_label: value } })} />
            <label className={labelClass}>Preferred launch product<select value={settings.suite_settings.preferred_launch_product} onChange={(event) => setSettings({ ...settings, suite_settings: { ...settings.suite_settings, preferred_launch_product: event.target.value } })} className={inputClass}>{settings.products.map((product) => <option key={product.slug} value={product.slug}>{product.title}</option>)}</select></label>
            <Field label="Default topics" value={settings.suite_settings.default_topics} onChange={(value) => setSettings({ ...settings, suite_settings: { ...settings.suite_settings, default_topics: value } })} />
            <Field label="Default languages" value={settings.suite_settings.default_languages} onChange={(value) => setSettings({ ...settings, suite_settings: { ...settings.suite_settings, default_languages: value } })} />
            <Area label="Mission" value={settings.suite_settings.mission} onChange={(value) => setSettings({ ...settings, suite_settings: { ...settings.suite_settings, mission: value } })} />
            <Area label="Operator notes" value={settings.suite_settings.notes} onChange={(value) => setSettings({ ...settings, suite_settings: { ...settings.suite_settings, notes: value } })} />
            <Area label="Repository allowlist (managed below)" value={policies ? repositoryListText(policies, "allowlisted") : settings.suite_settings.repo_allowlist} readOnly />
            <Area label="Repository denylist (managed below)" value={policies ? repositoryListText(policies, "operator_excluded") : settings.suite_settings.repo_denylist} readOnly />
            <div className="sm:col-span-2"><Area label="Opt-out notes" value={settings.suite_settings.opt_out_notes} onChange={(value) => setSettings({ ...settings, suite_settings: { ...settings.suite_settings, opt_out_notes: value } })} /></div>
          </div>
        </details>
      )}

      {policies && (
        <details className="group mt-2 rounded-lg border border-border bg-background/40">
          <Summary icon={ShieldCheck} title="Repository safety" detail={`${policies.policies.length} policies · public opt-outs remain read-only`} />
          <div className="space-y-2 border-t border-border/60 p-3">
            <div className="flex gap-2"><input value={newRepository} onChange={(event) => setNewRepository(event.target.value)} placeholder="owner/repository" className={inputClass} /><button onClick={addRepository} className={buttonClass}><Plus className="h-3 w-3" /> Add</button></div>
            {policies.policies.length === 0 && <p className="text-xs text-muted-foreground">No structured repository policies saved.</p>}
            {policies.policies.map((policy, index) => {
              const locked = policy.public_opt_out;
              return (
                <div key={`${policy.repository}-${policy.source}`} className="rounded border border-border/70 p-2">
                  <div className="flex flex-wrap items-center gap-3">
                    <strong className="font-mono text-xs">{policy.repository}</strong>
                    {policy.public_opt_out && <span className="rounded border border-[var(--crit)]/40 px-1.5 py-0.5 font-display text-[9px] uppercase text-[var(--crit)]">public opt-out</span>}
                    <Toggle label="trusted" checked={policy.trusted} disabled={locked} onChange={(checked) => setPolicies({ ...policies, policies: policies.policies.map((item, itemIndex) => itemIndex === index ? { ...item, trusted: checked, operator_excluded: checked ? false : item.operator_excluded } : item) })} />
                    <Toggle label="allowlisted" checked={policy.allowlisted} disabled={locked} onChange={(checked) => setPolicies({ ...policies, policies: policies.policies.map((item, itemIndex) => itemIndex === index ? { ...item, allowlisted: checked } : item) })} />
                    <Toggle label="exclude automation" checked={policy.operator_excluded} disabled={locked} onChange={(checked) => setPolicies({ ...policies, policies: policies.policies.map((item, itemIndex) => itemIndex === index ? { ...item, operator_excluded: checked, trusted: checked ? false : item.trusted } : item) })} />
                    {!locked && <button aria-label={`Remove ${policy.repository}`} onClick={() => setPolicies({ ...policies, policies: policies.policies.filter((_, itemIndex) => itemIndex !== index) })} className="ml-auto text-muted-foreground hover:text-[var(--crit)]"><Trash2 className="h-3.5 w-3.5" /></button>}
                  </div>
                  {!locked && <input value={policy.notes} onChange={(event) => setPolicies({ ...policies, policies: policies.policies.map((item, itemIndex) => itemIndex === index ? { ...item, notes: event.target.value } : item) })} placeholder="Reason or operator note" className={`${inputClass} mt-2`} />}
                </div>
              );
            })}
          </div>
        </details>
      )}

      {budgets && (
        <details className="group mt-2 rounded-lg border border-border bg-background/40">
          <Summary icon={Activity} title="Pull-request budgets" detail={`${budgets.suite_used} used · ${budgets.suite_remaining} remaining suite-wide`} />
          <div className="border-t border-border/60 p-3">
            <div className="max-w-xs"><NumberField label="Suite-wide PR ceiling" value={budgets.suite_limit} onChange={(value) => setBudgets({ ...budgets, suite_limit: value })} /></div>
            <div className="mt-3 grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
              {budgets.products.map((product, index) => <div key={product.product} className="rounded border border-border/70 p-2"><NumberField label={product.product} value={product.limit} onChange={(value) => setBudgets({ ...budgets, products: budgets.products.map((item, itemIndex) => itemIndex === index ? { ...item, limit: value } : item) })} /><div className="mt-1 text-[10px] text-muted-foreground">{product.used} used · {product.remaining} remaining</div></div>)}
            </div>
            <div className="mt-3 space-y-1">
              <div className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">Recent reservations</div>
              {budgets.reservations.length === 0 && <p className="text-xs text-muted-foreground">No PR reservations recorded.</p>}
              {budgets.reservations.map((reservation) => <div key={reservation.id} className="flex flex-wrap items-center gap-2 rounded border border-border/70 px-2 py-1.5 text-[11px]"><span className="rounded border border-border px-1.5 py-0.5 font-display text-[9px] uppercase text-muted-foreground">{reservation.lifecycle.state}</span><strong>{reservation.product}</strong><code className="text-[10px] text-muted-foreground">{reservation.repository}</code><span className="flex-1 truncate text-muted-foreground">{reservation.run_id}</span>{(reservation.lifecycle.state === "reserved" || reservation.lifecycle.state === "committed") && <button onClick={() => releaseReservation(reservation.id)} disabled={Boolean(busy)} className={buttonClass}>{busy === reservation.id && <Loader2 className="h-3 w-3 animate-spin" />} Release</button>}</div>)}
            </div>
          </div>
        </details>
      )}

      {settings && (
        <details className="group mt-2 rounded-lg border border-border bg-background/40">
          <Summary icon={Database} title="Product endpoints" detail="Enablement, URL overrides, machine auth, and notes" />
          <div className="grid gap-2 border-t border-border/60 p-3 md:grid-cols-2">
            {settings.products.map((product, index) => (
              <details key={product.slug} className="rounded border border-border/70 bg-background/40 p-2">
                <summary className="cursor-pointer list-none"><div className="flex items-center gap-2"><span className={`h-1.5 w-1.5 rounded-full ${product.enabled ? "bg-[var(--ok)]" : "bg-muted-foreground"}`} /><strong className="font-display text-xs">{product.title}</strong><span className="ml-auto text-[10px] text-muted-foreground">{product.auth_mode}</span></div></summary>
                <div className="mt-3 space-y-2">
                  <Toggle label="enabled" checked={product.enabled} onChange={(checked) => setSettings({ ...settings, products: settings.products.map((item, itemIndex) => itemIndex === index ? { ...item, enabled: checked } : item) })} />
                  <Field label="Frontend URL override" value={product.override_frontend_url} placeholder={product.default_frontend_url} onChange={(value) => setSettings({ ...settings, products: settings.products.map((item, itemIndex) => itemIndex === index ? { ...item, override_frontend_url: value } : item) })} />
                  <Field label="API URL override" value={product.override_api_url} placeholder={product.default_api_url} onChange={(value) => setSettings({ ...settings, products: settings.products.map((item, itemIndex) => itemIndex === index ? { ...item, override_api_url: value } : item) })} />
                  {product.slug !== "hive-core" && <Field label="Replace service token (write-only)" type="password" value={product.service_token ?? ""} placeholder={product.service_token_configured ? "Configured — enter only to replace" : "Not configured"} onChange={(value) => setSettings({ ...settings, products: settings.products.map((item, itemIndex) => itemIndex === index ? { ...item, service_token: value } : item) })} />}
                  <Area label="Notes" value={product.notes} onChange={(value) => setSettings({ ...settings, products: settings.products.map((item, itemIndex) => itemIndex === index ? { ...item, notes: value } : item) })} />
                </div>
              </details>
            ))}
          </div>
        </details>
      )}
    </section>
  );
}

const inputClass = "w-full rounded border border-border bg-background/60 px-2 py-1.5 text-xs text-foreground outline-none focus:border-[var(--honey)]/50";
const labelClass = "grid gap-1 font-display text-[9px] uppercase tracking-wider text-muted-foreground";
const buttonClass = "inline-flex shrink-0 items-center gap-1.5 rounded border border-border px-2 py-1.5 font-display text-[9px] uppercase tracking-wider text-muted-foreground transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)] disabled:opacity-40";
const primaryButtonClass = "inline-flex items-center gap-1.5 rounded border border-[var(--honey)]/50 bg-[var(--honey)]/10 px-2.5 py-1.5 font-display text-[9px] uppercase tracking-wider text-[var(--honey)] transition hover:brightness-125 disabled:opacity-40";

function Summary({ icon: Icon, title, detail }: { icon: typeof Settings; title: string; detail: string }) {
  return <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-2.5"><Icon className="h-3.5 w-3.5 text-[var(--honey)]" /><strong className="font-display text-xs">{title}</strong><span className="ml-auto text-[10px] text-muted-foreground">{detail}</span><ChevronDown className="h-3 w-3 text-muted-foreground transition group-open:rotate-180" /></summary>;
}

function Diagnostic({ label, ok, value }: { label: string; ok: boolean; value: string }) {
  const Icon = ok ? CheckCircle2 : value === "not observed" ? AlertTriangle : XCircle;
  return <div className="rounded border border-border/70 p-2"><div className="flex items-center gap-1.5 font-display text-[9px] uppercase tracking-wider text-muted-foreground"><Icon className={`h-3 w-3 ${ok ? "text-[var(--ok)]" : "text-[var(--warn)]"}`} />{label}</div><div className="mt-1 truncate font-mono text-[10px] text-foreground" title={value}>{value}</div></div>;
}

function Field({ label, value, onChange, placeholder = "", type = "text" }: { label: string; value: string; onChange: (value: string) => void; placeholder?: string; type?: string }) {
  return <label className={labelClass}>{label}<input type={type} value={value} onChange={(event) => onChange(event.target.value)} placeholder={placeholder} autoComplete="off" className={inputClass} /></label>;
}

function Area({ label, value, onChange, readOnly = false }: { label: string; value: string; onChange?: (value: string) => void; readOnly?: boolean }) {
  return <label className={labelClass}>{label}<textarea value={value} readOnly={readOnly} onChange={(event) => onChange?.(event.target.value)} rows={3} className={`${inputClass} resize-y normal-case ${readOnly ? "cursor-default opacity-70" : ""}`} /></label>;
}

function NumberField({ label, value, onChange }: { label: string; value: number; onChange: (value: number) => void }) {
  return <label className={labelClass}>{label}<input type="number" min={0} value={value} onChange={(event) => onChange(Math.max(0, Number(event.target.value) || 0))} className={inputClass} /></label>;
}

function Toggle({ label, checked, onChange, disabled = false }: { label: string; checked: boolean; onChange: (checked: boolean) => void; disabled?: boolean }) {
  return <label className="inline-flex items-center gap-1.5 font-display text-[9px] uppercase tracking-wider text-muted-foreground"><input type="checkbox" checked={checked} disabled={disabled} onChange={(event) => onChange(event.target.checked)} />{label}</label>;
}

function observationText<T>(evidence: RuntimeObservation<T>, format: (value: T) => string = String): string {
  return evidence.state === "observed" ? format(evidence.value) : `${evidence.state.replaceAll("_", " ")} · ${evidence.reason}`;
}

function Evidence<T>({ label, evidence, format = String }: { label: string; evidence: RuntimeObservation<T>; format?: (value: T) => string }) {
  return <div className="rounded border border-border/60 px-2 py-1"><div className="font-display text-[8px] uppercase tracking-wider text-muted-foreground">{label} · {evidence.state.replaceAll("_", " ")}</div><div className="mt-0.5 truncate font-mono text-[9px] text-foreground" title={observationText(evidence, format)}>{observationText(evidence, format)}</div></div>;
}

function repositoryListText(
  value: RepositoryPoliciesResponse,
  field: "allowlisted" | "operator_excluded",
): string {
  return value.policies
    .filter((policy) => policy[field])
    .map((policy) => policy.repository)
    .join(", ");
}
