import { useCallback, useEffect, useState } from "react";
import { KeyRound, ListChecks, Save, Webhook } from "lucide-react";
import {
  ControlButton,
  ControlField,
  ControlPanelTitle,
  ControlSelectField,
  GuidanceNotice,
  ProductControlSection,
  ProductControlsLayout,
  ProductControlsPair,
  ProductControlsSafetyBoundary,
  ProductScheduleManager,
  V3_TEXT,
} from "@patchhivehq/ui-v3";
import { readResponse, serializeRunParams } from "./shared.jsx";

const REPO_LISTS = [
  { value: "allowlist", label: "Allowlist" },
  { value: "denylist", label: "Denylist" },
  { value: "opt_out", label: "Opt-out" },
];

function ScopeFields({ mode, params, setParams }) {
  const set = (key) => (value) => setParams((current) => ({ ...current, [key]: value }));
  return <div className="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
    {mode === "direct" ? <ControlField label="Target repository" onChange={set("target_repo")} placeholder="owner/repository" value={params.target_repo}/> : <ControlField label="Discovery query" onChange={set("search_query")} placeholder="topic:rust is:public" value={params.search_query}/>} 
    <ControlField label="Language" onChange={set("language")} value={params.language}/>
    <ControlField label="Issue labels" onChange={set("labels")} value={params.labels}/>
    <ControlField label="Minimum stars" min="0" onChange={set("min_stars")} type="number" value={params.min_stars}/>
    <ControlField label="Repository cap" max="100" min="1" onChange={set("max_repos")} type="number" value={params.max_repos}/>
    <ControlField label="Issue cap" max="100" min="1" onChange={set("max_issues")} type="number" value={params.max_issues}/>
    <ControlField label="Concurrency" max="32" min="1" onChange={set("concurrency")} type="number" value={params.concurrency}/>
    <ControlField label="Cost budget USD" min="0" onChange={set("cost_budget_usd")} step="0.01" type="number" value={params.cost_budget_usd}/>
  </div>;
}

