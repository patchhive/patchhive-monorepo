import { Btn, S, Tag } from "@patchhivehq/ui";
import { buildDashboardSummary, summarizeScanHighlights } from "../report.js";

function StatCard({ label, value, detail, color = "var(--text)" }) {
  return (
    <div
      style={{
        ...S.field,
        minWidth: 170,
        border: "1px solid var(--border)",
        borderRadius: 10,
        padding: "12px 14px",
        background:
          "linear-gradient(180deg, var(--bg-panel), color-mix(in srgb, var(--bg-panel) 82%, black))",
      }}
    >
      <div style={S.label}>{label}</div>
      <div style={{ fontSize: 24, fontWeight: 800, color, letterSpacing: "-0.05em" }}>{value}</div>
      {detail && <div style={{ color: "var(--text-dim)", fontSize: 11, lineHeight: 1.5 }}>{detail}</div>}
    </div>
  );
}

function InsightCard({ title, accent = "var(--accent)", children }) {
  return (
    <div
      style={{
        border: "1px solid var(--border)",
        borderRadius: 10,
        padding: "14px 16px",
        background: "linear-gradient(180deg, var(--bg), color-mix(in srgb, var(--bg-panel) 78%, black))",
        display: "grid",
        gap: 10,
      }}
    >
      <div style={{ fontSize: 12, fontWeight: 700, color: accent, letterSpacing: "0.08em", textTransform: "uppercase" }}>
        {title}
      </div>
      {children}
    </div>
  );
}

