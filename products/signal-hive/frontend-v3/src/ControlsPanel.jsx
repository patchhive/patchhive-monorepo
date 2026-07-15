import { useCallback, useEffect, useState } from "react";
import { CalendarClock, KeyRound, ListChecks, Save, ShieldCheck } from "lucide-react";
import { GuidanceNotice, V3_TEXT, countLabel, readJson } from "@patchhivehq/ui-v3";

const CADENCES = [{ value: "6", label: "Every 6 hours" }, { value: "12", label: "Every 12 hours" }, { value: "24", label: "Daily" }, { value: "72", label: "Every 3 days" }, { value: "168", label: "Weekly" }];
const LISTS = [{ value: "allowlist", label: "Allowlist" }, { value: "denylist", label: "Denylist" }, { value: "opt_out", label: "Opt-out" }];

function Field({ label, onChange, placeholder, type = "text", value }) {
  return <label className="block"><span className={`text-[10px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>{label}</span><div className="surface-inset mt-2 flex h-11 items-center rounded-xl px-3"><input className={`w-full bg-transparent text-[12px] outline-none ${V3_TEXT.strong}`} min={type === "number" ? 1 : undefined} onChange={(event) => onChange(event.target.value)} placeholder={placeholder} type={type} value={value} /></div></label>;
}

function SelectField({ label, onChange, options, value }) {
  return <label className="block"><span className={`text-[10px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>{label}</span><div className="surface-inset mt-2 flex h-11 items-center rounded-xl px-3"><select className={`w-full bg-transparent text-[12px] outline-none ${V3_TEXT.strong}`} onChange={(event) => onChange(event.target.value)} value={value}>{options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}</select></div></label>;
}

function Button({ children, disabled, onClick, primary = false, tone = "normal" }) {
  const toneClass = tone === "danger" ? "text-red-800 dark:text-red-300" : V3_TEXT.body;
  return <button className={primary ? "h-10 rounded-full px-4 text-[12px] font-semibold text-white disabled:opacity-40" : `surface-inset h-10 rounded-full px-4 text-[12px] disabled:opacity-40 ${toneClass}`} disabled={disabled} onClick={onClick} style={primary ? { backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))" } : undefined} type="button">{children}</button>;
}

function PanelTitle({ children, icon: Icon, subtitle }) {
  return <div><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Icon size={12} />{children}</div>{subtitle ? <p className={`mt-2 text-[12px] leading-relaxed ${V3_TEXT.mute}`}>{subtitle}</p> : null}</div>;
}

export default function ControlsPanel({ apiBase, fetcher, form, health, onError, onLoad, onRefresh, serialize, setForm, toFormParams }) {
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
  const [repoControl, setRepoControl] = useState({ repo: "", list_type: "allowlist" });
  const [generatedToken, setGeneratedToken] = useState("");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);

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
      setSelectedPreset((current) => nextPresets.some((entry) => entry.name === current) ? current : nextPresets[0]?.name || "");
      setSelectedSchedule((current) => nextSchedules.some((entry) => entry.name === current) ? current : nextSchedules[0]?.name || "");
    } catch (error) {
      onError(error.message || "Could not load SignalHive controls.");
    }
  }, [apiBase, fetcher, onError]);

  useEffect(() => { refresh(); }, [refresh]);

  async function perform(action, success) {
    setBusy(true);
    setMessage("");
    try {
      await action();
      setMessage(success);
      await refresh();
      onRefresh?.();
    } catch (error) {
      onError(error.message || "SignalHive control update failed.");
    } finally {
      setBusy(false);
    }
  }

  const currentPreset = presets.find((entry) => entry.name === selectedPreset);
  const currentSchedule = schedules.find((entry) => entry.name === selectedSchedule);

  function loadPreset() {
    if (!currentPreset) return;
    setForm((current) => ({ ...current, ...toFormParams(currentPreset.params) }));
    setPresetName(currentPreset.name);
    setMessage(`Loaded preset: ${currentPreset.name}. Open Sources to review or run it.`);
  }

  function loadSchedule() {
    if (!currentSchedule) return;
    setForm((current) => ({ ...current, ...toFormParams(currentSchedule.params) }));
    setScheduleName(currentSchedule.name);
    setCadence(String(currentSchedule.cadence_hours || 24));
    setScheduleEnabled(currentSchedule.enabled ? "true" : "false");
    setMessage(`Loaded schedule: ${currentSchedule.name}.`);
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
      setMessage("Service token copied. Store it now; SignalHive will not show it again after this page changes.");
    } catch {
      onError("Could not copy the generated service token.");
    }
  }

  return <div className="space-y-6">
    <section className="surface p-6 sm:p-8"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Discovery operations</div><h1 className={`mt-2 font-display text-[42px] font-semibold ${V3_TEXT.strong}`}>Presets, schedules, and scope.</h1><p className={`mt-3 max-w-3xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>Reuse broad discovery scopes, schedule read-only scans, and enforce repository boundaries before SignalHive searches GitHub.</p>{message ? <div className={`surface-inset mt-5 rounded-xl p-3 text-[12px] ${V3_TEXT.body}`}>{message}</div> : null}</section>

    <div className="grid grid-cols-1 gap-6 xl:grid-cols-2">
      <section className="surface p-6"><PanelTitle icon={Save} subtitle="Save the current Sources form or load a known reconnaissance scope.">Scan presets</PanelTitle><div className="mt-5 grid gap-3 sm:grid-cols-2"><Field label="Preset name" onChange={setPresetName} placeholder="public-rust-maintenance" value={presetName} /><SelectField label="Saved presets" onChange={setSelectedPreset} options={[{ value: "", label: "No saved presets" }, ...presets.map((entry) => ({ value: entry.name, label: entry.name }))]} value={selectedPreset} /></div><div className="mt-4 flex flex-wrap gap-2"><Button disabled={busy || !presetName.trim()} onClick={() => perform(async () => readJson(await fetcher(`${apiBase}/presets`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ name: presetName.trim(), params: serialize(form) }) })), `Preset saved: ${presetName.trim()}.`)} primary>Save preset</Button><Button disabled={!currentPreset} onClick={loadPreset}>Load</Button><Button disabled={busy || !currentPreset} onClick={() => perform(async () => readJson(await fetcher(`${apiBase}/presets/${encodeURIComponent(currentPreset.name)}`, { method: "DELETE" })), `Preset deleted: ${currentPreset?.name}.`)} tone="danger">Delete</Button></div><div className={`mt-4 text-[11px] ${V3_TEXT.mute}`}>{countLabel(presets.length, "saved preset")}</div></section>

      <section className="surface p-6"><PanelTitle icon={CalendarClock} subtitle="Scheduled scans use the same read-only parameters and repository controls as manual scans.">Scan schedules</PanelTitle><div className="mt-5 grid gap-3 sm:grid-cols-2"><Field label="Schedule name" onChange={setScheduleName} placeholder="daily-maintenance-radar" value={scheduleName} /><SelectField label="Saved schedules" onChange={setSelectedSchedule} options={[{ value: "", label: "No saved schedules" }, ...schedules.map((entry) => ({ value: entry.name, label: entry.enabled ? `${entry.name} · enabled` : `${entry.name} · paused` }))]} value={selectedSchedule} /><SelectField label="Cadence" onChange={setCadence} options={CADENCES} value={cadence} /><SelectField label="State" onChange={setScheduleEnabled} options={[{ value: "true", label: "Enabled" }, { value: "false", label: "Paused" }]} value={scheduleEnabled} /></div><div className="mt-4 flex flex-wrap gap-2"><Button disabled={busy || !scheduleName.trim()} onClick={() => perform(async () => readJson(await fetcher(`${apiBase}/schedules`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ name: scheduleName.trim(), params: serialize(form), cadence_hours: Number(cadence), enabled: scheduleEnabled === "true" }) })), `Schedule saved: ${scheduleName.trim()}.`)} primary>Save schedule</Button><Button disabled={!currentSchedule} onClick={loadSchedule}>Load</Button><Button disabled={busy || !currentSchedule} onClick={() => perform(async () => { const scan = await readJson(await fetcher(`${apiBase}/schedules/${encodeURIComponent(currentSchedule.name)}/run`, { method: "POST" })); await onLoad(scan.id); }, `Schedule run completed: ${currentSchedule?.name}.`)}>Run now</Button><Button disabled={busy || !currentSchedule} onClick={() => perform(async () => readJson(await fetcher(`${apiBase}/schedules/${encodeURIComponent(currentSchedule.name)}`, { method: "DELETE" })), `Schedule deleted: ${currentSchedule?.name}.`)} tone="danger">Delete</Button></div>{currentSchedule ? <div className={`surface-inset mt-4 rounded-xl p-3 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Next run: {currentSchedule.next_run_at ? new Date(currentSchedule.next_run_at).toLocaleString() : "not scheduled"} · Last status: {currentSchedule.last_status || "never"}{currentSchedule.last_error ? ` · ${currentSchedule.last_error}` : ""}</div> : null}</section>
    </div>

    <section className="surface p-6"><PanelTitle icon={ListChecks} subtitle="Opt-outs and denylists always override discovery. An allowlist narrows scans to explicitly approved repositories.">Repository controls</PanelTitle><div className="mt-5 grid gap-3 sm:grid-cols-[minmax(0,1fr)_220px_auto]"><Field label="Repository" onChange={(repo) => setRepoControl((current) => ({ ...current, repo }))} placeholder="owner/repository" value={repoControl.repo} /><SelectField label="Control" onChange={(list_type) => setRepoControl((current) => ({ ...current, list_type }))} options={LISTS} value={repoControl.list_type} /><div className="flex items-end"><Button disabled={busy || !/^[^/\s]+\/[^/\s]+$/.test(repoControl.repo.trim())} onClick={() => perform(async () => { await readJson(await fetcher(`${apiBase}/repo-lists`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ repo: repoControl.repo.trim(), list_type: repoControl.list_type }) })); setRepoControl((current) => ({ ...current, repo: "" })); }, "Repository control saved.")} primary>Save control</Button></div></div><div className="mt-5 space-y-2">{repoLists.map((entry) => <div className="surface-inset flex flex-col gap-3 rounded-xl p-3 sm:flex-row sm:items-center sm:justify-between" key={entry.repo}><div><div className={`font-display text-[14px] ${V3_TEXT.strong}`}>{entry.repo}</div><div className={`mt-0.5 text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>{entry.list_type.replaceAll("_", "-")} · added {new Date(entry.added_at).toLocaleString()}</div></div><Button disabled={busy} onClick={() => perform(async () => readJson(await fetcher(`${apiBase}/repo-lists/${encodeURIComponent(entry.repo)}`, { method: "DELETE" })), `Repository control removed: ${entry.repo}.`)} tone="danger">Remove</Button></div>)}{!repoLists.length ? <div className={`py-8 text-center text-[12px] ${V3_TEXT.mute}`}>No repository-specific controls are saved.</div> : null}</div></section>

    <section className="surface p-6"><PanelTitle icon={KeyRound} subtitle="This token is for authenticated HiveCore or peer-product dispatch. It is separate from the browser API key.">Suite service token</PanelTitle><div className="mt-5 flex flex-wrap gap-2"><Button disabled={busy || authStatus.service_auth_enabled} onClick={() => serviceToken("/auth/generate-service-token", "HiveCore service token generated.")} primary>Generate token</Button><Button disabled={busy || !authStatus.service_auth_enabled} onClick={() => serviceToken("/auth/rotate-service-token", "HiveCore service token rotated.")}>Rotate token</Button>{generatedToken ? <Button onClick={copyToken}>Copy new token</Button> : null}</div>{generatedToken ? <div className="surface-inset mt-4 rounded-xl p-3"><div className={`text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>Shown once</div><code className={`mt-2 block break-all text-[11px] ${V3_TEXT.strong}`}>{generatedToken}</code></div> : null}<GuidanceNotice label="Authentication">Browser API auth is {health.auth_enabled ? "enabled" : "disabled"}. Suite service auth is {authStatus.service_auth_enabled ? "configured" : "not configured"}. Generation and rotation remain protected by the shared peer-aware bootstrap rules.</GuidanceNotice></section>

    <section className="surface p-6"><PanelTitle icon={ShieldCheck} subtitle="These controls change local SignalHive configuration only. They do not write to GitHub.">Safety boundary</PanelTitle><div className="mt-4 grid gap-3 sm:grid-cols-3"><div className={`surface-inset rounded-xl p-4 text-[11px] leading-relaxed ${V3_TEXT.mute}`}><span className={`block font-display text-[15px] ${V3_TEXT.strong}`}>Discovery only</span>Schedules gather evidence; they do not create issues, comments, commits, or pull requests.</div><div className={`surface-inset rounded-xl p-4 text-[11px] leading-relaxed ${V3_TEXT.mute}`}><span className={`block font-display text-[15px] ${V3_TEXT.strong}`}>Explicit exclusion</span>Repository opt-outs and denylists are checked before a scan retains discovered targets.</div><div className={`surface-inset rounded-xl p-4 text-[11px] leading-relaxed ${V3_TEXT.mute}`}><span className={`block font-display text-[15px] ${V3_TEXT.strong}`}>Bounded output</span>Each scope carries a repository cap, issue cap, stale window, and minimum-star threshold.</div></div></section>
  </div>;
}
