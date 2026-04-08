import { useEffect, useMemo, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, S, Sel, Tag, timeAgo } from "@patchhivehq/ui";
import ReportDashboard from "../components/ReportDashboard.jsx";
import ScanTimelineChart from "../components/ScanTimelineChart.jsx";
import SignalCard from "../components/SignalCard.jsx";
import { buildDashboardSummary, downloadTextFile, exportDashboardHtml } from "../report.js";
import { SORT_OPTIONS, sortRepos } from "../sort.js";

function TrendTag({ label, value }) {
  const color = value > 0 ? "var(--accent)" : value < 0 ? "var(--green)" : "var(--gold)";
  return <Tag color={color}>{label} {value > 0 ? "+" : ""}{value}</Tag>;
}

export default function HistoryPanel({ apiKey }) {
  const [history, setHistory] = useState([]);
  const [selected, setSelected] = useState(null);
  const [timeline, setTimeline] = useState(null);
  const [loading, setLoading] = useState(false);
  const [sortBy, setSortBy] = useState("priority");
  const fetch_ = createApiFetcher(apiKey);
  const sortedRepos = useMemo(() => sortRepos(selected?.repos || [], sortBy), [selected, sortBy]);

  const loadHistory = () =>
    fetch_(`${API}/history`)
      .then((res) => res.json())
      .then((data) => setHistory(data.scans || []))
      .catch(() => setHistory([]));

  const loadScan = (id) => {
    setLoading(true);
    setTimeline(null);
    fetch_(`${API}/history/${id}`)
      .then((res) => res.json())
      .then(setSelected)
      .finally(() => setLoading(false));
    fetch_(`${API}/history/${id}/timeline`)
      .then((res) => res.json())
      .then(setTimeline)
      .catch(() => setTimeline(null));
  };

  useEffect(() => {
    loadHistory();
  }, [apiKey]);

  const downloadReport = async (scanId) => {
    if (!scanId) {
      return;
    }
    const res = await fetch_(`${API}/history/${scanId}/report`);
    const data = await res.json();
    if (!res.ok) {
      return;
    }
    downloadTextFile(
      data.filename || `signalhive-report-${scanId}.md`,
      data.markdown,
      "text/markdown;charset=utf-8",
    );
  };

  const copySummary = async () => {
    if (!selected) {
      return;
    }
    try {
      await navigator.clipboard.writeText(buildDashboardSummary(selected));
    } catch {}
  };

  return (
    <div style={{ display: "grid", gridTemplateColumns: "340px 1fr", gap: 18 }}>
      <div style={{ ...S.panel, display: "flex", flexDirection: "column", gap: 10, minHeight: 500 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8 }}>
          <div style={{ fontSize: 16, fontWeight: 700 }}>Scan History</div>
          <Btn onClick={loadHistory} style={{ padding: "5px 10px" }}>Refresh</Btn>
        </div>

        {history.length === 0 ? (
          <EmptyState icon="◎" text="Signal scans will show up here after you run them." />
        ) : (
          history.map((scan) => (
            <button
              key={scan.id}
              onClick={() => loadScan(scan.id)}
              style={{
                textAlign: "left",
                background: selected?.id === scan.id ? "var(--accent)10" : "var(--bg)",
                border: `1px solid ${selected?.id === scan.id ? "var(--accent)55" : "var(--border)"}`,
                borderRadius: 6,
                color: "var(--text)",
                cursor: "pointer",
                padding: "10px 12px",
                fontFamily: "inherit",
              }}
            >
              <div style={{ fontSize: 12, fontWeight: 700 }}>{scan.top_repo || "No top repo"}</div>
              <div style={{ color: "var(--text-dim)", fontSize: 11, marginTop: 4 }}>
                {scan.total_repos} repos • {scan.total_signals} signals • {timeAgo(scan.created_at)}
              </div>
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginTop: 6 }}>
                <Tag color={scan.trigger_type === "scheduled" ? "var(--gold)" : "var(--accent)"}>
                  {scan.trigger_type || "manual"}
                </Tag>
                {scan.schedule_name && <Tag>{scan.schedule_name}</Tag>}
              </div>
              <div style={{ color: "var(--text-muted)", fontSize: 10, marginTop: 6 }}>
                {scan.search_query || scan.topics.join(", ") || scan.languages.join(", ")}
              </div>
            </button>
          ))
        )}
      </div>

      <div style={{ display: "grid", gap: 16 }}>
        {!selected && !loading && <EmptyState icon="📂" text="Choose a saved scan to review its ranked queue." />}
        {loading && <div style={{ ...S.panel, color: "var(--text-dim)" }}>Loading scan…</div>}

        {selected && !loading && (
          <>
            <ReportDashboard
              scan={selected}
              timeline={timeline}
              onCopySummary={copySummary}
              onExportMarkdown={() => downloadReport(selected.id)}
              onExportHtml={() => exportDashboardHtml(selected, timeline)}
            />

            <div style={{ ...S.panel, display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap" }}>
              <div>
                <div style={{ fontSize: 18, fontWeight: 700 }}>Saved Scan</div>
                <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
                  {selected.summary.total_repos} repos • {selected.summary.total_signals} signals • {timeAgo(selected.created_at)}
                </div>
                <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginTop: 8 }}>
                  <Tag color={selected.trigger_type === "scheduled" ? "var(--gold)" : "var(--accent)"}>
                    {selected.trigger_type || "manual"}
                  </Tag>
                  {selected.schedule_name && <Tag>{selected.schedule_name}</Tag>}
                </div>
              </div>
              <div style={{ display: "flex", gap: 12, alignItems: "flex-end", flexWrap: "wrap" }}>
                <Btn onClick={() => downloadReport(selected.id)} color="var(--gold)">
                  Export Report
                </Btn>
                <div style={{ minWidth: 180, ...S.field }}>
                  <div style={S.label}>Sort Queue</div>
                  <Sel value={sortBy} onChange={setSortBy} opts={SORT_OPTIONS} />
                </div>
                <div style={{ color: "var(--text-muted)", fontSize: 11 }}>Scan ID {selected.id}</div>
              </div>
            </div>

            {selected.trend && (
              <div style={{ ...S.panel, display: "grid", gap: 8 }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap" }}>
                  <div style={{ fontSize: 13, fontWeight: 700 }}>Trend vs previous similar scan</div>
                  <div style={{ color: "var(--text-muted)", fontSize: 10 }}>
                    {timeAgo(selected.trend.compared_to_created_at)}
                  </div>
                </div>
                <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                  <TrendTag label="Signals" value={selected.trend.total_signals_delta} />
                  <TrendTag label="Repos" value={selected.trend.total_repos_delta} />
                  <Tag>{selected.trend.new_repos} new</Tag>
                  <Tag>{selected.trend.dropped_repos} dropped</Tag>
                  <Tag color="var(--accent)">{selected.trend.rising_repos} rising</Tag>
                  <Tag color="var(--green)">{selected.trend.improving_repos} improving</Tag>
                  <Tag color="var(--gold)">{selected.trend.steady_repos} steady</Tag>
                </div>
              </div>
            )}
            <ScanTimelineChart timeline={timeline} />
            {sortedRepos.map((repo) => <SignalCard key={repo.full_name} repo={repo} />)}
          </>
        )}
      </div>
    </div>
  );
}
