import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, Input, S, Tag } from "@patchhivehq/ui";

const statusColor = (status) =>
  status === "pass" || status === "ready"
    ? "var(--green)"
    : status === "warn" || status === "watch"
      ? "var(--gold)"
      : "var(--accent)";

function Metric({ label, value, color = "var(--text)" }) {
  return (
    <div style={{ ...S.panel, padding: 12, minWidth: 120 }}>
      <div style={S.label}>{label}</div>
      <div style={{ fontSize: 22, fontWeight: 800, color }}>{value}</div>
    </div>
  );
}

function Field({ label, children }) {
  return (
    <label style={S.field}>
      <span style={S.label}>{label}</span>
      {children}
    </label>
  );
}

function CheckCard({ check }) {
  return (
    <div style={{ ...S.panel, display: "grid", gap: 8 }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 10, alignItems: "center" }}>
        <div style={{ fontSize: 13, fontWeight: 800 }}>{check.label}</div>
        <Tag color={statusColor(check.status)}>{check.status}</Tag>
      </div>
      <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.55 }}>{check.detail}</div>
      {check.evidence?.length > 0 && (
        <div style={{ display: "grid", gap: 4 }}>
          {check.evidence.map((item) => (
            <div key={item} style={{ color: "var(--text-muted)", fontSize: 11 }}>
              {item}
            </div>
          ))}
        </div>
      )}
      {check.links?.length > 0 && (
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          {check.links.map((link) => (
            <a key={`${link.label}-${link.url}`} href={link.url} target="_blank" rel="noreferrer" style={{ color: "var(--accent)", fontSize: 11 }}>
              {link.label}
            </a>
          ))}
        </div>
      )}
    </div>
  );
}

export default function OverviewPanel({ apiKey }) {
  const [overview, setOverview] = useState(null);
  const [result, setResult] = useState(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [form, setForm] = useState({
    repo: "patchhive/patchhive2",
    branch: "main",
    target_version: "",
    target_tag: "",
    changelog_path: "CHANGELOG.md",
    workflow_run_limit: "20",
  });
  const fetch_ = createApiFetcher(apiKey);

  const setField = (key, value) => setForm((current) => ({ ...current, [key]: value }));

  const refresh = () => {
    fetch_(`${API}/overview`)
      .then((res) => res.json())
      .then(setOverview)
      .catch(() => setOverview(null));
  };

  useEffect(() => {
    refresh();
  }, [apiKey]);

  const runCheck = async () => {
    setLoading(true);
    setError("");
    try {
      const res = await fetch_(`${API}/check/github/release`, {
        method: "POST",
        body: JSON.stringify({
          ...form,
          workflow_run_limit: Number(form.workflow_run_limit || 20),
          blocker_labels: ["release-blocker", "blocker", "critical", "regression"],
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || `Release check failed with HTTP ${res.status}`);
      }
      setResult(data);
      refresh();
    } catch (err) {
      setError(err.message || String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ display: "grid", gap: 18 }}>
      <div style={{ ...S.panel, display: "grid", gap: 14 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 16, flexWrap: "wrap" }}>
          <div>
            <div style={S.label}>Release gate</div>
            <div style={{ fontSize: 24, fontWeight: 900, letterSpacing: "-0.04em" }}>
              Ship/no-ship evidence.
            </div>
            <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6, maxWidth: 720 }}>
              ReleaseSentry checks GitHub releases, tags, changelog coverage, Actions health, open blocker labels,
              and common release surface files before it makes a ready/watch/hold call.
            </div>
          </div>
          <div style={{ display: "flex", gap: 8, alignItems: "flex-start", flexWrap: "wrap" }}>
            <Tag color="var(--green)">read-only</Tag>
            <Tag color="var(--gold)">ready / watch / hold</Tag>
            <Tag color="var(--accent)">HiveCore dispatchable</Tag>
          </div>
        </div>

        {overview && (
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(120px, 1fr))", gap: 10 }}>
            <Metric label="Runs" value={overview.counts.runs} />
            <Metric label="Repos" value={overview.counts.repos} />
            <Metric label="Ready" value={overview.counts.ready} color="var(--green)" />
            <Metric label="Watch" value={overview.counts.watch} color="var(--gold)" />
            <Metric label="Hold" value={overview.counts.hold} color="var(--accent)" />
          </div>
        )}
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: 18, alignItems: "start" }}>
        <div style={{ ...S.panel, display: "grid", gap: 12 }}>
          <div>
            <div style={{ fontSize: 16, fontWeight: 800 }}>Check a release candidate</div>
            <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.5 }}>
              Leave version/tag blank to check the current release posture instead of a specific tag.
            </div>
          </div>
          <Field label="Repository">
            <Input value={form.repo} onChange={(value) => setField("repo", value)} placeholder="owner/name" />
          </Field>
          <Field label="Branch">
            <Input value={form.branch} onChange={(value) => setField("branch", value)} placeholder="main" />
          </Field>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
            <Field label="Target version">
              <Input value={form.target_version} onChange={(value) => setField("target_version", value)} placeholder="0.2.0" />
            </Field>
            <Field label="Target tag">
              <Input value={form.target_tag} onChange={(value) => setField("target_tag", value)} placeholder="v0.2.0" />
            </Field>
          </div>
          <Field label="Changelog path">
            <Input value={form.changelog_path} onChange={(value) => setField("changelog_path", value)} placeholder="CHANGELOG.md" />
          </Field>
          <Field label="Workflow run limit">
            <Input value={form.workflow_run_limit} onChange={(value) => setField("workflow_run_limit", value)} placeholder="20" />
          </Field>
          {error && <div style={{ color: "var(--accent)", fontSize: 12 }}>{error}</div>}
          <Btn onClick={runCheck} disabled={loading || !form.repo.trim()} color="var(--green)">
            {loading ? "Checking..." : "Run ReleaseSentry"}
          </Btn>
        </div>

        {result ? (
          <div style={{ display: "grid", gap: 14 }}>
            <div style={{ ...S.panel, display: "grid", gap: 10 }}>
              <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap" }}>
                <div>
                  <div style={S.label}>{result.repo} · {result.branch}</div>
                  <div style={{ fontSize: 22, fontWeight: 900, color: statusColor(result.decision) }}>
                    {result.decision?.toUpperCase()} · {result.score}/100
                  </div>
                </div>
                <Tag color={statusColor(result.decision)}>{result.id}</Tag>
              </div>
              <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{result.summary}</div>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                <Tag color="var(--green)">{result.metrics.passed} passed</Tag>
                <Tag color="var(--gold)">{result.metrics.warned} warned</Tag>
                <Tag color="var(--accent)">{result.metrics.blocked} blocked</Tag>
                <Tag color="var(--blue)">{result.metrics.workflow_runs} workflow runs</Tag>
              </div>
              {result.warnings?.length > 0 && (
                <div style={{ display: "grid", gap: 4 }}>
                  {result.warnings.map((warning) => (
                    <div key={warning} style={{ color: "var(--gold)", fontSize: 11 }}>{warning}</div>
                  ))}
                </div>
              )}
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))", gap: 12 }}>
              {result.checks?.map((check) => <CheckCard key={check.key} check={check} />)}
            </div>
          </div>
        ) : (
          <EmptyState icon="🚦" text="Run a release readiness check to see the ship/no-ship evidence." />
        )}
      </div>
    </div>
  );
}
