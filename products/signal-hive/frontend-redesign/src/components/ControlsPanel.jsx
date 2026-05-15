import { useState, useEffect } from "react";
import { API } from "../config.js";

const CADENCE_OPTIONS = [
  { v: "6", l: "Every 6 hours" },
  { v: "12", l: "Every 12 hours" },
  { v: "24", l: "Daily" },
  { v: "72", l: "Every 3 days" },
  { v: "168", l: "Weekly" },
];

function toList(value) {
  return value.split(",").map(p => p.trim()).filter(Boolean);
}

function toRequestParams(params) {
  return {
    search_query: params.search_query,
    topics: toList(params.topics),
    languages: toList(params.languages),
    min_stars: Number(params.min_stars) || 25,
    max_repos: Number(params.max_repos) || 8,
    issues_per_repo: Number(params.issues_per_repo) || 30,
    stale_days: Number(params.stale_days) || 45,
  };
}

function toFormParams(params) {
  return {
    search_query: params.search_query || "",
    topics: (params.topics || []).join(","),
    languages: (params.languages || []).join(","),
    min_stars: String(params.min_stars ?? 25),
    max_repos: String(params.max_repos ?? 8),
    issues_per_repo: String(params.issues_per_repo ?? 30),
    stale_days: String(params.stale_days ?? 45),
  };
}

function formatTS(value) {
  if (!value) return "never";
  const d = new Date(value);
  if (isNaN(d.getTime())) return value;
  return d.toLocaleString();
}

