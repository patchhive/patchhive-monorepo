import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, Input, S, Tag, timeAgo } from "@patchhivehq/ui";

async function copyText(text) {
  if (!navigator?.clipboard?.writeText) return false;
  await navigator.clipboard.writeText(text);
  return true;
}

function DiffGroup({ label, color, items, empty }) {
  return (
    <div style={{ display: "grid", gap: 8 }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
        <div style={{ fontWeight: 700 }}>{label}</div>
        <Tag color={color}>{items.length}</Tag>
      </div>
      {items.length ? (
        items.map((item) => (
          <div key={`${label}-${item.memory_ref}`} style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 10, display: "grid", gap: 6 }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "flex-start", flexWrap: "wrap" }}>
              <div>
                <div style={{ fontWeight: 700 }}>{item.title}</div>
                <div style={{ color: "var(--text-dim)", fontSize: 11 }}>{item.kind}</div>
              </div>
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                {item.current_confidence != null && <Tag color="var(--green)">{Math.round(item.current_confidence)}%</Tag>}
                {item.previous_confidence != null && <Tag color="var(--gold)">was {Math.round(item.previous_confidence)}%</Tag>}
              </div>
            </div>
            <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>{item.prompt_line}</div>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              <Tag color="var(--blue)">
                Δ confidence {item.delta_confidence >= 0 ? "+" : ""}{Math.round(item.delta_confidence)}
              </Tag>
              <Tag color="var(--accent)">
                Δ hits {item.delta_frequency >= 0 ? "+" : ""}{item.delta_frequency}
              </Tag>
            </div>
          </div>
        ))
      ) : (
        <div style={{ color: "var(--text-dim)" }}>{empty}</div>
      )}
    </div>
  );
}

