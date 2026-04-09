import { useEffect, useMemo, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, Input, S, Tag } from "@patchhivehq/ui";

const DEFAULT_RULE_FORM = {
  repo: "",
  blocked_paths: ".github/workflows/, infra/, terraform/, migrations/, schema.sql",
  warn_paths: "auth/, permissions, billing, Dockerfile, docker-compose",
  require_test_for_paths: "src/, app/, lib/, server/, backend/",
  test_paths: "tests/, __tests__/, .test., .spec.",
  suspicious_terms: "TODO, FIXME, skip ci, eval(, exec(, unsafe, curl | sh, rm -rf, password, secret, token",
  blocked_terms: "BEGIN PRIVATE KEY, PRIVATE KEY-----, ghp_, github_pat_, sk-, AKIA",
  max_files: "12",
  max_additions: "400",
  max_deletions: "250",
  notes: "",
};

function splitList(value) {
  return value
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean);
}

function joinList(value = []) {
  return value.join(", ");
}

function toPayload(form) {
  return {
    repo: form.repo.trim(),
    blocked_paths: splitList(form.blocked_paths),
    warn_paths: splitList(form.warn_paths),
    require_test_for_paths: splitList(form.require_test_for_paths),
    test_paths: splitList(form.test_paths),
    suspicious_terms: splitList(form.suspicious_terms),
    blocked_terms: splitList(form.blocked_terms),
    max_files: Number(form.max_files) || 12,
    max_additions: Number(form.max_additions) || 400,
    max_deletions: Number(form.max_deletions) || 250,
    notes: form.notes.trim(),
  };
}

function fromRules(ruleSet) {
  return {
    repo: ruleSet.repo || "",
    blocked_paths: joinList(ruleSet.blocked_paths),
    warn_paths: joinList(ruleSet.warn_paths),
    require_test_for_paths: joinList(ruleSet.require_test_for_paths),
    test_paths: joinList(ruleSet.test_paths),
    suspicious_terms: joinList(ruleSet.suspicious_terms),
    blocked_terms: joinList(ruleSet.blocked_terms),
    max_files: String(ruleSet.max_files ?? 12),
    max_additions: String(ruleSet.max_additions ?? 400),
    max_deletions: String(ruleSet.max_deletions ?? 250),
    notes: ruleSet.notes || "",
  };
}