export default function ControlsPanel({ apiKey, params, setParams, fetch_ }) {
  const [presets, setPresets] = useState([]);
  const [selectedPresetName, setSelectedPresetName] = useState("");
  const [saveName, setSaveName] = useState("");
  const [presetBusy, setPresetBusy] = useState(false);
  const [presetError, setPresetError] = useState("");

  const [schedules, setSchedules] = useState([]);
  const [selectedScheduleName, setSelectedScheduleName] = useState("");
  const [scheduleName, setScheduleName] = useState("");
  const [scheduleCadence, setScheduleCadence] = useState("24");
  const [scheduleEnabled, setScheduleEnabled] = useState("true");
  const [scheduleBusy, setScheduleBusy] = useState(false);
  const [scheduleError, setScheduleError] = useState("");

  const loadPresets = (preferred = "") => {
    fetch_(`${API}/presets`).then(r => r.json()).then(data => {
      const next = data.presets || [];
      const sel = preferred || selectedPresetName;
      setPresets(next);
      setSelectedPresetName(next.some(p => p.name === sel) ? sel : next[0]?.name || "");
    }).catch(() => setPresets([]));
  };

  const loadSchedules = (preferred = "") => {
    fetch_(`${API}/schedules`).then(r => r.json()).then(data => {
      const next = data.schedules || [];
      const sel = preferred || selectedScheduleName;
      setSchedules(next);
      setSelectedScheduleName(next.some(s => s.name === sel) ? sel : next[0]?.name || "");
    }).catch(() => setSchedules([]));
  };

  useEffect(() => { loadPresets(); loadSchedules(); }, [apiKey]);

  const selectedPreset = presets.find(p => p.name === selectedPresetName);
  const selectedSchedule = schedules.find(s => s.name === selectedScheduleName);

  const savePreset = async () => {
    if (!saveName.trim()) return;
    setPresetBusy(true); setPresetError("");
    try {
      const res = await fetch_(`${API}/presets`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: saveName.trim(), params: toRequestParams(params) }),
      });
      if (!res.ok) throw new Error("Could not save preset.");
      setSaveName(""); await loadPresets(saveName.trim());
    } catch (err) { setPresetError(err.message); }
    finally { setPresetBusy(false); }
  };

  const deletePreset = async () => {
    if (!selectedPreset) return;
    setPresetBusy(true); setPresetError("");
    try {
      const res = await fetch_(`${API}/presets/${encodeURIComponent(selectedPreset.name)}`, { method: "DELETE" });
      if (!res.ok) throw new Error("Could not delete preset.");
      if (saveName === selectedPreset.name) setSaveName("");
      await loadPresets();
    } catch (err) { setPresetError(err.message); }
    finally { setPresetBusy(false); }
  };

  const saveSchedule = async () => {
    if (!scheduleName.trim()) return;
    setScheduleBusy(true); setScheduleError("");
    try {
      const res = await fetch_(`${API}/schedules`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: scheduleName.trim(), params: toRequestParams(params),
          cadence_hours: Number(scheduleCadence) || 24, enabled: scheduleEnabled === "true",
        }),
      });
      if (!res.ok) throw new Error("Could not save schedule.");
      await loadSchedules(scheduleName.trim());
    } catch (err) { setScheduleError(err.message); }
    finally { setScheduleBusy(false); }
  };

  const deleteSchedule = async () => {
    if (!selectedSchedule) return;
    setScheduleBusy(true); setScheduleError("");
    try {
      const res = await fetch_(`${API}/schedules/${encodeURIComponent(selectedSchedule.name)}`, { method: "DELETE" });
      if (!res.ok) throw new Error("Could not delete schedule.");
      if (scheduleName === selectedSchedule.name) setScheduleName("");
      await loadSchedules();
    } catch (err) { setScheduleError(err.message); }
    finally { setScheduleBusy(false); }
  };

  const runScheduleNow = async () => {
    if (!selectedSchedule) return;
    setScheduleBusy(true); setScheduleError("");
    try {
      const res = await fetch_(`${API}/schedules/${encodeURIComponent(selectedSchedule.name)}/run`, { method: "POST" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Could not run schedule.");
      await loadSchedules(selectedSchedule.name);
    } catch (err) { setScheduleError(err.message); }
    finally { setScheduleBusy(false); }
  };

  return (
    <div style={{ display: "grid", gap: 16 }}>
      {/* Presets */}
      <div className="card">
        <div className="card-header">
          <h3>Scan Presets</h3>
          <span className="tag tag-accent">{presets.length} saved</span>
        </div>
        <div className="card-body" style={{ display: "grid", gap: 14 }}>
          <div className="form-row-3">
            <div className="field">
              <label>Saved Preset</label>
              <select value={selectedPresetName} onChange={e => setSelectedPresetName(e.target.value)}>
                {presets.length > 0 ? presets.map(p => <option key={p.name} value={p.name}>{p.name}</option>) : <option value="">No saved presets</option>}
              </select>
            </div>
            <button className="btn btn-sm" onClick={() => { if (selectedPreset) setParams(toFormParams(selectedPreset.params)); setSaveName(selectedPreset?.name || ""); }} disabled={!selectedPreset || presetBusy}>Load</button>
            <button className="btn btn-sm" onClick={deletePreset} disabled={!selectedPreset || presetBusy}>Delete</button>
          </div>
          <div className="form-row">
            <div className="field">
              <label>Save Current Config</label>
              <input value={saveName} onChange={e => setSaveName(e.target.value)} placeholder="nightly rust maintenance" />
            </div>
            <button className="btn btn-primary btn-sm" onClick={savePreset} disabled={presetBusy || !saveName.trim()}>{presetBusy ? "Saving…" : "Save Preset"}</button>
          </div>
          {selectedPreset && (
            <div style={{ fontSize: 11, color: "var(--text-muted)", display: "grid", gap: 4 }}>
              <div>Last updated {formatTS(selectedPreset.updated_at)}</div>
              <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                {selectedPreset.params.search_query && <span className="tag tag-surface">{selectedPreset.params.search_query}</span>}
                {(selectedPreset.params.languages || []).map(l => <span key={l} className="tag tag-surface">{l}</span>)}
                {(selectedPreset.params.topics || []).map(t => <span key={t} className="tag tag-surface">{t}</span>)}
                <span className="tag tag-surface">{selectedPreset.params.max_repos} repos</span>
                <span className="tag tag-surface">{selectedPreset.params.stale_days}d stale</span>
              </div>
            </div>
          )}
          {presetError && <div style={{ fontSize: 11, color: "var(--red)" }}>{presetError}</div>}
        </div>
      </div>

      {/* Schedules */}
      <div className="card">
        <div className="card-header">
          <h3>Scheduled Scans</h3>
          <span className="tag tag-green">{schedules.filter(s => s.enabled).length} active</span>
        </div>
        <div className="card-body" style={{ display: "grid", gap: 14 }}>
          <div className="form-row-3">
            <div className="field">
              <label>Saved Schedule</label>
              <select value={selectedScheduleName} onChange={e => setSelectedScheduleName(e.target.value)}>
                {schedules.length > 0 ? schedules.map(s => <option key={s.name} value={s.name}>{s.name}</option>) : <option value="">No saved schedules</option>}
              </select>
            </div>
            <button className="btn btn-sm" onClick={() => { if (selectedSchedule) { setParams(toFormParams(selectedSchedule.params)); setScheduleName(selectedSchedule.name); setScheduleCadence(String(selectedSchedule.cadence_hours)); setScheduleEnabled(selectedSchedule.enabled ? "true" : "false"); } }} disabled={!selectedSchedule}>Load</button>
            <button className="btn btn-sm" onClick={runScheduleNow} disabled={!selectedSchedule || scheduleBusy}>Run Now</button>
          </div>
          <div className="form-row-4">
            <div className="field">
              <label>Schedule Name</label>
              <input value={scheduleName} onChange={e => setScheduleName(e.target.value)} placeholder="daily rust queue" />
            </div>
            <div className="field">
              <label>Cadence</label>
              <select value={scheduleCadence} onChange={e => setScheduleCadence(e.target.value)}>
                {CADENCE_OPTIONS.map(o => <option key={o.v} value={o.v}>{o.l}</option>)}
              </select>
            </div>
            <div className="field">
              <label>Status</label>
              <select value={scheduleEnabled} onChange={e => setScheduleEnabled(e.target.value)}>
                <option value="true">Enabled</option>
                <option value="false">Paused</option>
              </select>
            </div>
            <button className="btn btn-primary btn-sm" onClick={saveSchedule} disabled={scheduleBusy || !scheduleName.trim()}>{scheduleBusy ? "Working…" : "Save Schedule"}</button>
          </div>
          {selectedSchedule && (
            <div style={{ fontSize: 11, color: "var(--text-muted)", display: "flex", gap: 4, flexWrap: "wrap" }}>
              <span className={`tag ${selectedSchedule.enabled ? "tag-green" : "tag-amber"}`}>{selectedSchedule.enabled ? "enabled" : "paused"}</span>
              <span className="tag tag-surface">every {selectedSchedule.cadence_hours}h</span>
              <span className="tag tag-surface">next {formatTS(selectedSchedule.next_run_at)}</span>
              <span className="tag tag-surface">last {selectedSchedule.last_run_at ? formatTS(selectedSchedule.last_run_at) : "never"}</span>
              {selectedSchedule.last_status && <span className="tag tag-surface">{selectedSchedule.last_status}</span>}
              {deleteSchedule.name && <button className="btn btn-sm" onClick={deleteSchedule} disabled={!selectedSchedule || scheduleBusy}>Delete</button>}
            </div>
          )}
          {selectedSchedule?.last_error && <div style={{ fontSize: 11, color: "var(--red)" }}>{selectedSchedule.last_error}</div>}
          {scheduleError && <div style={{ fontSize: 11, color: "var(--red)" }}>{scheduleError}</div>}
        </div>
      </div>
    </div>
  );
}
