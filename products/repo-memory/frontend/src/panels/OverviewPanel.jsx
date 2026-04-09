import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, S, Tag, timeAgo } from "@patchhivehq/ui";

function confidenceColor(confidence) {
  if (confidence >= 80) return "var(--green)";
  if (confidence >= 60) return "var(--gold)";
  return "var(--accent)";
}

export default function OverviewPanel({ apiKey, activeRepo, setActiveRepo, onOpenIngest }) {
  const [overview, setOverview] = useState(null);
  const fetch_ = createApiFetcher(apiKey);

  const refresh = () => {
    fetch_(`${API}/overview`)
      .then((res) => res.json())
      .then(setOverview)
      .catch(() => setOverview(null));
  };

  useEffect(() => {
    refresh();
  }, [apiKey]);

  return (
    <div style={{ display: "grid", gap: 18 }}>
      <div style={{ ...S.panel, display: "grid", gap: 12 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 16, alignItems: "flex-start", flexWrap: "wrap" }}>
          <div style={{ display: "grid", gap: 8 }}>
            <div style={{ fontSize: 20, fontWeight: 800 }}>RepoMemory builds a durable knowledge layer for repos.</div>
            <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.7, maxWidth: 860 }}>
              It mines merged PRs, reviewer feedback, recurring bug history, and code-review pain so humans and agents stop relearning the
              same repo rules every week.
            </div>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <Tag color="var(--green)">no AI required for MVP</Tag>
              <Tag color="var(--accent)">merged PRs</Tag>
              <Tag color="var(--gold)">review habits</Tag>
              <Tag color="var(--blue)">prompt-pack output</Tag>
            </div>
          </div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <Btn onClick={refresh}>Refresh</Btn>
            <Btn onClick={onOpenIngest}>Ingest a repo</Btn>
          </div>
        </div>
      </div>

      {overview ? (
        <>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 12 }}>
            <div style={{ ...S.panel, display: "grid", gap: 6 }}>
              <div style={S.label}>Tracked repos</div>
              <div style={{ fontSize: 24, fontWeight: 800 }}>{overview.counts.repos}</div>
            </div>
            <div style={{ ...S.panel, display: "grid", gap: 6 }}>
              <div style={S.label}>Memory runs</div>
              <div style={{ fontSize: 24, fontWeight: 800 }}>{overview.counts.runs}</div>
            </div>
            <div style={{ ...S.panel, display: "grid", gap: 6 }}>
              <div style={S.label}>Memory entries</div>
              <div style={{ fontSize: 24, fontWeight: 800 }}>{overview.counts.memories}</div>
            </div>
            <div style={{ ...S.panel, display: "grid", gap: 6 }}>
              <div style={S.label}>Current focus</div>
              <div style={{ fontSize: 13, color: "var(--accent)", fontWeight: 700 }}>{activeRepo || "Pick a repo to explore"}</div>
            </div>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1.1fr 1fr", gap: 16 }}>
            <div style={{ ...S.panel, display: "grid", gap: 12 }}>
              <div style={{ fontSize: 16, fontWeight: 700 }}>Tracked repos</div>
              {overview.repos?.length ? (
                overview.repos.map((repo) => (
                  <div key={repo.repo} style={{ display: "grid", gap: 8, border: "1px solid var(--border)", borderRadius: 8, padding: 12 }}>
                    <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                      <div>
                        <div style={{ fontSize: 14, fontWeight: 700 }}>{repo.repo}</div>
                        <div style={{ color: "var(--text-dim)", fontSize: 11 }}>Last ingested {timeAgo(repo.last_ingested_at)}</div>
                      </div>
                      <Btn onClick={() => setActiveRepo(repo.repo)} style={{ padding: "4px 10px" }}>
                        Focus repo
                      </Btn>
                    </div>
                    <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                      <Tag color="var(--green)">{repo.run_count} runs</Tag>
                      <Tag color="var(--blue)">{repo.memory_count} memories</Tag>
                    </div>
                    <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>{repo.top_memory || "No strong top memory yet."}</div>
                  </div>
                ))
              ) : (
                <EmptyState icon="🧠" text="Run your first RepoMemory ingest to start building durable repo knowledge." />
              )}
            </div>

            <div style={{ ...S.panel, display: "grid", gap: 12 }}>
              <div style={{ fontSize: 16, fontWeight: 700 }}>Featured memories</div>
              {overview.featured_memories?.length ? (
                overview.featured_memories.map((entry) => (
                  <div key={entry.id} style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 12, display: "grid", gap: 8 }}>
                    <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                      <div style={{ fontWeight: 700 }}>{entry.title}</div>
                      <Tag color={confidenceColor(entry.confidence)}>{Math.round(entry.confidence)}%</Tag>
                    </div>
                    <div style={{ color: "var(--text-dim)", fontSize: 11 }}>{entry.repo}</div>
                    <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>{entry.detail}</div>
                    <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                      {entry.tags.map((tag) => (
                        <Tag key={`${entry.id}-${tag}`}>{tag}</Tag>
                      ))}
                    </div>
                  </div>
                ))
              ) : (
                <EmptyState icon="◎" text="Strong repo memories will show up here after the first successful ingest." />
              )}
            </div>
          </div>
        </>
      ) : (
        <EmptyState icon="?" text="RepoMemory overview data is not available yet." />
      )}
    </div>
  );
}