export default function ControlsPanel({
  apiBase,
  config,
  dryParams,
  dryTargetMode,
  fetcher,
  onError,
  onRefresh,
  params,
  setDryParams,
  setDryTargetMode,
  setParams,
  setTargetMode,
  targetMode,
  watchMode,
}) {
  const [repoLists, setRepoLists] = useState([]);
  const [repoControl, setRepoControl] = useState({ repo: "", list_type: "allowlist" });
  const [settings, setSettings] = useState({});
  const [authStatus, setAuthStatus] = useState({});
  const [generatedToken, setGeneratedToken] = useState("");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => setSettings({
    BOT_GITHUB_USER: config?.BOT_GITHUB_USER || "",
    BOT_GITHUB_EMAIL: config?.BOT_GITHUB_EMAIL || "",
    COST_BUDGET_USD: config?.COST_BUDGET_USD || "",
    MIN_REVIEW_CONFIDENCE: config?.MIN_REVIEW_CONFIDENCE || "",
    PATCHHIVE_AI_URL: config?.PATCHHIVE_AI_URL || "",
    OLLAMA_BASE_URL: config?.OLLAMA_BASE_URL || "",
    REPO_REAPER_GITHUB_TOKEN_RW: "",
    PROVIDER_API_KEY: "",
    WEBHOOK_SECRET: "",
  }), [config]);

  const refreshControls = useCallback(async () => {
    try {
      const [repos, auth] = await Promise.all([
        readResponse(await fetcher(`${apiBase}/repo-lists`), "Repository controls could not load"),
        readResponse(await fetcher(`${apiBase}/auth/status`), "Service auth status could not load"),
      ]);
      setRepoLists(repos.repos || []);
      setAuthStatus(auth);
    } catch (error) { onError(error.message); }
  }, [apiBase, fetcher, onError]);
  useEffect(() => { refreshControls(); }, [refreshControls]);

  async function perform(work, success) {
    setBusy(true); setMessage("");
    try { await work(); setMessage(success); await refreshControls(); await onRefresh(); }
    catch (error) { onError(error.message || "RepoReaper control update failed."); }
    finally { setBusy(false); }
  }

  function loadSchedule(setter, modeSetter) {
    return (payload, mode) => {
      setter((current) => ({ ...current, ...Object.fromEntries(Object.entries(payload || {}).map(([key, value]) => [key, Array.isArray(value) ? value.join(", ") : String(value ?? "")])) }));
      modeSetter(mode);
    };
  }

  async function serviceToken(path) {
    await perform(async () => {
      const payload = await readResponse(await fetcher(`${apiBase}${path}`, { method: "POST" }), "Service token update failed");
      setGeneratedToken(payload.service_token || "");
    }, "Suite service token updated. Copy it now; it will not be shown again.");
  }

  return <div className="mx-auto max-w-[1440px] px-3 py-6 sm:px-6">
    <ProductControlsLayout eyebrow="Mission operations" description="Configure explicit direct or autonomous work, schedules, repository boundaries, watch mode, and restart-backed runtime defaults." message={message}>
      <ProductControlsPair>
        <ProductScheduleManager actionLabel="patch mission" apiBase={`${apiBase}/automation/run`} currentPayload={serializeRunParams(params, targetMode)} description="Recurring write-capable missions remain bounded by repository policy, test proof, review confidence, existing-PR checks, and suite PR budgets." eyebrow="Write schedules" fetcher={fetcher} onError={onError} onLoadPayload={loadSchedule(setParams, setTargetMode)} onRefresh={onRefresh} onRunComplete={onRefresh} onTargetSelectionModeChange={setTargetMode} productName="RepoReaper" safetyNote="Enabling this schedule is recurring authorization to attempt a bounded mission. It is never authorization to bypass a failed gate." targetConfiguration={<ScopeFields mode={targetMode} params={params} setParams={setParams}/>} targetSelectionMode={targetMode} title="Patch mission schedules."/>
        <ProductScheduleManager actionLabel="Dry Stalk" apiBase={`${apiBase}/automation/dry_run`} currentPayload={serializeRunParams(dryParams, dryTargetMode)} description="Schedule no-write issue discovery and Scout analysis independently from patch execution." eyebrow="Read-only schedules" fetcher={fetcher} onError={onError} onLoadPayload={loadSchedule(setDryParams, setDryTargetMode)} onRefresh={onRefresh} onRunComplete={onRefresh} onTargetSelectionModeChange={setDryTargetMode} productName="RepoReaper" safetyNote="Dry Stalk schedules discover and score evidence. They never clone for editing, run tests, push branches, or open pull requests." targetConfiguration={<ScopeFields mode={dryTargetMode} params={dryParams} setParams={setDryParams}/>} targetSelectionMode={dryTargetMode} title="Dry Stalk schedules."/>
      </ProductControlsPair>

      <ProductControlSection>
        <ControlPanelTitle icon={ListChecks} subtitle="Opt-outs and denylists always win. Once an allowlist exists, autonomous discovery is constrained to it.">Repository controls</ControlPanelTitle>
        <div className="mt-5 grid gap-3 sm:grid-cols-[minmax(0,1fr)_220px_auto]"><ControlField label="Repository" onChange={(repo) => setRepoControl((current) => ({ ...current, repo }))} placeholder="owner/repository" value={repoControl.repo}/><ControlSelectField label="Control" onChange={(list_type) => setRepoControl((current) => ({ ...current, list_type }))} options={REPO_LISTS} value={repoControl.list_type}/><div className="flex items-end"><ControlButton disabled={busy || !/^[^/\s]+\/[^/\s]+$/.test(repoControl.repo.trim())} onClick={() => perform(async () => { await readResponse(await fetcher(`${apiBase}/repo-lists`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ repo: repoControl.repo.trim(), list_type: repoControl.list_type }) }), "Repository control could not save"); setRepoControl((current) => ({ ...current, repo: "" })); }, "Repository control saved.")} primary>Save control</ControlButton></div></div>
        <div className="mt-5 space-y-2">{repoLists.map((entry) => <div className="surface-inset flex flex-col gap-3 rounded-xl p-3 sm:flex-row sm:items-center sm:justify-between" key={entry.repo}><div><div className={`font-display text-[14px] ${V3_TEXT.strong}`}>{entry.repo}</div><div className={`mt-0.5 text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>{entry.list_type.replaceAll("_", "-")}</div></div><ControlButton disabled={busy} onClick={() => perform(async () => readResponse(await fetcher(`${apiBase}/repo-lists/${encodeURIComponent(entry.repo)}`, { method: "DELETE" }), "Repository control could not be removed"), `Repository control removed: ${entry.repo}.`)} tone="danger">Remove</ControlButton></div>)}{!repoLists.length ? <div className={`py-8 text-center text-[12px] ${V3_TEXT.mute}`}>No repository-specific controls are saved.</div> : null}</div>
      </ProductControlSection>

      <ProductControlsPair>
        <ProductControlSection>
          <ControlPanelTitle icon={Webhook} subtitle="Watch mode listens for signed GitHub events and may start a guarded write operation.">Webhook posture</ControlPanelTitle>
          <div className="mt-5 flex flex-wrap gap-2"><ControlButton disabled={busy} onClick={() => perform(async () => readResponse(await fetcher(`${apiBase}/watch-mode`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ enabled: !watchMode }) }), "Watch mode could not change"), `Watch mode ${watchMode ? "disabled" : "enabled"}.`)} primary>{watchMode ? "Disable watch mode" : "Enable watch mode"}</ControlButton></div>
          <GuidanceNotice label="Signed delivery">Watch mode still rejects delivery unless WEBHOOK_SECRET is configured and the signature verifies. Webhooks share the one-operation engine lock and normal write gates.</GuidanceNotice>
        </ProductControlSection>
        <ProductControlSection>
          <ControlPanelTitle icon={KeyRound} subtitle="Generate or rotate the credential HiveCore uses to dispatch RepoReaper.">Suite service token</ControlPanelTitle>
          <div className="mt-5 flex flex-wrap gap-2"><ControlButton disabled={busy || authStatus.service_auth_enabled} onClick={() => serviceToken("/auth/generate-service-token")} primary>Generate token</ControlButton><ControlButton disabled={busy || !authStatus.service_auth_enabled} onClick={() => serviceToken("/auth/rotate-service-token")}>Rotate token</ControlButton>{generatedToken ? <ControlButton onClick={() => navigator.clipboard.writeText(generatedToken)}>Copy token</ControlButton> : null}</div>
          {generatedToken ? <code className={`surface-inset mt-4 block break-all rounded-xl p-3 text-[11px] ${V3_TEXT.strong}`}>{generatedToken}</code> : null}
          <GuidanceNotice label="Separate credentials">This is not the browser API key or GitHub write PAT. It exists only for authenticated suite-to-product dispatch.</GuidanceNotice>
        </ProductControlSection>
      </ProductControlsPair>

      <ProductControlSection>
        <ControlPanelTitle icon={Save} subtitle="Runtime configuration is written to the canonical PatchHive .env and takes effect after backend restart.">Runtime defaults</ControlPanelTitle>
        <div className="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          <ControlField label="Bot GitHub user" onChange={(value) => setSettings((current) => ({ ...current, BOT_GITHUB_USER: value }))} value={settings.BOT_GITHUB_USER}/><ControlField label="Bot Git email" onChange={(value) => setSettings((current) => ({ ...current, BOT_GITHUB_EMAIL: value }))} value={settings.BOT_GITHUB_EMAIL}/><ControlField label="Cost budget USD" onChange={(value) => setSettings((current) => ({ ...current, COST_BUDGET_USD: value }))} type="number" value={settings.COST_BUDGET_USD}/><ControlField label="Minimum review confidence" onChange={(value) => setSettings((current) => ({ ...current, MIN_REVIEW_CONFIDENCE: value }))} type="number" value={settings.MIN_REVIEW_CONFIDENCE}/><ControlField label="PatchHive AI URL" onChange={(value) => setSettings((current) => ({ ...current, PATCHHIVE_AI_URL: value }))} value={settings.PATCHHIVE_AI_URL}/><ControlField label="Ollama URL" onChange={(value) => setSettings((current) => ({ ...current, OLLAMA_BASE_URL: value }))} value={settings.OLLAMA_BASE_URL}/><ControlField label={`GitHub write PAT${config?.REPO_REAPER_GITHUB_TOKEN_RW_SET ? " · configured" : ""}`} onChange={(value) => setSettings((current) => ({ ...current, REPO_REAPER_GITHUB_TOKEN_RW: value }))} placeholder="Leave blank to preserve" type="password" value={settings.REPO_REAPER_GITHUB_TOKEN_RW}/><ControlField label={`Provider key${config?.PROVIDER_API_KEY_SET ? " · configured" : ""}`} onChange={(value) => setSettings((current) => ({ ...current, PROVIDER_API_KEY: value }))} placeholder="Leave blank to preserve" type="password" value={settings.PROVIDER_API_KEY}/><ControlField label={`Webhook secret${config?.WEBHOOK_SECRET_SET ? " · configured" : ""}`} onChange={(value) => setSettings((current) => ({ ...current, WEBHOOK_SECRET: value }))} placeholder="Leave blank to preserve" type="password" value={settings.WEBHOOK_SECRET}/>
        </div><div className="mt-5"><ControlButton disabled={busy} onClick={() => perform(async () => readResponse(await fetcher(`${apiBase}/config`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(settings) }), "Configuration could not save"), "Configuration saved. Restart the unified backend before relying on the new values.")} primary>Save configuration</ControlButton></div>
      </ProductControlSection>

      <ProductControlsSafetyBoundary cards={[{ title: "Explicit target mode", body: "Target repo never falls through to broad discovery. Autonomous discovery is selected and stored deliberately." }, { title: "One active operation", body: "Operator, schedule, webhook, and suite dispatch share one RepoReaper operation lock." }, { title: "Guarded writes", body: "A mission may open a PR only after policy, testing, review confidence, existing-PR, attribution, and suite-budget checks pass." }]} subtitle="These controls authorize bounded work. They do not bypass RepoReaper's write, test, or repository safety gates."/>
    </ProductControlsLayout>
  </div>;
}
