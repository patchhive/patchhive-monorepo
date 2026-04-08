import { S, Divider, ScoreBadge, Tag } from "@patchhivehq/ui";

function Stat({ label, value }) {
  return (
    <div style={{ ...S.field, minWidth: 90 }}>
      <div style={S.label}>{label}</div>
      <div style={{ fontSize: 16, fontWeight: 700, color: "var(--text)" }}>{value}</div>
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
        <Stat label="Stale Issues" value={repo.stale_issues} />
        <Stat label="Duplicates" value={repo.duplicate_candidates.length} />
        <Stat label="TODO" value={repo.todo_count} />
        <Stat label="FIXME" value={repo.fixme_count} />
      </div>

      {repo.signals?.length > 0 && (
        <>
          <Divider />
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <div style={S.label}>Why It Ranked</div>
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
    </div>
  );
}
