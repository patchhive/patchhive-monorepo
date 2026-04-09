import { useEffect, useMemo, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, Input, S, Sel, Tag, timeAgo } from "@patchhivehq/ui";

function confidenceColor(confidence) {
  if (confidence >= 80) return "var(--green)";
  if (confidence >= 60) return "var(--gold)";
  return "var(--accent)";
}

export default function MemoryPanel({ apiKey, activeRepo, setActiveRepo, currentRun }) {
  const [repos, setRepos] = useState([]);
  const [memories, setMemories] = useState([]);
  const [kind, setKind] = useState("");
  const [search, setSearch] = useState("");
  const fetch_ = createApiFetcher(apiKey);

  useEffect(() => {
    fetch_(`${API}/repos`)
      .then((res) => res.json())
      .then((data) => setRepos(data.repos || []))
      .catch(() => setRepos([]));
  }, [apiKey]);

  useEffect(() => {
    if (!activeRepo && repos[0]?.repo) {
      setActiveRepo(repos[0].repo);
    }
  }, [repos, activeRepo, setActiveRepo]);

  useEffect(() => {
    const params = new URLSearchParams();
    if (activeRepo) params.set("repo", activeRepo);
    if (kind) params.set("kind", kind);
    if (search.trim()) params.set("search", search.trim());
    fetch_(`${API}/memories?${params.toString()}`)
      .then((res) => res.json())
      .then((data) => setMemories(data.memories || []))
      .catch(() => setMemories([]));
  }, [apiKey, activeRepo, kind, search]);

  const repoOpts = useMemo(
    () =>
      repos.length
        ? repos.map((repo) => ({ v: repo.repo, l: repo.repo }))
        : [{ v: "", l: "No tracked repos yet" }],
    [repos]
  );

  return (
    <div style={{ display: "grid", gap: 18 }}>
      <div style={{ ...S.panel, display: "grid", gap: 12 }}>
        <div style={{ fontSize: 18, fontWeight: 700 }}>Searchable repo memory</div>
        <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>
          RepoMemory keeps the strongest extracted conventions, failure patterns, and hotspots searchable so humans and agents can reuse what the repo has already learned.
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "1.4fr 1fr 1.2fr", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Repo</div>
            <Sel value={activeRepo} onChange={setActiveRepo} opts={repoOpts} />
          </div>
          <div style={S.field}>
            <div style={S.label}>Kind</div>
            <Sel
              value={kind}
              onChange={setKind}
              opts={[
                { v: "", l: "All memory" },
                { v: "review_rule", l: "Review rules" },
                { v: "testing_expectation", l: "Testing expectations" },
                { v: "hotspot", l: "Hotspots" },
                { v: "failure_pattern", l: "Failure patterns" },
              ]}
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Search</div>
            <Input value={search} onChange={setSearch} placeholder="tests, helper, timeout..." />
          </div>
        </div>
        {currentRun?.repo === activeRepo && (
          <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
            <Tag color="var(--accent)">current run</Tag>
            <div style={{ color: "var(--text-dim)" }}>
              {currentRun.summary.memories_created} memories from the latest ingest for {currentRun.repo}
            </div>
          </div>
        )}
      </div>

      {memories.length ? (
        memories.map((entry) => (
          <div key={entry.id} style={{ ...S.panel, display: "grid", gap: 10 }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 10, alignItems: "flex-start", flexWrap: "wrap" }}>
              <div style={{ display: "grid", gap: 6 }}>
                <div style={{ fontSize: 16, fontWeight: 700 }}>{entry.title}</div>
                <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>{entry.detail}</div>
              </div>
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap", justifyContent: "flex-end" }}>
                <Tag color={confidenceColor(entry.confidence)}>{Math.round(entry.confidence)}%</Tag>
                <Tag color="var(--blue)">{entry.kind}</Tag>
                <Tag color="var(--gold)">{entry.frequency} hits</Tag>
              </div>
            </div>

            <div style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 10, background: "var(--bg-input)" }}>
              <div style={S.label}>Agent prompt line</div>
              <div style={{ marginTop: 6, color: "var(--text)", lineHeight: 1.6 }}>{entry.prompt_line}</div>
            </div>

            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {entry.tags.map((tag) => (
                <Tag key={`${entry.id}-${tag}`}>{tag}</Tag>
              ))}
            </div>

            {entry.evidence?.length ? (
              <div style={{ display: "grid", gap: 8 }}>
                <div style={S.label}>Evidence</div>
                {entry.evidence.map((evidence, index) => (
                  <div key={`${entry.id}-evidence-${index}`} style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 10, display: "grid", gap: 6 }}>
                    <div style={{ display: "flex", justifyContent: "space-between", gap: 8, flexWrap: "wrap" }}>
                      <a href={evidence.url} target="_blank" rel="noreferrer" style={{ color: "var(--accent)", textDecoration: "none", fontWeight: 700 }}>
                        {evidence.title}
                      </a>
                      {evidence.path && <Tag color="var(--blue)">{evidence.path}</Tag>}
                    </div>
                    <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>{evidence.excerpt}</div>
                  </div>
                ))}
              </div>
            ) : (
              <div style={{ color: "var(--text-dim)" }}>
                This memory came from repeated path or PR structure patterns rather than one quoted comment.
              </div>
            )}

            <div style={{ color: "var(--text-dim)", fontSize: 11 }}>
              Captured {timeAgo(entry.created_at)}
            </div>
          </div>
        ))
      ) : (
        <EmptyState icon="◎" text="No memories match this repo/filter combination yet." />
      )}
    </div>
  );
}
