function scoreColor(impact) {
  if (impact >= 15) return "var(--red)";
  if (impact >= 8) return "var(--amber)";
  return "var(--green)";
}

function scoreClass(impact) {
  if (impact >= 15) return "score-high";
  if (impact >= 8) return "score-medium";
  return "score-low";
}

function StatChip({ value, label, color }) {
  return (
    <div className="stat-chip" style={color ? { borderColor: color + "33" } : {}}>
      <span className="stat-value" style={color ? { color } : {}}>{value}</span>
      <span className="stat-label">{label}</span>
    </div>
  );
}

function ScoreFactorRow({ factor }) {
  return (
    <div style={{ display: "grid", gap: 4 }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span style={{ fontSize: 12, fontWeight: 500, color: "var(--text)" }}>{factor.label}</span>
        <span style={{ fontSize: 12, fontWeight: 600, color: scoreColor(factor.impact) }}>{factor.impact.toFixed(1)}</span>
      </div>
      <div className="score-bar">
        <div className={`score-bar-fill ${scoreClass(factor.impact)}`} style={{ width: `${Math.min(factor.impact * 5, 100)}%` }} />
      </div>
      <span style={{ fontSize: 11, color: "var(--text-muted)" }}>{factor.detail}</span>
    </div>
  );
}

function DupeCard({ dupe }) {
  return (
    <div style={{ padding: "8px 10px", background: "var(--surface)", borderRadius: "var(--radius-sm)", fontSize: 12, border: "1px solid var(--border)" }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 8, marginBottom: 4 }}>
        <a href={`https://github.com/${dupe.left_title.split("/").slice(0,2).join("/")}/issues/${dupe.left_number}`} target="_blank" rel="noopener noreferrer" style={{ color: "var(--accent-light)", textDecoration: "none" }}>
          #{dupe.left_number}
        </a>
        <span style={{ color: "var(--text-muted)" }}>{dupe.similarity?.toFixed(0)}% match</span>
        <a href={`https://github.com/${dupe.right_title.split("/").slice(0,2).join("/")}/issues/${dupe.right_number}`} target="_blank" rel="noopener noreferrer" style={{ color: "var(--accent-light)", textDecoration: "none" }}>
          #{dupe.right_number}
        </a>
      </div>
      <div style={{ color: "var(--text-secondary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
        {dupe.left_title} <span style={{ color: "var(--text-muted)" }}>vs</span> {dupe.right_title}
      </div>
    </div>
  );
}

export default function SignalCard({ repo, expanded, onToggle }) {
  const maxImpact = Math.max(...(repo.score_factors || []).map(f => f.impact), 1);
  const doneScans = repo.scanned_markers !== undefined;

  return (
    <div className="signal-card">
      <div className="signal-card-header" onClick={onToggle}>
        <div className="repo-info">
          <span style={{ fontSize: 16 }}>📦</span>
          <div>
            <div className="repo-name">
              <a href={repo.repo_url} target="_blank" rel="noopener noreferrer" style={{ color: "inherit", textDecoration: "none" }}>{repo.full_name}</a>
            </div>
            <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 2 }}>
              {repo.language} · {repo.stars} stars · {repo.open_issues} open issues
            </div>
          </div>
        </div>
        <div className="repo-meta">
          <span className="tag tag-accent">Priority {repo.priority_score?.toFixed(1) || "—"}</span>
          <span style={{ fontSize: 12, color: "var(--text-muted)", transition: "transform 0.2s" }}>
            {expanded ? "▾" : "▸"}
          </span>
        </div>
      </div>

      {expanded && (
        <div className="signal-card-body">
          {/* Stats */}
          <div className="signal-stats">
            <StatChip value={repo.stale_issues || 0} label="stale" color="var(--red)" />
            <StatChip value={repo.todo_count || 0} label="TODOs" color="var(--accent-light)" />
            <StatChip value={repo.duplicate_candidates?.length || 0} label="dupes" color="var(--amber)" />
            <StatChip value={repo.recurring_bug_clusters?.length || 0} label="bug clusters" color="var(--red)" />
            {doneScans && (
              <>
                <StatChip value={repo.scanned_markers} label="markers scanned" />
                <StatChip value={repo.stale_bug_issues || 0} label="stale bugs" />
              </>
            )}
          </div>

          {/* Score Factors */}
          {repo.score_factors?.length > 0 && (
            <div style={{ display: "grid", gap: 8 }}>
              <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.05em" }}>Score Breakdown</div>
              {repo.score_factors.map((f, i) => (
                <ScoreFactorRow key={i} factor={f} />
              ))}
            </div>
          )}

          {/* Duplicates */}
          {repo.duplicate_candidates?.length > 0 && (
            <div style={{ display: "grid", gap: 6 }}>
              <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.05em" }}>
                Duplicate Candidates ({repo.duplicate_candidates.length})
              </div>
              <div style={{ display: "grid", gap: 6 }}>
                {repo.duplicate_candidates.slice(0, 5).map((d, i) => (
                  <DupeCard key={i} dupe={d} />
                ))}
                {repo.duplicate_candidates.length > 5 && (
                  <div style={{ fontSize: 11, color: "var(--text-muted)", textAlign: "center" }}>
                    +{repo.duplicate_candidates.length - 5} more
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Bug Clusters */}
          {repo.recurring_bug_clusters?.length > 0 && (
            <div style={{ display: "grid", gap: 6 }}>
              <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.05em" }}>
                Bug Clusters ({repo.recurring_bug_clusters.length})
              </div>
              {repo.recurring_bug_clusters.slice(0, 3).map((c, i) => (
                <div key={i} style={{ padding: "8px 10px", background: "var(--surface)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border)" }}>
                  <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 4 }}>{c.label}</div>
                  <div style={{ fontSize: 11, color: "var(--text-muted)" }}>
                    {c.issue_count} issues · {c.shared_terms?.slice(0, 4).join(", ")}{c.shared_terms?.length > 4 ? "…" : ""}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
