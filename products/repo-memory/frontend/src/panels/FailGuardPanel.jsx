import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, Input, S, Tag } from "@patchhivehq/ui";

const DEFAULT_FORM = {
  repo: "",
  title: "",
  outcome: "",
  lesson: "",
  prevention: "",
  affected_paths: "",
  evidence: "",
  disposition: "policy",
  pinned: true,
};

function lines(text) {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
}

export default function FailGuardPanel({
  apiKey,
  activeRepo,
  setActiveRepo,
  running,
  setRunning,
  setError,
  onCaptured,
  onShowMemory,
}) {
  const [form, setForm] = useState(() => ({ ...DEFAULT_FORM, repo: activeRepo || "" }));
  const [result, setResult] = useState(null);
  const fetch_ = createApiFetcher(apiKey);

  useEffect(() => {
    if (activeRepo && !form.repo) {
      setForm((current) => ({ ...current, repo: activeRepo }));
    }
  }, [activeRepo, form.repo]);

  function update(key, value) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  async function captureLesson() {
    setRunning(true);
    setError("");
    setResult(null);
    try {
      const res = await fetch_(`${API}/failguard/lessons`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          repo: form.repo.trim(),
          title: form.title.trim(),
          outcome: form.outcome.trim(),
          lesson: form.lesson.trim(),
          prevention: form.prevention.trim(),
          affected_paths: lines(form.affected_paths),
          evidence: lines(form.evidence),
          disposition: form.disposition,
          pinned: form.pinned,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "RepoMemory could not capture that FailGuard lesson.");
      }
      setResult(data);
      setActiveRepo(data.run.repo);
      onCaptured?.(data.run);
    } catch (err) {
      setError(err.message || "RepoMemory could not capture that FailGuard lesson.");
    } finally {
      setRunning(false);
    }
  }

  const required =
    form.repo.trim() &&
    form.title.trim() &&
    form.outcome.trim() &&
    form.lesson.trim() &&
    form.prevention.trim();

  return (
    <div style={{ display: "grid", gap: 18 }}>
      <div style={{ ...S.panel, display: "grid", gap: 14 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start", flexWrap: "wrap" }}>
          <div style={{ display: "grid", gap: 6 }}>
            <div style={{ fontSize: 18, fontWeight: 800 }}>Capture a FailGuard lesson</div>
            <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>
              Turn a bug, outage, rejected patch, or painful review into a pinned RepoMemory failure pattern that TrustGate can treat as policy.
            </div>
          </div>
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
            <Tag color="var(--accent)">failure pattern</Tag>
            <Tag color="var(--green)">RepoMemory policy</Tag>
            <Tag color="var(--blue)">TrustGate context</Tag>
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Repo</div>
            <Input value={form.repo} onChange={(value) => update("repo", value)} placeholder="owner/repo" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Disposition</div>
            <select
              value={form.disposition}
              onChange={(event) => update("disposition", event.target.value)}
              style={S.select}
            >
              <option value="policy">Policy</option>
              <option value="signal">Signal</option>
            </select>
          </div>
          <label style={{ display: "flex", alignItems: "end", gap: 8, color: "var(--text-dim)", paddingBottom: 9 }}>
            <input
              type="checkbox"
              checked={form.pinned}
              onChange={(event) => update("pinned", event.target.checked)}
            />
            pin this lesson
          </label>
        </div>

        <div style={S.field}>
          <div style={S.label}>Title</div>
          <Input
            value={form.title}
            onChange={(value) => update("title", value)}
            placeholder="Webhook endpoints must fail closed without a secret"
          />
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Bad Outcome</div>
            <textarea
              value={form.outcome}
              onChange={(event) => update("outcome", event.target.value)}
              style={{ ...S.input, minHeight: 106, resize: "vertical" }}
              placeholder="Unsigned webhook payloads could trigger autonomous work."
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Future Prevention</div>
            <textarea
              value={form.prevention}
              onChange={(event) => update("prevention", event.target.value)}
              style={{ ...S.input, minHeight: 106, resize: "vertical" }}
              placeholder="Reject webhook delivery when the signing secret is missing."
            />
          </div>
        </div>

        <div style={S.field}>
          <div style={S.label}>Durable Lesson</div>
          <textarea
            value={form.lesson}
            onChange={(event) => update("lesson", event.target.value)}
            style={{ ...S.input, minHeight: 90, resize: "vertical" }}
            placeholder="Public webhook routes must not accept unsigned payloads or silently skip signature verification."
          />
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Affected Paths</div>
            <textarea
              value={form.affected_paths}
              onChange={(event) => update("affected_paths", event.target.value)}
              style={{ ...S.input, minHeight: 96, resize: "vertical" }}
              placeholder={"backend/src/routes/webhook.rs\ncrates/patchhive-product-core/src/auth.rs"}
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Evidence</div>
            <textarea
              value={form.evidence}
              onChange={(event) => update("evidence", event.target.value)}
              style={{ ...S.input, minHeight: 96, resize: "vertical" }}
              placeholder={"Hermes review C2\nhttps://github.com/owner/repo/pull/123"}
            />
          </div>
        </div>

        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
          <Btn onClick={captureLesson} disabled={running || !required} color="var(--green)">
            {running ? "Capturing..." : "Capture lesson"}
          </Btn>
          {result?.entry && (
            <Btn onClick={onShowMemory} style={{ padding: "4px 10px" }}>
              Open memory
            </Btn>
          )}
        </div>
      </div>

      {result?.entry ? (
        <div style={{ ...S.panel, display: "grid", gap: 10, borderColor: "var(--green)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
            <div>
              <div style={{ fontSize: 16, fontWeight: 800 }}>{result.entry.title}</div>
              <div style={{ color: "var(--text-dim)", fontSize: 11 }}>{result.entry.memory_ref}</div>
            </div>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              <Tag color="var(--accent)">{result.entry.kind}</Tag>
              <Tag color="var(--green)">{result.entry.disposition}</Tag>
              {result.entry.pinned && <Tag color="var(--blue)">pinned</Tag>}
            </div>
          </div>
          <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>{result.entry.detail}</div>
          <div style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 10, background: "var(--bg-input)" }}>
            <div style={S.label}>Prompt line</div>
            <div style={{ marginTop: 6, lineHeight: 1.6 }}>{result.entry.prompt_line}</div>
          </div>
        </div>
      ) : (
        <EmptyState icon="🧠" text="Captured lessons will appear as policy-ready RepoMemory failure patterns." />
      )}
    </div>
  );
}
