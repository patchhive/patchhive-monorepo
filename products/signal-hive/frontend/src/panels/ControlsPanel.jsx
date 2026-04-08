import { useEffect, useMemo, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, Input, S, Sel, Tag } from "@patchhivehq/ui";

const LIST_OPTIONS = [
  { v: "allowlist", l: "Allowlist" },
  { v: "denylist", l: "Denylist" },
  { v: "opt_out", l: "Opt-Out" },
];

const SECTION_META = {
  allowlist: {
    label: "Allowlist",
    color: "var(--green)",
    empty: "No explicitly allowed repos.",
    note: "If any allowlist entries exist, SignalHive scans those repos directly even without a search query.",
  },
  denylist: {
    label: "Denylist",
    color: "var(--accent)",
    empty: "No explicitly denied repos.",
    note: "Denied repos are excluded even if your search would normally discover them.",
  },
  opt_out: {
    label: "Opt-Out",
    color: "var(--gold)",
    empty: "No opted-out repos.",
    note: "Opt-out is the strongest exclusion and should be respected across PatchHive products.",
  },
};

export default function ControlsPanel({ apiKey }) {
  const [repos, setRepos] = useState([]);
  const [repo, setRepo] = useState("");
  const [listType, setListType] = useState("allowlist");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const fetch_ = createApiFetcher(apiKey);

  const load = () =>
    fetch_(`${API}/repo-lists`)
      .then((res) => res.json())
      .then((data) => setRepos(data.repos || []))
      .catch(() => setRepos([]));

  useEffect(() => {
    load();
  }, [apiKey]);

  const grouped = useMemo(
    () => ({
      allowlist: repos.filter((item) => item.list_type === "allowlist"),
      denylist: repos.filter((item) => item.list_type === "denylist"),
      opt_out: repos.filter((item) => item.list_type === "opt_out"),
    }),
    [repos],
  );

  const add = async () => {
    if (!repo.trim()) {
      return;
    }
    setSaving(true);
    setError("");
    try {
      const res = await fetch_(`${API}/repo-lists`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ repo, list_type: listType }),
      });
      if (!res.ok) {
        throw new Error("SignalHive expects repo names in owner/repo format.");
      }
      setRepo("");
      await load();
    } catch (err) {
      setError(err.message || "Unable to save repo control.");
    } finally {
      setSaving(false);
    }
  };

  const remove = async (fullName) => {
    setSaving(true);
    setError("");
    try {
      const res = await fetch_(`${API}/repo-lists/${encodeURIComponent(fullName)}`, {
        method: "DELETE",
      });
      if (!res.ok) {
        throw new Error("Unable to remove repo control.");
      }
      await load();
    } catch (err) {
      setError(err.message || "Unable to remove repo control.");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div style={{ display: "grid", gap: 18 }}>
      <div style={{ ...S.panel, display: "grid", gap: 14 }}>
        <div style={{ display: "grid", gap: 6 }}>
          <div style={{ fontSize: 18, fontWeight: 700 }}>Discovery Controls</div>
          <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6, maxWidth: 920 }}>
            SignalHive should discover work without feeling invasive. These controls let you constrain
            which repos it is allowed to scan before anything gets ranked.
          </div>
        </div>

        <div style={{ display: "flex", gap: 12, flexWrap: "wrap", alignItems: "flex-end" }}>
          <div style={{ ...S.field, flex: "1 1 280px" }}>
            <div style={S.label}>Repo</div>
            <Input value={repo} onChange={setRepo} placeholder="owner/repo" />
          </div>
          <div style={{ ...S.field, minWidth: 180 }}>
            <div style={S.label}>List Type</div>
            <Sel value={listType} onChange={setListType} opts={LIST_OPTIONS} />
          </div>
          <Btn onClick={add} disabled={saving}>
            {saving ? "Saving…" : "Add Repo Control"}
          </Btn>
        </div>

        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <Tag color="var(--green)">Allowlist limits scanning to named repos</Tag>
          <Tag color="var(--accent)">Denylist excludes discovered repos</Tag>
          <Tag color="var(--gold)">Opt-Out overrides everything</Tag>
        </div>

        {error && (
          <div style={{ color: "var(--accent)", fontSize: 11 }}>
            {error}
          </div>
        )}
      </div>

      {Object.entries(SECTION_META).map(([key, section]) => {
        const items = grouped[key];
        return (
          <div key={key} style={{ ...S.panel, display: "grid", gap: 10 }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
              <div style={{ display: "grid", gap: 4 }}>
                <div style={{ fontSize: 15, fontWeight: 700, color: section.color }}>
                  {section.label}
                </div>
                <div style={{ color: "var(--text-dim)", fontSize: 11, lineHeight: 1.5 }}>
                  {section.note}
                </div>
              </div>
              <Tag color={section.color}>{items.length} repos</Tag>
            </div>

            {items.length === 0 ? (
              <EmptyState icon="◌" text={section.empty} />
            ) : (
              items.map((item) => (
                <div
                  key={`${item.list_type}-${item.repo}`}
                  style={{
                    border: "1px solid var(--border)",
                    borderRadius: 6,
                    padding: "10px 12px",
                    background: "var(--bg)",
                    display: "flex",
                    justifyContent: "space-between",
                    gap: 12,
                    alignItems: "center",
                  }}
                >
                  <div style={{ display: "grid", gap: 4 }}>
                    <div style={{ fontSize: 12, fontWeight: 700, color: "var(--text)" }}>{item.repo}</div>
                    <div style={{ fontSize: 10, color: "var(--text-muted)" }}>
                      Added {new Date(item.added_at).toLocaleString()}
                    </div>
                  </div>
                  <Btn onClick={() => remove(item.repo)} disabled={saving} color="var(--accent)">
                    Remove
                  </Btn>
                </div>
              ))
            )}
          </div>
        );
      })}
    </div>
  );
}
