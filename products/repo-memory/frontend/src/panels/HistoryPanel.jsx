import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, Input, S, Tag, timeAgo } from "@patchhivehq/ui";

async function copyText(text) {
  if (!navigator?.clipboard?.writeText) return false;
  await navigator.clipboard.writeText(text);
  return true;
}

export default function HistoryPanel({ apiKey, activeRepo, onLoadRun }) {
  const [repoFilter, setRepoFilter] = useState(activeRepo || "");
  const [history, setHistory] = useState([]);
  const [selectedId, setSelectedId] = useState("");
  const [promptPack, setPromptPack] = useState("");
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

  const loadPromptPack = async (id) => {
    setError("");
    try {
      const res = await fetch_(`${API}/history/${id}/prompt-pack`);
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "RepoMemory could not load that prompt pack.");
      }
      setSelectedId(id);
      setPromptPack(data.prompt_pack || "");
    } catch (err) {
      setError(err.message || "RepoMemory could not load that prompt pack.");
    }
  };

  return (
    <div style={{ display: "grid", gap: 18 }}>
      <div style={{ ...S.panel, display: "grid", gap: 12 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
          <div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>Memory runs</div>
            <div style={{ color: "var(--text-dim)" }}>
              RepoMemory keeps full ingest runs so you can reopen a repo memory snapshot and reuse the prompt pack later.
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
            <div style={{ fontSize: 16, fontWeight: 700 }}>Prompt pack snapshot</div>
            {promptPack && (
              <Btn onClick={() => copyText(promptPack)} style={{ padding: "4px 10px" }}>
                Copy
              </Btn>
            )}
          </div>
          {selectedId ? (
            <div style={{ color: "var(--text-dim)", fontSize: 11 }}>Run {selectedId.slice(0, 8)}</div>
          ) : (
            <div style={{ color: "var(--text-dim)" }}>Pick a run to load its generated prompt pack.</div>
          )}
          {promptPack ? (
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
