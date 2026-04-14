import { useEffect, useState } from "react";
import { useApiFetcher } from "@patchhivehq/product-shell";
import { S, Input, Btn } from "@patchhivehq/ui";
import { API } from "../config.js";

const SECTIONS = [
  { key: "allowlist", label: "Allowlist", color: "#2a8a4a", empty: "No explicitly allowed repos." },
  { key: "denylist", label: "Denylist", color: "#c41e3a", empty: "No explicitly denied repos." },
  { key: "opt_out", label: "Opt-Out", color: "#c8922a", empty: "No opted-out repos." },
];

export function RepoListsPanel({ apiKey = "" }) {
  const [repos, setRepos] = useState([]);
  const [input, setInput] = useState("");
  const [type, setType] = useState("allowlist");
  const fetch_ = useApiFetcher(apiKey);

  const load = () => fetch_(`${API}/repo-lists`).then(r => r.json()).then(d => setRepos(d.repos || []));

  useEffect(load, [fetch_]);

  const add = async () => {
    if (!input.trim()) return;
    await fetch_(`${API}/repo-lists`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ repo: input.trim(), list_type: type }),
    });
    setInput("");
    load();
  };

  const remove = async repo => {
    await fetch_(`${API}/repo-lists/${encodeURIComponent(repo)}`, { method: "DELETE" });
    load();
  };

  const grouped = {
    allowlist: repos.filter(r => r.list_type === "allowlist"),
    denylist: repos.filter(r => r.list_type === "denylist" || r.list_type === "blocklist"),
    opt_out: repos.filter(r => r.list_type === "opt_out"),
  };

  return (
    <div>
      <div style={{ fontSize: 13, fontWeight: 700, color: "#d4d4e8", marginBottom: 16 }}>◎ Repo Lists</div>
      <div style={{ ...S.panel, marginBottom: 16 }}>
        <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
          <Input value={input} onChange={setInput} placeholder="owner/repo" style={{ flex: 1 }} />
          <select value={type} onChange={e => setType(e.target.value)} style={{ ...S.select, width: "auto" }}>
            <option value="allowlist">Allowlist</option>
            <option value="denylist">Denylist</option>
            <option value="opt_out">Opt-Out</option>
          </select>
          <Btn onClick={add} color="#c41e3a">Add</Btn>
        </div>
        <div style={{ fontSize: 10, color: "#484868" }}>
          Allowlist: hunt only these repos. Denylist: never hunt these repos. Opt-Out: strongest exclusion and should be respected across PatchHive.
        </div>
      </div>

      {SECTIONS.map(section => {
        const list = grouped[section.key];
        return (
          <div key={section.key} style={{ marginBottom: 16 }}>
            <div style={{ fontSize: 11, color: section.color, fontWeight: 700, marginBottom: 8 }}>
              {section.label} ({list.length})
            </div>
            {list.length === 0 ? (
              <div style={{ fontSize: 10, color: "#484868" }}>{section.empty}</div>
            ) : (
              list.map(r => (
                <div key={r.repo} style={{ display: "flex", alignItems: "center", gap: 8, padding: "4px 0" }}>
                  <span style={{ fontSize: 11, color: "#d4d4e8", flex: 1 }}>{r.repo}</span>
                  <button
                    onClick={() => remove(r.repo)}
                    style={{ background: "transparent", border: "none", cursor: "pointer", color: "#484868", fontSize: 14 }}
                  >
                    ×
                  </button>
                </div>
              ))
            )}
          </div>
        );
      })}
    </div>
  );
}
