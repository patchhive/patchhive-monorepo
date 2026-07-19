import { useCallback, useEffect, useState } from "react";
import {
  CalendarClock,
  FolderSearch,
  KeyRound,
  ListChecks,
  LoaderCircle,
  Save,
} from "lucide-react";
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
  V3_TEXT,
  countLabel,
  readJson,
} from "@patchhivehq/ui-v3";

const CADENCES = [
  { value: "1", label: "Hourly" },
  { value: "6", label: "Every 6 hours" },
  { value: "12", label: "Every 12 hours" },
  { value: "24", label: "Daily" },
  { value: "168", label: "Weekly" },
];
const TARGET_MODES = [
  { value: "direct", label: "Target repo" },
  { value: "discovery", label: "Autonomous discovery" },
];
const REPO_LISTS = [
  { value: "allowlist", label: "Allowlist" },
  { value: "denylist", label: "Denylist" },
  { value: "opt_out", label: "Opt-out" },
];

function scopeList(value) {
  return String(value || "")
    .split(/[\n,]+/)
    .map((part) => part.trim())
    .filter(Boolean);
}

function normalizedPayload({
  cooldownDays,
  languages,
  maxFiles,
  minStars,
  query,
  targetRepo,
  targetSelectionMode,
  topics,
}) {
  return {
    repo_path: targetSelectionMode === "direct" ? targetRepo.trim() : "",
    max_files: Number(maxFiles) || 250,
    discovery: {
      query: query.trim(),
      topics: scopeList(topics),
      languages: scopeList(languages),
      min_stars: Number(minStars) || 25,
      cooldown_days: Number(cooldownDays) || 30,
    },
  };
}