export default function ReportDashboard({
  scan,
  timeline,
  onExportMarkdown,
  onExportHtml,
  onCopySummary,
}) {
  const repos = scan?.repos || [];
  const {
    topRepo,
    topRecurring,
    topStale,
    topDuplicates,
    topMarkers,
    risingRepos,
    improvingRepos,
  } = summarizeScanHighlights(scan);
  const summary = buildDashboardSummary(scan);

  return (
    <div
      style={{
        ...S.panel,
        display: "grid",
        gap: 18,
        background:
          "radial-gradient(circle at top right, color-mix(in srgb, var(--accent) 12%, transparent), transparent 28%), linear-gradient(180deg, var(--bg-panel), color-mix(in srgb, var(--bg-panel) 82%, black))",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", gap: 14, flexWrap: "wrap", alignItems: "flex-start" }}>
        <div style={{ display: "grid", gap: 8, maxWidth: 860 }}>
          <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
            <div
              style={{
                fontSize: 11,
                color: "var(--accent)",
                border: "1px solid var(--accent-dim)",
                borderRadius: 999,
                padding: "4px 10px",
                letterSpacing: "0.12em",
                textTransform: "uppercase",
                fontWeight: 700,
              }}
            >
              SignalHive Report
            </div>
            <Tag color={scan.trigger_type === "scheduled" ? "var(--gold)" : "var(--accent)"}>
              {scan.trigger_type || "manual"}
            </Tag>
            {scan.schedule_name && <Tag>{scan.schedule_name}</Tag>}
            <Tag>{timeline?.points?.length || 0} timeline points</Tag>
          </div>
          <div style={{ fontSize: 24, fontWeight: 800, letterSpacing: "-0.05em" }}>
            Maintenance pressure at a glance
          </div>
          <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>
            {summary}
          </div>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <Btn onClick={onCopySummary}>Copy Readout</Btn>
          <Btn onClick={onExportMarkdown} color="var(--gold)">Export Markdown</Btn>
          <Btn onClick={onExportHtml} color="var(--accent)">Export Snapshot</Btn>
        </div>
      </div>

      <div style={{ display: "flex", gap: 14, flexWrap: "wrap" }}>
        <StatCard label="Repos" value={scan.summary.total_repos} />
        <StatCard label="Signals" value={scan.summary.total_signals} />
        <StatCard label="Top Repo" value={scan.summary.top_repo} detail={topRepo ? `${Math.round(topRepo.priority_score)} priority` : ""} color="var(--accent)" />
        <StatCard label="Most Stale" value={topStale?.stale_issues || 0} detail={topStale?.full_name || "No stale backlog spike"} color="var(--gold)" />
        <StatCard
          label="Recurring Bugs"
          value={topRecurring?.recurring_bug_clusters?.length || 0}
          detail={topRecurring?.full_name || "No strong recurring cluster"}
          color="var(--green)"
        />
      </div>

      {scan.trend && (
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
          <div style={{ ...S.label, marginRight: 4 }}>Scan Trend</div>
          <Tag color={scan.trend.total_signals_delta > 0 ? "var(--accent)" : scan.trend.total_signals_delta < 0 ? "var(--green)" : "var(--gold)"}>
            Signals {scan.trend.total_signals_delta > 0 ? "+" : ""}{scan.trend.total_signals_delta}
          </Tag>
          <Tag color={scan.trend.total_repos_delta > 0 ? "var(--accent)" : scan.trend.total_repos_delta < 0 ? "var(--green)" : "var(--gold)"}>
            Repos {scan.trend.total_repos_delta > 0 ? "+" : ""}{scan.trend.total_repos_delta}
          </Tag>
          <Tag color="var(--accent)">{scan.trend.rising_repos} rising</Tag>
          <Tag color="var(--green)">{scan.trend.improving_repos} improving</Tag>
          <Tag color="var(--gold)">{scan.trend.steady_repos} steady</Tag>
        </div>
      )}

      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))", gap: 12 }}>
        <InsightCard title="Top Queue" accent="var(--accent)">
          <div style={{ display: "grid", gap: 8 }}>
            {repos.slice(0, 5).map((repo) => (
              <div key={`queue-${repo.full_name}`} style={{ display: "grid", gap: 4 }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: 10 }}>
                  <div style={{ fontWeight: 700, fontSize: 12 }}>{repo.full_name}</div>
                  <Tag color="var(--accent)">{Math.round(repo.priority_score)}</Tag>
                </div>
                <div style={{ color: "var(--text-dim)", fontSize: 11 }}>{repo.summary}</div>
              </div>
            ))}
          </div>
        </InsightCard>

        <InsightCard title="Biggest Movers" accent="var(--gold)">
          <div style={{ display: "grid", gap: 10 }}>
            <div>
              <div style={{ ...S.label, marginBottom: 6 }}>Rising</div>
              {risingRepos.length === 0 ? (
                <div style={{ color: "var(--text-dim)", fontSize: 11 }}>No sharply rising repos in this scan.</div>
              ) : (
                risingRepos.slice(0, 3).map((repo) => (
                  <div key={`rising-${repo.full_name}`} style={{ color: "var(--text)", fontSize: 11, marginBottom: 4 }}>
                    {repo.full_name} • score {repo.trend?.priority_delta > 0 ? "+" : ""}{repo.trend?.priority_delta.toFixed(1)}
                  </div>
                ))
              )}
            </div>
            <div>
              <div style={{ ...S.label, marginBottom: 6 }}>Improving</div>
              {improvingRepos.length === 0 ? (
                <div style={{ color: "var(--text-dim)", fontSize: 11 }}>No strong improvement movement yet.</div>
              ) : (
                improvingRepos.slice(0, 3).map((repo) => (
                  <div key={`improving-${repo.full_name}`} style={{ color: "var(--text)", fontSize: 11, marginBottom: 4 }}>
                    {repo.full_name} • score {repo.trend?.priority_delta.toFixed(1)}
                  </div>
                ))
              )}
            </div>
          </div>
        </InsightCard>

        <InsightCard title="Pressure Highlights" accent="var(--green)">
          <div style={{ display: "grid", gap: 8, fontSize: 11, color: "var(--text)" }}>
            <div>
              <span style={{ color: "var(--text-dim)" }}>Duplicate hotspot:</span>{" "}
              {topDuplicates ? `${topDuplicates.full_name} • ${topDuplicates.duplicate_candidates.length} likely duplicate pairs` : "none"}
            </div>
            <div>
              <span style={{ color: "var(--text-dim)" }}>Marker hotspot:</span>{" "}
              {topMarkers ? `${topMarkers.full_name} • ${topMarkers.todo_count + topMarkers.fixme_count} TODO/FIXME markers` : "none"}
            </div>
            <div>
              <span style={{ color: "var(--text-dim)" }}>Recurring cluster leader:</span>{" "}
              {topRecurring ? `${topRecurring.full_name} • ${topRecurring.recurring_bug_clusters.length} clusters` : "none"}
            </div>
          </div>
        </InsightCard>
      </div>
    </div>
  );
}
