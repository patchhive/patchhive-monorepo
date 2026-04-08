import { S, Divider, ScoreBadge, Tag } from "@patchhivehq/ui";

function Stat({ label, value }) {
  return (
    <div style={{ ...S.field, minWidth: 90 }}>
      <div style={S.label}>{label}</div>
      <div style={{ fontSize: 16, fontWeight: 700, color: "var(--text)" }}>{value}</div>
    </div>
  );
}

function impactColor(impact) {
  if (impact >= 15) {
    return "var(--accent)";
  }
  if (impact >= 8) {
    return "var(--gold)";
  }
  return "var(--green)";
}

function ScoreFactorRow({ factor }) {
  const color = impactColor(factor.impact);
  return (
    <div
      style={{
        border: "1px solid var(--border)",
        borderRadius: 6,
        padding: "10px 12px",
        background: "var(--bg)",
        display: "grid",
        gap: 5,
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", gap: 10, alignItems: "center" }}>
        <div style={{ fontSize: 12, fontWeight: 700, color: "var(--text)" }}>{factor.label}</div>
        <div
          style={{
            color,
            fontSize: 10,
            fontWeight: 700,
            border: `1px solid ${color}55`,
            borderRadius: 999,
            padding: "2px 7px",
            whiteSpace: "nowrap",
          }}
        >
          +{Math.round(factor.impact)}
        </div>
      </div>
      <div style={{ color: "var(--text-dim)", fontSize: 11, lineHeight: 1.5 }}>{factor.detail}</div>
    </div>
  );
}

export default function SignalCard({ repo }) {
  return (
    <div style={{ ...S.panel, display: "flex", flexDirection: "column", gap: 14 }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 12 }}>
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          <a
            href={repo.repo_url}
            target="_blank"
            rel="noreferrer"
            style={{ color: "var(--accent)", textDecoration: "none", fontSize: 17, fontWeight: 700 }}
          >
            {repo.full_name}
          </a>
          <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.5 }}>
            {repo.description || "No repository description available."}
          </div>
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
            <Tag color="var(--accent)">{repo.language}</Tag>
            <Tag>{repo.stars} stars</Tag>
            <Tag>{repo.open_issues} open issues</Tag>
          </div>
        </div>
        <ScoreBadge score={Math.round(repo.priority_score)} />
      </div>

      <div style={{ color: "var(--text)", fontSize: 13, lineHeight: 1.5 }}>{repo.summary}</div>

      <div style={{ display: "flex", gap: 18, flexWrap: "wrap" }}>
        <Stat label="Sampled Issues" value={repo.sampled_issues} />
        <Stat label="Stale Issues" value={repo.stale_issues} />
        <Stat label="Unlabeled" value={repo.unlabeled_issues} />
        <Stat label="Stale Bugs" value={repo.stale_bug_issues} />
        <Stat label="Stale w/Discussion" value={repo.stale_high_comment_issues} />
        <Stat label="Recurring Bugs" value={repo.recurring_bug_clusters.length} />
        <Stat label="Duplicates" value={repo.duplicate_candidates.length} />
        <Stat label="TODO" value={repo.todo_count} />
        <Stat label="FIXME" value={repo.fixme_count} />
      </div>

      {repo.score_breakdown?.length > 0 && (
        <>
          <Divider />
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <div style={S.label}>Score Drivers</div>
            {repo.score_breakdown.map((factor) => (
              <ScoreFactorRow key={factor.key} factor={factor} />
            ))}
          </div>
        </>
      )}

      {repo.signals?.length > 0 && (
        <>
          <Divider />
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <div style={S.label}>Maintenance Signals</div>
            {repo.signals.map((signal) => (
              <div key={signal} style={{ color: "var(--text-dim)", fontSize: 12 }}>
                • {signal}
              </div>
            ))}
          </div>
        </>
      )}

      {repo.issue_examples?.length > 0 && (
        <>
          <Divider />
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <div style={S.label}>Stale Issue Examples</div>
            {repo.issue_examples.map((issue) => (
              <a
                key={issue.number}
                href={issue.url}
                target="_blank"
                rel="noreferrer"
                style={{
                  color: "var(--text)",
                  textDecoration: "none",
                  border: "1px solid var(--border)",
                  borderRadius: 6,
                  padding: "10px 12px",
                  display: "flex",
                  justifyContent: "space-between",
                  gap: 12,
                  background: "var(--bg)",
                }}
              >
                <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  <div style={{ fontWeight: 600 }}>#{issue.number} {issue.title}</div>
                  <div style={{ color: "var(--text-dim)", fontSize: 11 }}>
                    Last updated {issue.age_days}d ago • {issue.comments} comments
                  </div>
                </div>
              </a>
            ))}
          </div>
        </>
      )}

      {repo.duplicate_candidates?.length > 0 && (
        <>
          <Divider />
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <div style={S.label}>Potential Duplicates</div>
            {repo.duplicate_candidates.map((pair) => (
              <div
                key={`${pair.left_number}-${pair.right_number}`}
                style={{ border: "1px solid var(--border)", borderRadius: 6, padding: "10px 12px", background: "var(--bg)" }}
              >
                <div style={{ color: "var(--text)", fontSize: 12, fontWeight: 600 }}>
                  #{pair.left_number} ↔ #{pair.right_number} ({Math.round(pair.similarity * 100)}% overlap)
                </div>
                <div style={{ color: "var(--text-dim)", fontSize: 11, marginTop: 6 }}>
                  {pair.left_title}
                </div>
                <div style={{ color: "var(--text-dim)", fontSize: 11 }}>
                  {pair.right_title}
                </div>
              </div>
            ))}
          </div>
        </>
      )}

      {repo.recurring_bug_clusters?.length > 0 && (
        <>
          <Divider />
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <div style={S.label}>Recurring Bug Patterns</div>
            {repo.recurring_bug_clusters.map((cluster) => (
              <div
                key={`${repo.full_name}-${cluster.label}`}
                style={{ border: "1px solid var(--border)", borderRadius: 6, padding: "10px 12px", background: "var(--bg)", display: "grid", gap: 8 }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
                  <div style={{ color: "var(--text)", fontSize: 12, fontWeight: 700 }}>{cluster.label}</div>
                  <Tag color="var(--gold)">{cluster.issue_count} issues</Tag>
                </div>
                {cluster.shared_terms?.length > 0 && (
                  <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                    {cluster.shared_terms.map((term) => (
                      <Tag key={term}>{term}</Tag>
                    ))}
                  </div>
                )}
                <div style={{ display: "grid", gap: 6 }}>
                  {cluster.examples.map((issue) => (
                    <a
                      key={`${cluster.label}-${issue.number}`}
                      href={issue.url}
                      target="_blank"
                      rel="noreferrer"
                      style={{ color: "var(--text-dim)", textDecoration: "none", fontSize: 11, lineHeight: 1.5 }}
                    >
                      #{issue.number} {issue.title} • {issue.comments} comments • {issue.age_days}d old
                    </a>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
