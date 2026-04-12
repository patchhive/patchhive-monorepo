import { useCallback, useEffect, useMemo, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, Input, S, Sel, Tag, timeAgo } from "@patchhivehq/ui";

function confidenceColor(confidence) {
  if (confidence >= 80) return "var(--green)";
  if (confidence >= 60) return "var(--gold)";
  return "var(--accent)";
}

function parsePaths(text) {
  return text
    .split("\n")
    .map((value) => value.trim())
    .filter(Boolean);
}

export default function MemoryPanel({ apiKey, activeRepo, setActiveRepo, currentRun }) {
  const [repos, setRepos] = useState([]);
  const [memories, setMemories] = useState([]);
  const [kind, setKind] = useState("");
  const [search, setSearch] = useState("");
  const [savingRef, setSavingRef] = useState("");
  const [previewConsumer, setPreviewConsumer] = useState("repo-reaper");
  const [previewPaths, setPreviewPaths] = useState("");
  const [previewTask, setPreviewTask] = useState("");
  const [previewDiff, setPreviewDiff] = useState("");
  const [previewLoading, setPreviewLoading] = useState(false);
  const [previewError, setPreviewError] = useState("");
  const [preview, setPreview] = useState(null);
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

  const loadMemories = useCallback(() => {
    const params = new URLSearchParams();
    if (activeRepo) params.set("repo", activeRepo);
    if (kind) params.set("kind", kind);
    if (search.trim()) params.set("search", search.trim());
    return fetch_(`${API}/memories?${params.toString()}`)
      .then((res) => res.json())
      .then((data) => setMemories(data.memories || []))
      .catch(() => setMemories([]));
  }, [activeRepo, apiKey, kind, search]);

  useEffect(() => {
    loadMemories();
  }, [loadMemories]);

  const runContextPreview = useCallback(async () => {
    if (!activeRepo) return;
    setPreviewLoading(true);
    setPreviewError("");
    try {
      const res = await fetch_(`${API}/context`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          repo: activeRepo,
          consumer: previewConsumer,
          changed_paths: parsePaths(previewPaths),
          task_summary: previewTask.trim(),
          diff_summary: previewDiff.trim(),
          limit: 6,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "RepoMemory could not build that context preview.");
      }
      setPreview(data);
    } catch (err) {
      setPreviewError(err.message || "RepoMemory could not build that context preview.");
      setPreview(null);
    } finally {
      setPreviewLoading(false);
    }
  }, [activeRepo, fetch_, previewConsumer, previewDiff, previewPaths, previewTask]);

  const saveCuration = useCallback(
    async (entry, next) => {
      setSavingRef(entry.memory_ref);
      try {
        const res = await fetch_(`${API}/memories/curation`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            repo: entry.repo,
            memory_ref: entry.memory_ref,
            disposition: next.disposition ?? entry.disposition ?? "signal",
            pinned: next.pinned ?? !!entry.pinned,
          }),
        });
        const data = await res.json();
        if (!res.ok) {
          throw new Error(data.error || "RepoMemory could not save that curation.");
        }
        await loadMemories();
        if (preview?.repo === entry.repo) {
          await runContextPreview();
        }
      } finally {
        setSavingRef("");
      }
    },
    [fetch_, loadMemories, preview, runContextPreview]
  );

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

      <div style={{ ...S.panel, display: "grid", gap: 12 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
          <div>
            <div style={{ fontSize: 16, fontWeight: 700 }}>Context preview</div>
            <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>
              Preview the exact retrieval shape that RepoReaper or TrustGate would receive before you trust the memory layer.
            </div>
          </div>
          <Btn onClick={runContextPreview} disabled={!activeRepo || previewLoading}>
            {previewLoading ? "Loading..." : "Preview context"}
          </Btn>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1.4fr", gap: 12 }}>
          <div style={{ display: "grid", gap: 12 }}>
            <div style={S.field}>
              <div style={S.label}>Consumer</div>
              <Sel
                value={previewConsumer}
                onChange={setPreviewConsumer}
                opts={[
                  { v: "repo-reaper", l: "RepoReaper" },
                  { v: "trust-gate", l: "TrustGate" },
                  { v: "", l: "Generic" },
                ]}
              />
            </div>
            <div style={S.field}>
              <div style={S.label}>Task summary</div>
              <Input value={previewTask} onChange={setPreviewTask} placeholder="Fix flaky background job timeout regression" />
            </div>
            <div style={S.field}>
              <div style={S.label}>Changed paths</div>
              <textarea
                value={previewPaths}
                onChange={(event) => setPreviewPaths(event.target.value)}
                placeholder={"src/jobs/runner.rs\ntests/runner.spec.js"}
                style={{ ...S.input, minHeight: 110, resize: "vertical" }}
              />
            </div>
          </div>

          <div style={{ display: "grid", gap: 12 }}>
            <div style={S.field}>
              <div style={S.label}>Diff summary</div>
              <textarea
                value={previewDiff}
                onChange={(event) => setPreviewDiff(event.target.value)}
                placeholder="Retry path updates queue timing and changes timeout handling in the worker loop."
                style={{ ...S.input, minHeight: 142, resize: "vertical" }}
              />
            </div>
            {previewError && <div style={{ color: "var(--accent)" }}>{previewError}</div>}
            {preview ? (
              <div style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 12, display: "grid", gap: 10 }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
                  <div style={{ fontWeight: 700 }}>{preview.consumer || "generic"} context</div>
                  <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                    <Tag color="var(--blue)">{preview.entries?.length || 0} entries</Tag>
                    <Tag color="var(--green)">{preview.repo}</Tag>
                  </div>
                </div>
                <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>{preview.summary}</div>
                {(preview.prompt_lines || []).length ? (
                  <div style={{ display: "grid", gap: 6 }}>
                    {(preview.entries || []).map((entry) => (
                      <div key={`preview-${entry.memory_ref}`} style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 10, display: "grid", gap: 6 }}>
                        <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                          <div style={{ fontWeight: 700 }}>{entry.title}</div>
                          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                            {entry.pinned && <Tag color="var(--green)">pinned</Tag>}
                            {entry.disposition === "policy" && <Tag color="var(--accent)">policy</Tag>}
                            <Tag color="var(--blue)">{entry.kind}</Tag>
                            <Tag color={confidenceColor(entry.confidence)}>{Math.round(entry.confidence)}%</Tag>
                          </div>
                        </div>
                        <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>{entry.prompt_line}</div>
                        {!!entry.matched_paths?.length && (
                          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                            {entry.matched_paths.map((path) => (
                              <Tag key={`${entry.memory_ref}-${path}`} color="var(--gold)">
                                {path}
                              </Tag>
                            ))}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                ) : (
                  <EmptyState icon="◎" text="RepoMemory did not find any especially relevant context for that preview." />
                )}
              </div>
            ) : (
              <EmptyState icon="🧠" text="Run a context preview to see what RepoReaper or TrustGate would retrieve." />
            )}
          </div>
        </div>
      </div>

      {memories.length ? (
        memories.map((entry) => (
          <div key={entry.id} style={{ ...S.panel, display: "grid", gap: 10, opacity: entry.disposition === "suppressed" ? 0.75 : 1 }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 10, alignItems: "flex-start", flexWrap: "wrap" }}>
              <div style={{ display: "grid", gap: 6 }}>
                <div style={{ fontSize: 16, fontWeight: 700 }}>{entry.title}</div>
                <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>{entry.detail}</div>
              </div>
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap", justifyContent: "flex-end" }}>
                {entry.pinned && <Tag color="var(--green)">pinned</Tag>}
                {entry.disposition === "policy" && <Tag color="var(--accent)">policy</Tag>}
                {entry.disposition === "suppressed" && <Tag color="var(--gold)">suppressed</Tag>}
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

            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <Btn
                onClick={() => saveCuration(entry, { pinned: !entry.pinned })}
                disabled={savingRef === entry.memory_ref}
                style={{ padding: "4px 10px" }}
              >
                {entry.pinned ? "Unpin" : "Pin"}
              </Btn>
              <Btn
                onClick={() =>
                  saveCuration(entry, {
                    disposition: entry.disposition === "policy" ? "signal" : "policy",
                  })
                }
                disabled={savingRef === entry.memory_ref}
                style={{ padding: "4px 10px" }}
              >
                {entry.disposition === "policy" ? "Return To Signal" : "Promote To Policy"}
              </Btn>
              <Btn
                onClick={() =>
                  saveCuration(entry, {
                    disposition: entry.disposition === "suppressed" ? "signal" : "suppressed",
                    pinned: entry.disposition === "suppressed" ? entry.pinned : false,
                  })
                }
                disabled={savingRef === entry.memory_ref}
                style={{ padding: "4px 10px" }}
              >
                {entry.disposition === "suppressed" ? "Unsuppress" : "Suppress"}
              </Btn>
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