export default function ControlsPanel({
  apiBase,
  fetcher,
  form,
  health,
  onError,
  onLoad,
  onRefresh,
  setForm,
}) {
  const [presets, setPresets] = useState([]);
  const [schedules, setSchedules] = useState([]);
  const [repoLists, setRepoLists] = useState([]);
  const [authStatus, setAuthStatus] = useState({});
  const [presetName, setPresetName] = useState("");
  const [selectedPreset, setSelectedPreset] = useState("");
  const [scheduleName, setScheduleName] = useState("");
  const [selectedSchedule, setSelectedSchedule] = useState("");
  const [cadence, setCadence] = useState("24");
  const [scheduleEnabled, setScheduleEnabled] = useState("true");
  const [targetSelectionMode, setTargetSelectionMode] = useState("direct");
  const [targetRepo, setTargetRepo] = useState(form.repo_path || "");
  const [maxFiles, setMaxFiles] = useState(form.max_files || "250");
  const [query, setQuery] = useState("");
  const [topics, setTopics] = useState("");
  const [languages, setLanguages] = useState("rust, typescript, python");
  const [minStars, setMinStars] = useState("25");
  const [cooldownDays, setCooldownDays] = useState("30");
  const [repoControl, setRepoControl] = useState({ repo: "", list_type: "allowlist" });
  const [generatedToken, setGeneratedToken] = useState("");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const [runningSchedule, setRunningSchedule] = useState("");

  const refresh = useCallback(async () => {
    try {
      const [presetPayload, schedulePayload, repoPayload, nextAuth] = await Promise.all([
        readJson(await fetcher(`${apiBase}/presets`)),
        readJson(await fetcher(`${apiBase}/schedules`)),
        readJson(await fetcher(`${apiBase}/repo-lists`)),
        readJson(await fetcher(`${apiBase}/auth/status`)),
      ]);
      const nextPresets = presetPayload.presets || [];
      const nextSchedules = schedulePayload.schedules || [];
      setPresets(nextPresets);
      setSchedules(nextSchedules);
      setRepoLists(repoPayload.repos || []);
      setAuthStatus(nextAuth);
      setSelectedPreset((current) => nextPresets.some((entry) => entry.name === current)
        ? current
        : nextPresets[0]?.name || "");
      setSelectedSchedule((current) => nextSchedules.some((entry) => entry.name === current)
        ? current
        : nextSchedules[0]?.name || "");
    } catch (error) {
      onError(error.message || "Could not load RefactorScout controls.");
    }
  }, [apiBase, fetcher, onError]);

  useEffect(() => { refresh(); }, [refresh]);

  const currentPayload = normalizedPayload({
    cooldownDays,
    languages,
    maxFiles,
    minStars,
    query,
    targetRepo,
    targetSelectionMode,
    topics,
  });
  const currentPreset = presets.find((entry) => entry.name === selectedPreset);
  const currentSchedule = schedules.find((entry) => entry.name === selectedSchedule);

  async function perform(action, success) {
    setBusy(true);
    setMessage("");
    try {
      await action();
      setMessage(success);
      await refresh();
      await onRefresh?.();
    } catch (error) {
      onError(error.message || "RefactorScout control update failed.");
    } finally {
      setBusy(false);
    }
  }

  function loadInputs(payload, mode) {
    const discovery = payload?.discovery || {};
    setTargetSelectionMode(mode || "direct");
    setTargetRepo(payload?.repo_path || "");
    setMaxFiles(String(payload?.max_files || 250));
    setQuery(discovery.query || "");
    setTopics((discovery.topics || []).join(", "));
    setLanguages((discovery.languages || []).join(", "));
    setMinStars(String(discovery.min_stars || 25));
    setCooldownDays(String(discovery.cooldown_days || 30));
    if ((mode || "direct") === "direct") {
      setForm((current) => ({
        ...current,
        repo_path: payload?.repo_path || "",
        max_files: String(payload?.max_files || 250),
      }));
    }
  }

  function loadPreset() {
    if (!currentPreset) return;
    loadInputs(currentPreset.params, currentPreset.target_selection_mode);
    setPresetName(currentPreset.name);
    setMessage(`Loaded preset: ${currentPreset.name}.`);
  }

  function loadSchedule() {
    if (!currentSchedule) return;
    loadInputs(currentSchedule.payload, currentSchedule.target_selection_mode);
    setScheduleName(currentSchedule.name);
    setCadence(String(currentSchedule.cadence_hours || 24));
    setScheduleEnabled(currentSchedule.enabled ? "true" : "false");
    setMessage(`Loaded schedule: ${currentSchedule.name}.`);
  }

  async function runScheduleNow() {
    if (!currentSchedule) return;
    const name = currentSchedule.name;
    setBusy(true);
    setRunningSchedule(name);
    setMessage(`Running schedule: ${name}. RefactorScout is inspecting its saved scope now.`);
    try {
      const scan = await readJson(await fetcher(
        `${apiBase}/schedules/${encodeURIComponent(name)}/run`,
        { method: "POST" },
      ));
      setMessage(`Schedule run completed: ${name}. Opening the saved scan…`);
      await onLoad(scan.id);
    } catch (error) {
      const detail = error.message || "RefactorScout could not run this schedule.";
      setMessage(`Schedule run failed: ${name}. ${detail}`);
      onError(detail);
      await refresh();
    } finally {
      setRunningSchedule("");
      setBusy(false);
    }
  }

  async function serviceToken(path, success) {
    setBusy(true);
    setGeneratedToken("");
    try {
      const payload = await readJson(await fetcher(`${apiBase}${path}`, { method: "POST" }));
      setGeneratedToken(payload.service_token || "");
      setMessage(success);
      await refresh();
    } catch (error) {
      onError(error.message || "Could not update the HiveCore service token.");
    } finally {
      setBusy(false);
    }
  }

  async function copyToken() {
    try {
      await navigator.clipboard.writeText(generatedToken);
      setMessage("Service token copied. Store it now; RefactorScout will not show it again.");
    } catch {
      onError("Could not copy the generated service token.");
    }
  }

  const directTargetValid = targetSelectionMode === "direct"
    ? Boolean(targetRepo.trim())
    : Boolean(query.trim() || topics.trim() || languages.trim());
  const payloadValid = directTargetValid
    && Number(maxFiles) >= 25
    && Number(maxFiles) <= 1500;

  return (
    <ProductControlsLayout
      description="Reuse direct repository targets or bounded discovery scopes, schedule read-only inspections, and enforce repository boundaries before RefactorScout reads source code."
      eyebrow="Scan operations"
      message={message}
    >
      <ProductControlsPair>
        <ProductControlSection>
          <ControlPanelTitle icon={Save} subtitle="Save the current target and inspection scope or load a known refactor review setup.">Scan presets</ControlPanelTitle>
          <div className="mt-5 grid gap-3 sm:grid-cols-2">
            <ControlField label="Preset name" onChange={setPresetName} placeholder="public-maintenance-review" value={presetName} />
            <ControlSelectField label="Saved presets" onChange={setSelectedPreset} options={[{ value: "", label: "No saved presets" }, ...presets.map((entry) => ({ value: entry.name, label: entry.name }))]} value={selectedPreset} />
          </div>
          <div className="mt-4 flex flex-wrap gap-2">
            <ControlButton disabled={busy || !presetName.trim() || !payloadValid} onClick={() => perform(async () => readJson(await fetcher(`${apiBase}/presets`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ name: presetName.trim(), params: currentPayload, target_selection_mode: targetSelectionMode }) })), `Preset saved: ${presetName.trim()}.`)} primary>Save preset</ControlButton>
            <ControlButton disabled={!currentPreset} onClick={loadPreset}>Load</ControlButton>
            <ControlButton disabled={busy || !currentPreset} onClick={() => perform(async () => readJson(await fetcher(`${apiBase}/presets/${encodeURIComponent(currentPreset.name)}`, { method: "DELETE" })), `Preset deleted: ${currentPreset?.name}.`)} tone="danger">Delete</ControlButton>
          </div>
          <div className={`mt-4 text-[11px] ${V3_TEXT.mute}`}>{countLabel(presets.length, "saved preset")}</div>
        </ProductControlSection>

        <ProductControlSection>
          <ControlPanelTitle icon={CalendarClock} subtitle="Scheduled scans use the same target mode, read-only parameters, and repository controls as operator-started scans.">Scan schedules</ControlPanelTitle>
          <div className="mt-5 grid gap-3 sm:grid-cols-2">
            <ControlField label="Schedule name" onChange={setScheduleName} placeholder="daily-structural-review" value={scheduleName} />
            <ControlSelectField label="Saved schedules" onChange={setSelectedSchedule} options={[{ value: "", label: "No saved schedules" }, ...schedules.map((entry) => ({ value: entry.name, label: entry.enabled ? `${entry.name} · enabled` : `${entry.name} · paused` }))]} value={selectedSchedule} />
            <ControlSelectField label="Cadence" onChange={setCadence} options={CADENCES} value={cadence} />
            <ControlSelectField label="State" onChange={setScheduleEnabled} options={[{ value: "true", label: "Enabled" }, { value: "false", label: "Paused" }]} value={scheduleEnabled} />
          </div>
          <div className="mt-4 flex flex-wrap gap-2">
            <ControlButton disabled={busy || !scheduleName.trim() || !payloadValid} onClick={() => perform(async () => readJson(await fetcher(`${apiBase}/schedules`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ name: scheduleName.trim(), payload: currentPayload, target_selection_mode: targetSelectionMode, cadence_hours: Number(cadence), enabled: scheduleEnabled === "true" }) })), `Schedule saved: ${scheduleName.trim()}.`)} primary>Save schedule</ControlButton>
            <ControlButton disabled={busy || !currentSchedule} onClick={loadSchedule}>Load</ControlButton>
            <ControlButton disabled={busy || !currentSchedule} onClick={runScheduleNow}>{runningSchedule ? <span className="inline-flex items-center gap-2"><LoaderCircle className="animate-spin" size={13} />Running scan…</span> : "Run now"}</ControlButton>
            <ControlButton disabled={busy || !currentSchedule} onClick={() => perform(async () => readJson(await fetcher(`${apiBase}/schedules/${encodeURIComponent(currentSchedule.name)}`, { method: "DELETE" })), `Schedule deleted: ${currentSchedule?.name}.`)} tone="danger">Delete</ControlButton>
          </div>
          {runningSchedule ? <div aria-live="polite" className={`surface-inset mt-4 flex items-start gap-3 rounded-xl p-3 text-[11px] leading-relaxed ${V3_TEXT.body}`} role="status"><LoaderCircle className="mt-0.5 shrink-0 animate-spin" size={14} /><span><strong className={V3_TEXT.strong}>{runningSchedule}</strong> is inspecting its saved scope. RefactorScout will open the completed result automatically.</span></div> : null}
          {currentSchedule ? <div className={`surface-inset mt-4 rounded-xl p-3 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Next run: {currentSchedule.next_run_at ? new Date(currentSchedule.next_run_at).toLocaleString() : "not scheduled"} · Last status: {currentSchedule.last_status || "never"}{currentSchedule.last_error ? ` · ${currentSchedule.last_error}` : ""}</div> : null}
        </ProductControlSection>
      </ProductControlsPair>

      <ProductControlSection>
        <ControlPanelTitle icon={FolderSearch} subtitle="Choose a deliberate target or a discovery scope. Autonomous discovery selects a different eligible public GitHub repository after each cooldown.">Target and discovery scope</ControlPanelTitle>
        <div className="mt-5 grid gap-3 sm:grid-cols-2">
          <ControlSelectField label="Target mode" onChange={setTargetSelectionMode} options={TARGET_MODES} value={targetSelectionMode} />
          <ControlField label="Maximum source files" max="1500" min="25" onChange={setMaxFiles} type="number" value={maxFiles} />
          {targetSelectionMode === "direct" ? (
            <div className="sm:col-span-2">
              <ControlField label="Target repo or allowed local path" onChange={setTargetRepo} placeholder="owner/repository or /allowed/local/path" value={targetRepo} />
            </div>
          ) : (
            <>
              <div className="sm:col-span-2"><ControlField label="GitHub discovery query" onChange={setQuery} placeholder="maintenance, developer tools, agents…" value={query} /></div>
              <ControlField label="Topics" onChange={setTopics} placeholder="developer-tools, maintenance" value={topics} />
              <ControlField label="Languages" onChange={setLanguages} placeholder="rust, typescript, python" value={languages} />
              <ControlField label="Minimum stars" min="1" onChange={setMinStars} type="number" value={minStars} />
              <ControlField label="Repository cooldown days" max="365" min="1" onChange={setCooldownDays} type="number" value={cooldownDays} />
            </>
          )}
        </div>
      </ProductControlSection>

      <ProductControlSection>
        <ControlPanelTitle icon={ListChecks} subtitle="Opt-outs and denylists always override direct and discovery scans. An allowlist narrows GitHub scans to explicitly approved repositories.">Repository controls</ControlPanelTitle>
        <div className="mt-5 grid gap-3 sm:grid-cols-[minmax(0,1fr)_220px_auto]">
          <ControlField label="Repository" onChange={(repo) => setRepoControl((current) => ({ ...current, repo }))} placeholder="owner/repository" value={repoControl.repo} />
          <ControlSelectField label="Control" onChange={(list_type) => setRepoControl((current) => ({ ...current, list_type }))} options={REPO_LISTS} value={repoControl.list_type} />
          <div className="flex items-end"><ControlButton disabled={busy || !/^[^/\s]+\/[^/\s]+$/.test(repoControl.repo.trim())} onClick={() => perform(async () => { await readJson(await fetcher(`${apiBase}/repo-lists`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ repo: repoControl.repo.trim(), list_type: repoControl.list_type }) })); setRepoControl((current) => ({ ...current, repo: "" })); }, "Repository control saved.")} primary>Save control</ControlButton></div>
        </div>
        <div className="mt-5 space-y-2">
          {repoLists.map((entry) => <div className="surface-inset flex flex-col gap-3 rounded-xl p-3 sm:flex-row sm:items-center sm:justify-between" key={entry.repo}><div><div className={`font-display text-[14px] ${V3_TEXT.strong}`}>{entry.repo}</div><div className={`mt-0.5 text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>{entry.list_type.replaceAll("_", "-")} · added {new Date(entry.added_at).toLocaleString()}</div></div><ControlButton disabled={busy} onClick={() => perform(async () => readJson(await fetcher(`${apiBase}/repo-lists/${encodeURIComponent(entry.repo)}`, { method: "DELETE" })), `Repository control removed: ${entry.repo}.`)} tone="danger">Remove</ControlButton></div>)}
          {!repoLists.length ? <div className={`py-8 text-center text-[12px] ${V3_TEXT.mute}`}>No repository-specific controls are saved.</div> : null}
        </div>
      </ProductControlSection>

      <ProductControlSection>
        <ControlPanelTitle icon={KeyRound} subtitle="This token is for authenticated HiveCore or peer-product dispatch. It is separate from the browser API key.">Suite service token</ControlPanelTitle>
        <div className="mt-5 flex flex-wrap gap-2">
          <ControlButton disabled={busy || authStatus.service_auth_enabled} onClick={() => serviceToken("/auth/generate-service-token", "HiveCore service token generated.")} primary>Generate token</ControlButton>
          <ControlButton disabled={busy || !authStatus.service_auth_enabled} onClick={() => serviceToken("/auth/rotate-service-token", "HiveCore service token rotated.")}>Rotate token</ControlButton>
          {generatedToken ? <ControlButton onClick={copyToken}>Copy new token</ControlButton> : null}
        </div>
        {generatedToken ? <div className="surface-inset mt-4 rounded-xl p-3"><div className={`text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>Shown once</div><code className={`mt-2 block break-all text-[11px] ${V3_TEXT.strong}`}>{generatedToken}</code></div> : null}
        <GuidanceNotice label="Authentication">Browser API auth is {health?.auth_enabled ? "enabled" : "disabled"}. Suite service auth is {authStatus.service_auth_enabled ? "configured" : "not configured"}. Generation and rotation remain protected by the shared peer-aware bootstrap rules.</GuidanceNotice>
      </ProductControlSection>

      <ProductControlsSafetyBoundary
        cards={[
          { title: "Read only", body: "Target and discovery scans inspect source and save evidence; they do not edit files, run tests, commit, push, or open pull requests." },
          { title: "Explicit exclusion", body: "Repository opt-outs and denylists are checked before GitHub targets are cloned. A configured allowlist narrows eligible targets." },
          { title: "Bounded inspection", body: "Local targets stay within configured roots. GitHub targets use disposable shallow clones, file caps, and autonomous cooldowns." },
        ]}
        subtitle="These controls change local RefactorScout configuration only. They do not write to repositories."
      />
    </ProductControlsLayout>
  );
}