export default function RulesPanel({ apiKey, initialRepo }) {
  const [form, setForm] = useState(DEFAULT_RULE_FORM);
  const [rules, setRules] = useState([]);
  const [packs, setPacks] = useState([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const fetch_ = createApiFetcher(apiKey);

  const set = (key, value) => setForm((prev) => ({ ...prev, [key]: value }));

  const refresh = async () => {
    try {
      const [rulesRes, packsRes] = await Promise.all([
        fetch_(`${API}/rules`).then((res) => res.json()),
        fetch_(`${API}/rule-packs`).then((res) => res.json()),
      ]);
      setRules(rulesRes.rules || []);
      setPacks(packsRes.packs || []);
    } catch {
      setRules([]);
      setPacks([]);
    }
  };

  useEffect(() => {
    refresh();
  }, [apiKey]);

  useEffect(() => {
    if (initialRepo && !form.repo) {
      set("repo", initialRepo);
    }
  }, [initialRepo]);

  const savedRepos = useMemo(() => rules.map((item) => item.repo), [rules]);

  const applyPack = (pack) => {
    const next = fromRules(pack.rules || {});
    next.repo = form.repo;
    setForm(next);
  };

  const save = async () => {
    if (!form.repo.trim()) {
      setError("TrustGate needs an owner/repo before it can save rules.");
      return;
    }

    setBusy(true);
    setError("");
    try {
      const res = await fetch_(`${API}/rules`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(toPayload(form)),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "TrustGate could not save these rules.");
      }
      await refresh();
      set("repo", data.repo || form.repo.trim());
    } catch (err) {
      setError(err.message || "TrustGate could not save these rules.");
    } finally {
      setBusy(false);
    }
  };

  const remove = async (repo) => {
    setBusy(true);
    setError("");
    try {
      const res = await fetch_(`${API}/rules/${encodeURIComponent(repo)}`, {
        method: "DELETE",
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "TrustGate could not delete this rule set.");
      }
      if (form.repo === repo) {
        setForm(DEFAULT_RULE_FORM);
      }
      await refresh();
    } catch (err) {
      setError(err.message || "TrustGate could not delete this rule set.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div style={{ display: "grid", gridTemplateColumns: "1.2fr 0.8fr", gap: 18 }}>
      <div style={{ ...S.panel, display: "grid", gap: 14 }}>
        <div>
          <div style={{ fontSize: 18, fontWeight: 700 }}>Repo Rule Set</div>
          <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>
            Repo-specific rules are TrustGate&apos;s core memory. They define which file paths are blocked,
            which paths are merely sensitive, what counts as suspicious content, and how large a diff is
            allowed to grow before it should be escalated.
          </div>
        </div>

        {packs.length > 0 && (
          <div style={{ display: "grid", gap: 10 }}>
            <div style={{ fontSize: 14, fontWeight: 700 }}>Starter Rule Packs</div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: 10 }}>
              {packs.map((pack) => (
                <div
                  key={pack.id}
                  style={{
                    border: "1px solid var(--border)",
                    borderRadius: 8,
                    padding: 12,
                    display: "grid",
                    gap: 8,
                  }}
                >
                  <div style={{ display: "flex", justifyContent: "space-between", gap: 8, flexWrap: "wrap" }}>
                    <div style={{ fontWeight: 700 }}>{pack.label}</div>
                    <Tag color="var(--blue)">{pack.id}</Tag>
                  </div>
                  <div style={{ color: "var(--text-dim)", fontSize: 11, lineHeight: 1.5 }}>{pack.description}</div>
                  <Btn onClick={() => applyPack(pack)} color="var(--blue)">
                    Apply Pack
                  </Btn>
                </div>
              ))}
            </div>
          </div>
        )}

        {error && (
          <div
            style={{
              border: "1px solid var(--accent)44",
              background: "var(--accent)10",
              color: "var(--accent)",
              borderRadius: 8,
              padding: "10px 12px",
            }}
          >
            {error}
          </div>
        )}

        <div style={S.field}>
          <div style={S.label}>Repo</div>
          <Input value={form.repo} onChange={(value) => set("repo", value)} placeholder="owner/repo" />
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Blocked Paths</div>
            <textarea
              value={form.blocked_paths}
              onChange={(event) => set("blocked_paths", event.target.value)}
              style={{ ...S.input, minHeight: 90, resize: "vertical" }}
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Sensitive Paths</div>
            <textarea
              value={form.warn_paths}
              onChange={(event) => set("warn_paths", event.target.value)}
              style={{ ...S.input, minHeight: 90, resize: "vertical" }}
            />
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Require Tests For Paths</div>
            <textarea
              value={form.require_test_for_paths}
              onChange={(event) => set("require_test_for_paths", event.target.value)}
              style={{ ...S.input, minHeight: 90, resize: "vertical" }}
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Test Path Markers</div>
            <textarea
              value={form.test_paths}
              onChange={(event) => set("test_paths", event.target.value)}
              style={{ ...S.input, minHeight: 90, resize: "vertical" }}
            />
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Suspicious Terms</div>
            <textarea
              value={form.suspicious_terms}
              onChange={(event) => set("suspicious_terms", event.target.value)}
              style={{ ...S.input, minHeight: 90, resize: "vertical" }}
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Blocked Terms</div>
            <textarea
              value={form.blocked_terms}
              onChange={(event) => set("blocked_terms", event.target.value)}
              style={{ ...S.input, minHeight: 90, resize: "vertical" }}
            />
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Max Files</div>
            <Input value={form.max_files} onChange={(value) => set("max_files", value)} type="number" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Max Additions</div>
            <Input
              value={form.max_additions}
              onChange={(value) => set("max_additions", value)}
              type="number"
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Max Deletions</div>
            <Input
              value={form.max_deletions}
              onChange={(value) => set("max_deletions", value)}
              type="number"
            />
          </div>
        </div>

        <div style={S.field}>
          <div style={S.label}>Notes</div>
          <textarea
            value={form.notes}
            onChange={(event) => set("notes", event.target.value)}
            style={{ ...S.input, minHeight: 90, resize: "vertical" }}
            placeholder="Why these rules exist, what the repo cares about, or which changes need a human in the loop."
          />
        </div>

        <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
          <Btn onClick={save} disabled={busy}>
            {busy ? "Saving..." : "Save Rule Set"}
          </Btn>
          <Btn onClick={() => setForm({ ...DEFAULT_RULE_FORM, repo: form.repo })} color="var(--text-dim)">
            Reset To Defaults
          </Btn>
        </div>
      </div>

      <div style={{ ...S.panel, display: "grid", gap: 12, alignContent: "start" }}>
        <div>
          <div style={{ fontSize: 18, fontWeight: 700 }}>Saved Rule Sets</div>
          <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
            These get picked up automatically when TrustGate reviews a diff for the same repo.
          </div>
        </div>

        {rules.length === 0 ? (
          <EmptyState icon="◌" text="Save a repo rule set and it will show up here." />
        ) : (
          rules.map((item) => (
            <div
              key={item.repo}
              style={{
                border: "1px solid var(--border)",
                borderRadius: 8,
                padding: 12,
                display: "grid",
                gap: 10,
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                <div style={{ fontWeight: 700 }}>{item.repo}</div>
                {savedRepos.includes(item.repo) && <Tag color="var(--green)">saved</Tag>}
              </div>
              <div style={{ color: "var(--text-dim)", fontSize: 11, lineHeight: 1.5 }}>
                {item.rules.notes || "No notes yet. This repo will use the saved thresholds and path rules above."}
              </div>
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                <Tag color="var(--accent)">{item.rules.blocked_paths.length} blocked paths</Tag>
                <Tag color="var(--gold)">{item.rules.warn_paths.length} sensitive paths</Tag>
                <Tag color="var(--blue)">{item.rules.suspicious_terms.length} suspicious terms</Tag>
              </div>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                <Btn onClick={() => setForm(fromRules(item.rules))} color="var(--blue)">
                  Load
                </Btn>
                <Btn onClick={() => remove(item.repo)} color="var(--accent)" disabled={busy}>
                  Delete
                </Btn>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