export default function HistoryPanel({ apiKey, activeRepo, onLoadRun }) {
  const [repoFilter, setRepoFilter] = useState(activeRepo || "");
  const [history, setHistory] = useState([]);
  const [selectedId, setSelectedId] = useState("");
  const [detailMode, setDetailMode] = useState("");
  const [promptPack, setPromptPack] = useState("");
  const [runDiff, setRunDiff] = useState(null);
  const [error, setError] = useState("");
  const fetch_ = createApiFetcher(apiKey);

  const refresh = () => {
    const params = new URLSearchParams();
    if (repoFilter.trim()) params.set("repo", repoFilter.trim());
    fetch_(`${API}/history?${params.toString()}`)
      .then((res) => res.json())
      .then((data) => setHistory(data.history || []))
      .catch(() => setHistory([]));
  };

  useEffect(() => {
    setRepoFilter(activeRepo || "");
  }, [activeRepo]);

  useEffect(() => {
    refresh();
  }, [apiKey, repoFilter]);

  const selectRun = (id, mode) => {
    setError("");
    if (selectedId !== id) {
      setPromptPack("");
      setRunDiff(null);
    }
    setSelectedId(id);
    setDetailMode(mode);
  };

  const loadPromptPack = async (id) => {
    selectRun(id, "prompt");
    try {
      const res = await fetch_(`${API}/history/${id}/prompt-pack`);
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "RepoMemory could not load that prompt pack.");
      }
      setPromptPack(data.prompt_pack || "");
    } catch (err) {
      setError(err.message || "RepoMemory could not load that prompt pack.");
    }
  };

  const loadRunDiff = async (id) => {
    selectRun(id, "diff");
    try {
      const res = await fetch_(`${API}/history/${id}/diff`);
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "RepoMemory could not load that run diff.");
      }
      setRunDiff(data);
    } catch (err) {
      setError(err.message || "RepoMemory could not load that run diff.");
    }
  };

  return (
    <div style={{ display: "grid", gap: 18 }}>
      <div style={{ ...S.panel, display: "grid", gap: 12 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
          <div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>Memory runs</div>
            <div style={{ color: "var(--text-dim)" }}>
              RepoMemory keeps full ingest runs so you can reopen a repo memory snapshot, compare it with the prior one, and reuse the prompt pack later.
            </div>
          </div>
          <Btn onClick={refresh}>Refresh</Btn>
        </div>
        <div style={{ maxWidth: 320 }}>
          <div style={S.label}>Repo filter</div>
          <Input value={repoFilter} onChange={setRepoFilter} placeholder="owner/repo or blank for all" />
        </div>
        {error && <div style={{ color: "var(--accent)" }}>{error}</div>}
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1.1fr 1fr", gap: 16 }}>
        <div style={{ ...S.panel, display: "grid", gap: 10 }}>
          {history.length ? (
            history.map((item) => (
              <div key={item.id} style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 12, display: "grid", gap: 8 }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "flex-start", flexWrap: "wrap" }}>
                  <div>
                    <div style={{ fontWeight: 700 }}>{item.repo}</div>
                    <div style={{ color: "var(--text-dim)", fontSize: 11 }}>{timeAgo(item.created_at)}</div>
                  </div>
                  <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                    <Tag color="var(--green)">{item.conventions} conventions</Tag>
                    <Tag color="var(--accent)">{item.failures} failures</Tag>
                    <Tag color="var(--blue)">{item.hotspots} hotspots</Tag>
                  </div>
                </div>
                <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>{item.top_memory}</div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  <Btn onClick={() => onLoadRun(item.id)} style={{ padding: "4px 10px" }}>
                    Load run
                  </Btn>
                  <Btn onClick={() => loadRunDiff(item.id)} style={{ padding: "4px 10px" }}>
                    Changes
                  </Btn>
                  <Btn onClick={() => loadPromptPack(item.id)} style={{ padding: "4px 10px" }}>
                    Prompt pack
                  </Btn>
                </div>
              </div>
            ))
          ) : (
            <EmptyState icon="◎" text="No RepoMemory runs match this filter yet." />
          )}
        </div>

        <div style={{ ...S.panel, display: "grid", gap: 10 }}>
          <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
            <div style={{ fontSize: 16, fontWeight: 700 }}>
              {detailMode === "diff" ? "Run changes" : "Prompt pack snapshot"}
            </div>
            {detailMode === "prompt" && promptPack && (
              <Btn onClick={() => copyText(promptPack)} style={{ padding: "4px 10px" }}>
                Copy
              </Btn>
            )}
          </div>

          {selectedId ? (
            <div style={{ color: "var(--text-dim)", fontSize: 11 }}>Run {selectedId.slice(0, 8)}</div>
          ) : (
            <div style={{ color: "var(--text-dim)" }}>
              Pick a run to inspect its prompt pack or the memory changes since the previous ingest.
            </div>
          )}

          {detailMode === "diff" ? (
            runDiff ? (
              <div style={{ display: "grid", gap: 12 }}>
                <div style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 12, display: "grid", gap: 8 }}>
                  <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                    <Tag color="var(--green)">{runDiff.counts?.new_entries ?? 0} new</Tag>
                    <Tag color="var(--blue)">{runDiff.counts?.strengthened_entries ?? 0} stronger</Tag>
                    <Tag color="var(--gold)">{runDiff.counts?.faded_entries ?? 0} faded</Tag>
                    <Tag color="var(--accent)">{runDiff.counts?.retired_entries ?? 0} retired</Tag>
                  </div>
                  <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>{runDiff.summary}</div>
                  {runDiff.previous_created_at && (
                    <div style={{ color: "var(--text-dim)", fontSize: 11 }}>
                      Comparing {timeAgo(runDiff.created_at)} against {timeAgo(runDiff.previous_created_at)}
                    </div>
                  )}
                </div>

                <DiffGroup
                  label="New memories"
                  color="var(--green)"
                  items={runDiff.new_entries || []}
                  empty="No new durable memories appeared in this run."
                />
                <DiffGroup
                  label="Strengthened memories"
                  color="var(--blue)"
                  items={runDiff.strengthened_entries || []}
                  empty="No memories grew materially stronger in this run."
                />
                <DiffGroup
                  label="Faded memories"
                  color="var(--gold)"
                  items={runDiff.faded_entries || []}
                  empty="No memories weakened materially in this run."
                />
                <DiffGroup
                  label="Retired memories"
                  color="var(--accent)"
                  items={runDiff.retired_entries || []}
                  empty="No prior memories fell out of the latest snapshot."
                />
              </div>
            ) : (
              <EmptyState icon="⇆" text="Run changes will appear here after you load a history diff." />
            )
          ) : promptPack ? (
            <pre style={{
              margin: 0,
              whiteSpace: "pre-wrap",
              lineHeight: 1.6,
              color: "var(--text-dim)",
              background: "var(--bg-input)",
              border: "1px solid var(--border-in)",
              borderRadius: 8,
              padding: 12,
              maxHeight: 520,
              overflow: "auto",
            }}>
              {promptPack}
            </pre>
          ) : (
            <EmptyState icon="🧠" text="Prompt-pack output will appear here after you load a run." />
          )}
        </div>
      </div>
    </div>
  );
}
