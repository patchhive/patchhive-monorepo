import { useEffect, useMemo, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { Btn, EmptyState, Input, S, Sel, Tag, timeAgo } from "@patchhivehq/ui";
import { API } from "../config.js";
import ReportDashboard from "../components/ReportDashboard.jsx";
import ScanTimelineChart from "../components/ScanTimelineChart.jsx";
import SignalCard from "../components/SignalCard.jsx";
import { buildDashboardSummary, downloadTextFile, exportDashboardHtml } from "../report.js";
import { SORT_OPTIONS, sortRepos } from "../sort.js";

function toList(value) {
  return value
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean);
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

function formatTimestamp(value) {
  if (!value) {
    return "never";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleString();
}

const CADENCE_OPTIONS = [
  { v: "6", l: "Every 6 hours" },
  { v: "12", l: "Every 12 hours" },
  { v: "24", l: "Daily" },
  { v: "72", l: "Every 3 days" },
  { v: "168", l: "Weekly" },
];

function TrendTag({ label, value }) {
  const color = value > 0 ? "var(--accent)" : value < 0 ? "var(--green)" : "var(--gold)";
  return <Tag color={color}>{label} {value > 0 ? "+" : ""}{value}</Tag>;
}

export default function ScanPanel({ apiKey, params, setParams, running, onRun, scan, setScan }) {
  const [sortBy, setSortBy] = useState("priority");
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
  const [timeline, setTimeline] = useState(null);
  const set = (key, value) => setParams((prev) => ({ ...prev, [key]: value }));
  const sortedRepos = useMemo(() => sortRepos(scan?.repos || [], sortBy), [scan, sortBy]);
  const fetch_ = createApiFetcher(apiKey);
  const selectedPreset = presets.find((preset) => preset.name === selectedPresetName) || null;
  const selectedSchedule = schedules.find((schedule) => schedule.name === selectedScheduleName) || null;

  const loadPresets = (preferredName = "") =>
    fetch_(`${API}/presets`)
      .then((res) => res.json())
      .then((data) => {
        const nextPresets = data.presets || [];
        const nextSelected = preferredName || selectedPresetName;
        setPresets(nextPresets);
        if (nextPresets.some((preset) => preset.name === nextSelected)) {
          setSelectedPresetName(nextSelected);
        } else {
          setSelectedPresetName(nextPresets[0]?.name || "");
        }
      })
      .catch(() => setPresets([]));

  const loadSchedules = (preferredName = "") =>
    fetch_(`${API}/schedules`)
      .then((res) => res.json())
      .then((data) => {
        const nextSchedules = data.schedules || [];
        const nextSelected = preferredName || selectedScheduleName;
        setSchedules(nextSchedules);
        if (nextSchedules.some((schedule) => schedule.name === nextSelected)) {
          setSelectedScheduleName(nextSelected);
        } else {
          setSelectedScheduleName(nextSchedules[0]?.name || "");
        }
      })
      .catch(() => setSchedules([]));

  useEffect(() => {
    loadPresets();
    loadSchedules();
  }, [apiKey]);

  useEffect(() => {
    if (!scan?.id) {
      setTimeline(null);
      return;
    }
    fetch_(`${API}/history/${scan.id}/timeline`)
      .then((res) => res.json())
      .then(setTimeline)
      .catch(() => setTimeline(null));
  }, [apiKey, scan?.id]);

  const savePreset = async () => {
    if (!saveName.trim()) {
      return;
    }
    setPresetBusy(true);
    setPresetError("");
    try {
      const res = await fetch_(`${API}/presets`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: saveName.trim(),
          params: toRequestParams(params),
        }),
      });
      if (!res.ok) {
        throw new Error("SignalHive could not save this preset.");
      }
      const nextName = saveName.trim();
      setSaveName("");
      await loadPresets(nextName);
    } catch (err) {
      setPresetError(err.message || "SignalHive could not save this preset.");
    } finally {
      setPresetBusy(false);
    }
  };

  const loadPreset = () => {
    if (!selectedPreset) {
      return;
    }
    setParams(toFormParams(selectedPreset.params));
    setSaveName(selectedPreset.name);
  };

  const deletePreset = async () => {
    if (!selectedPreset) {
      return;
    }
    setPresetBusy(true);
    setPresetError("");
    try {
      const res = await fetch_(`${API}/presets/${encodeURIComponent(selectedPreset.name)}`, {
        method: "DELETE",
      });
      if (!res.ok) {
        throw new Error("SignalHive could not delete this preset.");
      }
      if (saveName === selectedPreset.name) {
        setSaveName("");
      }
      await loadPresets();
    } catch (err) {
      setPresetError(err.message || "SignalHive could not delete this preset.");
    } finally {
      setPresetBusy(false);
    }
  };

  const loadSchedule = () => {
    if (!selectedSchedule) {
      return;
    }
    setParams(toFormParams(selectedSchedule.params));
    setScheduleName(selectedSchedule.name);
    setScheduleCadence(String(selectedSchedule.cadence_hours));
    setScheduleEnabled(selectedSchedule.enabled ? "true" : "false");
  };

  const saveSchedule = async () => {
    if (!scheduleName.trim()) {
      return;
    }
    setScheduleBusy(true);
    setScheduleError("");
    try {
      const res = await fetch_(`${API}/schedules`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: scheduleName.trim(),
          params: toRequestParams(params),
          cadence_hours: Number(scheduleCadence) || 24,
          enabled: scheduleEnabled === "true",
        }),
      });
      if (!res.ok) {
        throw new Error("SignalHive could not save this schedule.");
      }
      await loadSchedules(scheduleName.trim());
    } catch (err) {
      setScheduleError(err.message || "SignalHive could not save this schedule.");
    } finally {
      setScheduleBusy(false);
    }
  };

  const deleteSchedule = async () => {
    if (!selectedSchedule) {
      return;
    }
    setScheduleBusy(true);
    setScheduleError("");
    try {
      const res = await fetch_(`${API}/schedules/${encodeURIComponent(selectedSchedule.name)}`, {
        method: "DELETE",
      });
      if (!res.ok) {
        throw new Error("SignalHive could not delete this schedule.");
      }
      if (scheduleName === selectedSchedule.name) {
        setScheduleName("");
      }
      await loadSchedules();
    } catch (err) {
      setScheduleError(err.message || "SignalHive could not delete this schedule.");
    } finally {
      setScheduleBusy(false);
    }
  };

  const runScheduleNow = async () => {
    if (!selectedSchedule) {
      return;
    }
    setScheduleBusy(true);
    setScheduleError("");
    try {
      const res = await fetch_(`${API}/schedules/${encodeURIComponent(selectedSchedule.name)}/run`, {
        method: "POST",
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "SignalHive could not run this schedule.");
      }
      setScan(data);
      await loadSchedules(selectedSchedule.name);
    } catch (err) {
      setScheduleError(err.message || "SignalHive could not run this schedule.");
    } finally {
      setScheduleBusy(false);
    }
  };

  const downloadReport = async (scanId) => {
    if (!scanId) {
      return;
    }
    try {
      const res = await fetch_(`${API}/history/${scanId}/report`);
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "SignalHive could not export this report.");
      }
      downloadTextFile(
        data.filename || `signalhive-report-${scanId}.md`,
        data.markdown,
        "text/markdown;charset=utf-8",
      );
    } catch (err) {
      setScheduleError(err.message || "SignalHive could not export this report.");
    }
  };

  const copySummary = async () => {
    if (!scan) {
      return;
    }
    try {
      await navigator.clipboard.writeText(buildDashboardSummary(scan));
    } catch (err) {
      setScheduleError("SignalHive could not copy the summary to your clipboard.");
    }
  };

  return (
    <div style={{ display: "grid", gap: 18 }}>
      <div style={{ ...S.panel, display: "grid", gap: 16 }}>
        <div style={{ display: "grid", gap: 6 }}>
          <div style={{ fontSize: 24, fontWeight: 700, color: "var(--accent)" }}>
            See the maintenance work your team is missing.
          </div>
          <div style={{ color: "var(--text-dim)", fontSize: 13, lineHeight: 1.6, maxWidth: 900 }}>
            SignalHive analyzes repository and issue history to surface stale risks, duplicate problems,
            recurring bug clusters, TODO/FIXME hotspots, and backlog drag before they slow delivery.
            This pass adds trends, scheduling, and exportable reports so the queue can keep paying off over time.
          </div>
        </div>

        <div style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 14, display: "grid", gap: 12 }}>
          <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
            <div style={{ display: "grid", gap: 4 }}>
              <div style={{ fontSize: 14, fontWeight: 700 }}>Scan Presets</div>
              <div style={{ color: "var(--text-dim)", fontSize: 11, lineHeight: 1.5 }}>
                Save repeatable scan shapes so you can jump between maintenance views quickly.
              </div>
            </div>
            <Tag color="var(--accent)">{presets.length} saved</Tag>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "minmax(220px, 1fr) auto auto auto", gap: 10 }}>
            <div style={S.field}>
              <div style={S.label}>Saved Preset</div>
              <Sel
                value={selectedPresetName}
                onChange={setSelectedPresetName}
                opts={
                  presets.length > 0
                    ? presets.map((preset) => ({ v: preset.name, l: preset.name }))
                    : [{ v: "", l: "No saved presets" }]
                }
              />
            </div>
            <Btn onClick={loadPreset} disabled={!selectedPreset || presetBusy}>
              Load
            </Btn>
            <Btn onClick={deletePreset} disabled={!selectedPreset || presetBusy} color="var(--accent)">
              Delete
            </Btn>
            <Btn onClick={() => loadPresets()} disabled={presetBusy} color="var(--text-dim)">
              Refresh
            </Btn>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "minmax(220px, 1fr) auto", gap: 10 }}>
            <div style={S.field}>
              <div style={S.label}>Save Current Config</div>
              <Input value={saveName} onChange={setSaveName} placeholder="nightly rust maintenance" />
            </div>
            <Btn onClick={savePreset} disabled={presetBusy || !saveName.trim()}>
              {presetBusy ? "Saving…" : "Save Preset"}
            </Btn>
          </div>

          {selectedPreset && (
            <div style={{ display: "grid", gap: 6, color: "var(--text-dim)", fontSize: 11 }}>
              <div>
                Last updated {timeAgo(selectedPreset.updated_at)} • created {timeAgo(selectedPreset.created_at)}
              </div>
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                {selectedPreset.params.search_query && <Tag>{selectedPreset.params.search_query}</Tag>}
                {(selectedPreset.params.languages || []).map((language) => (
                  <Tag key={`preset-language-${language}`}>{language}</Tag>
                ))}
                {(selectedPreset.params.topics || []).map((topic) => (
                  <Tag key={`preset-topic-${topic}`}>{topic}</Tag>
                ))}
                <Tag>min {selectedPreset.params.min_stars} stars</Tag>
                <Tag>{selectedPreset.params.max_repos} repos</Tag>
                <Tag>{selectedPreset.params.issues_per_repo} issues / repo</Tag>
                <Tag>{selectedPreset.params.stale_days}d stale threshold</Tag>
              </div>
            </div>
          )}

          {presetError && (
            <div style={{ color: "var(--accent)", fontSize: 11 }}>
              {presetError}
            </div>
          )}
        </div>

        <div style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 14, display: "grid", gap: 12 }}>
          <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
            <div style={{ display: "grid", gap: 4 }}>
              <div style={{ fontSize: 14, fontWeight: 700 }}>Scheduled Scans</div>
              <div style={{ color: "var(--text-dim)", fontSize: 11, lineHeight: 1.5 }}>
                Keep a maintenance view warm. SignalHive will rerun these configs in the background and save the results.
              </div>
            </div>
            <Tag color="var(--gold)">{schedules.filter((schedule) => schedule.enabled).length} active</Tag>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "minmax(220px, 1fr) auto auto auto", gap: 10 }}>
            <div style={S.field}>
              <div style={S.label}>Saved Schedule</div>
              <Sel
                value={selectedScheduleName}
                onChange={setSelectedScheduleName}
                opts={
                  schedules.length > 0
                    ? schedules.map((schedule) => ({ v: schedule.name, l: schedule.name }))
                    : [{ v: "", l: "No saved schedules" }]
                }
              />
            </div>
            <Btn onClick={loadSchedule} disabled={!selectedSchedule || scheduleBusy}>
              Load
            </Btn>
            <Btn onClick={runScheduleNow} disabled={!selectedSchedule || scheduleBusy}>
              Run Now
            </Btn>
            <Btn onClick={() => loadSchedules()} disabled={scheduleBusy} color="var(--text-dim)">
              Refresh
            </Btn>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "minmax(220px, 1.2fr) minmax(140px, 0.8fr) minmax(140px, 0.8fr) auto auto", gap: 10, alignItems: "end" }}>
            <div style={S.field}>
              <div style={S.label}>Schedule Name</div>
              <Input value={scheduleName} onChange={setScheduleName} placeholder="daily rust queue" />
            </div>
            <div style={S.field}>
              <div style={S.label}>Cadence</div>
              <Sel value={scheduleCadence} onChange={setScheduleCadence} opts={CADENCE_OPTIONS} />
            </div>
            <div style={S.field}>
              <div style={S.label}>Status</div>
              <Sel
                value={scheduleEnabled}
                onChange={setScheduleEnabled}
                opts={[
                  { v: "true", l: "Enabled" },
                  { v: "false", l: "Paused" },
                ]}
              />
            </div>
            <Btn onClick={saveSchedule} disabled={scheduleBusy || !scheduleName.trim()}>
              {scheduleBusy ? "Working…" : "Save Schedule"}
            </Btn>
            <Btn onClick={deleteSchedule} disabled={!selectedSchedule || scheduleBusy} color="var(--accent)">
              Delete
            </Btn>
          </div>

          {selectedSchedule && (
            <div style={{ display: "grid", gap: 6, color: "var(--text-dim)", fontSize: 11 }}>
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                <Tag color={selectedSchedule.enabled ? "var(--green)" : "var(--gold)"}>
                  {selectedSchedule.enabled ? "enabled" : "paused"}
                </Tag>
                <Tag>every {selectedSchedule.cadence_hours}h</Tag>
                <Tag>next {formatTimestamp(selectedSchedule.next_run_at)}</Tag>
                <Tag>last run {selectedSchedule.last_run_at ? timeAgo(selectedSchedule.last_run_at) : "never"}</Tag>
                <Tag>status {selectedSchedule.last_status}</Tag>
              </div>
              {selectedSchedule.last_error && (
                <div style={{ color: "var(--accent)" }}>{selectedSchedule.last_error}</div>
              )}
            </div>
          )}

          {scheduleError && (
            <div style={{ color: "var(--accent)", fontSize: 11 }}>
              {scheduleError}
            </div>
          )}
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Search Query</div>
            <Input value={params.search_query} onChange={(value) => set("search_query", value)} placeholder="bug triage, maintenance, backlog" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Topics</div>
            <Input value={params.topics} onChange={(value) => set("topics", value)} placeholder="payments, api, maintenance" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Languages</div>
            <Input value={params.languages} onChange={(value) => set("languages", value)} placeholder="rust,typescript,python" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Min Stars</div>
            <Input value={params.min_stars} onChange={(value) => set("min_stars", value)} placeholder="25" type="number" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Max Repos</div>
            <Input value={params.max_repos} onChange={(value) => set("max_repos", value)} placeholder="8" type="number" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Issues / Repo</div>
            <Input value={params.issues_per_repo} onChange={(value) => set("issues_per_repo", value)} placeholder="30" type="number" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Stale Threshold (days)</div>
            <Input value={params.stale_days} onChange={(value) => set("stale_days", value)} placeholder="45" type="number" />
          </div>
        </div>

        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 12, flexWrap: "wrap" }}>
          <div style={{ color: "var(--text-muted)", fontSize: 11, lineHeight: 1.5 }}>
            SignalHive only reads GitHub metadata and code search results. It does not clone repos or
            write anything. If you set an allowlist in Controls, that list can drive scans even when
            the search fields are blank.
          </div>
          <Btn onClick={onRun} disabled={running}>
            {running ? "Scanning…" : "Run Signal Scan"}
          </Btn>
        </div>
      </div>

      {!scan && <EmptyState icon="📡" text="Run your first SignalHive scan to generate a ranked maintenance queue." />}

      {scan && (
        <div style={{ display: "grid", gap: 16 }}>
          <ReportDashboard
            scan={scan}
            timeline={timeline}
            onCopySummary={copySummary}
            onExportMarkdown={() => downloadReport(scan.id)}
            onExportHtml={() => exportDashboardHtml(scan, timeline)}
          />

          <div style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap" }}>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                <div style={{ fontSize: 18, fontWeight: 700 }}>Latest Ranked Queue</div>
                <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
                  {scan.summary.total_repos} repos scanned • {scan.summary.total_signals} signals found • top repo {scan.summary.top_repo}
                </div>
              </div>
              <div style={{ display: "flex", gap: 10, alignItems: "flex-end", flexWrap: "wrap" }}>
                <Btn onClick={() => downloadReport(scan.id)} color="var(--gold)">
                  Export Report
                </Btn>
                <div style={{ minWidth: 180, ...S.field }}>
                  <div style={S.label}>Sort Queue</div>
                  <Sel value={sortBy} onChange={setSortBy} opts={SORT_OPTIONS} />
                </div>
                <div style={{ color: "var(--text-muted)", fontSize: 11 }}>
                  Scan ID {scan.id}
                </div>
              </div>
            </div>

            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              <Tag color={scan.trigger_type === "scheduled" ? "var(--gold)" : "var(--accent)"}>
                {scan.trigger_type || "manual"}
              </Tag>
              {scan.schedule_name && <Tag>{scan.schedule_name}</Tag>}
            </div>

            {scan.trend && (
              <div style={{ border: "1px solid var(--border)", borderRadius: 8, padding: "12px 14px", background: "var(--bg)", display: "grid", gap: 8 }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap" }}>
                  <div style={{ fontSize: 13, fontWeight: 700 }}>Trend vs previous similar scan</div>
                  <div style={{ color: "var(--text-muted)", fontSize: 10 }}>
                    {timeAgo(scan.trend.compared_to_created_at)}
                  </div>
                </div>
                <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                  <TrendTag label="Signals" value={scan.trend.total_signals_delta} />
                  <TrendTag label="Repos" value={scan.trend.total_repos_delta} />
                  <Tag>{scan.trend.new_repos} new</Tag>
                  <Tag>{scan.trend.dropped_repos} dropped</Tag>
                  <Tag color="var(--accent)">{scan.trend.rising_repos} rising</Tag>
                  <Tag color="var(--green)">{scan.trend.improving_repos} improving</Tag>
                  <Tag color="var(--gold)">{scan.trend.steady_repos} steady</Tag>
                </div>
              </div>
            )}
          </div>

          <ScanTimelineChart timeline={timeline} />

          {sortedRepos.length === 0 ? (
            <EmptyState icon="◌" text="No repositories matched strongly enough to rank in this scan." />
          ) : (
            sortedRepos.map((repo) => <SignalCard key={repo.full_name} repo={repo} />)
          )}
        </div>
      )}
    </div>
  );
}
