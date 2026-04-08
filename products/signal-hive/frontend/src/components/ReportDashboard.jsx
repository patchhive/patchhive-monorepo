import { Btn, S, Tag } from "@patchhivehq/ui";
import { buildDashboardSummary } from "../report.js";

function StatCard({ label, value, detail, color = "var(--text)" }) {
  return (
    <div style={{ ...S.field, minWidth: 160 }}>
      <div style={S.label}>{label}</div>
      <div style={{ fontSize: 24, fontWeight: 700, color }}>{value}</div>
      {detail && <div style={{ color: "var(--text-dim)", fontSize: 11, lineHeight: 1.5 }}>{detail}</div>}
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
  const topRepo = repos[0] || null;
  const topStale = [...repos].sort((left, right) => right.stale_issues - left.stale_issues)[0] || null;
  const topRecurring = [...repos].sort(
    (left, right) =>
      right.recurring_bug_clusters.reduce((sum, cluster) => sum + cluster.issue_count, 0) -
      left.recurring_bug_clusters.reduce((sum, cluster) => sum + cluster.issue_count, 0),
  )[0] || null;
  const risingRepos = repos.filter((repo) => repo.trend?.status === "rising").slice(0, 3);
  const improvingRepos = repos.filter((repo) => repo.trend?.status === "improving").slice(0, 3);
  const summary = buildDashboardSummary(scan);

  return (
    <div style={{ ...S.panel, display: "grid", gap: 16 }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "flex-start" }}>
        <div style={{ display: "grid", gap: 6, maxWidth: 860 }}>
          <div style={{ fontSize: 19, fontWeight: 700 }}>Report Dashboard</div>
          <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>
            {summary}
          </div>
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
            <Tag color={scan.trigger_type === "scheduled" ? "var(--gold)" : "var(--accent)"}>
              {scan.trigger_type || "manual"}
            </Tag>
            {scan.schedule_name && <Tag>{scan.schedule_name}</Tag>}
            <Tag>{timeline?.points?.length || 0} timeline points</Tag>
          </div>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <Btn onClick={onCopySummary}>Copy Summary</Btn>
          <Btn onClick={onExportMarkdown} color="var(--gold)">Export Markdown</Btn>
          <Btn onClick={onExportHtml} color="var(--accent)">Export Dashboard</Btn>
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
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
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
        <div style={{ border: "1px solid var(--border)", borderRadius: 8, padding: "12px 14px", background: "var(--bg)" }}>
          <div style={{ fontSize: 12, fontWeight: 700, marginBottom: 10 }}>Top Queue</div>
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
        </div>

        <div style={{ border: "1px solid var(--border)", borderRadius: 8, padding: "12px 14px", background: "var(--bg)" }}>
          <div style={{ fontSize: 12, fontWeight: 700, marginBottom: 10 }}>Biggest Movers</div>
          <div style={{ display: "grid", gap: 10 }}>
            <div>
              <div style={{ ...S.label, marginBottom: 6 }}>Rising</div>
              {risingRepos.length === 0 ? (
                <div style={{ color: "var(--text-dim)", fontSize: 11 }}>No sharply rising repos in this scan.</div>
              ) : (
                risingRepos.map((repo) => (
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
                improvingRepos.map((repo) => (
                  <div key={`improving-${repo.full_name}`} style={{ color: "var(--text)", fontSize: 11, marginBottom: 4 }}>
                    {repo.full_name} • score {repo.trend?.priority_delta.toFixed(1)}
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
