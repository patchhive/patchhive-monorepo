import { useMemo, useState } from "react";
import { Btn, DiffViewer, EmptyState, Input, S, ScoreBadge, Tag } from "@patchhivehq/ui";

function recommendationColor(recommendation) {
  if (recommendation === "safe") {
    return "var(--green)";
  }
  if (recommendation === "warn") {
    return "var(--gold)";
  }
  return "var(--accent)";
}

function severityColor(severity) {
  return recommendationColor(severity === "block" ? "block" : severity === "warn" ? "warn" : "safe");
}

function sourceLabel(sourceKind) {
  if (sourceKind === "github_pr") {
    return "GitHub PR";
  }
  return "Manual";
}

export default function ReviewPanel({
  form,
  setForm,
  running,
  onRun,
  onRunGitHub,
  review,
  setReview,
}) {
  const [showDiff, setShowDiff] = useState(false);
  const [showReport, setShowReport] = useState(false);

  const recommendationText = useMemo(() => {
    if (!review) {
      return "";
    }
    return review.recommendation.toUpperCase();
  }, [review]);

  const set = (key, value) => setForm((prev) => ({ ...prev, [key]: value }));

  return (
    <div style={{ display: "grid", gap: 18 }}>
      <div style={{ ...S.panel, display: "grid", gap: 16 }}>
        <div style={{ display: "grid", gap: 6 }}>
          <div style={{ fontSize: 18, fontWeight: 700 }}>Review Intake</div>
          <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>
            TrustGate can still review pasted diffs directly, but it now also knows how to fetch a GitHub pull
            request, review the real diff, and push a status/check back into the PR flow.
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1.5fr 1fr", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Repo</div>
            <Input value={form.repo} onChange={(value) => set("repo", value)} placeholder="owner/repo" />
          </div>
          <div style={S.field}>
            <div style={S.label}>AI Source</div>
            <Input
              value={form.ai_source}
              onChange={(value) => set("ai_source", value)}
              placeholder="Codex, Copilot, Claude, internal agent..."
            />
          </div>
        </div>

        <div
          style={{
            border: "1px solid var(--border)",
            borderRadius: 10,
            padding: 14,
            display: "grid",
            gap: 12,
          }}
        >
          <div>
            <div style={{ fontSize: 14, fontWeight: 700 }}>GitHub PR Review</div>
            <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.5 }}>
              Give TrustGate a repo and PR number, and it will fetch the live diff, apply the repo rule set, and
              optionally post the result back to GitHub.
            </div>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr auto", gap: 12, alignItems: "end" }}>
            <div style={S.field}>
              <div style={S.label}>PR Number</div>
              <Input
                value={form.pr_number}
                onChange={(value) => set("pr_number", value)}
                placeholder="123"
                type="number"
              />
            </div>
            <label
              style={{
                display: "flex",
                gap: 8,
                alignItems: "center",
                color: "var(--text-dim)",
                fontSize: 12,
                paddingBottom: 10,
              }}
            >
              <input
                type="checkbox"
                checked={!!form.publish_status}
                onChange={(event) => set("publish_status", event.target.checked)}
              />
              Publish GitHub status/check
            </label>
          </div>

          <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
            <Btn
              onClick={onRunGitHub}
              disabled={running || !form.repo.trim() || !String(form.pr_number).trim()}
              color="var(--blue)"
            >
              {running ? "Reviewing..." : "Fetch PR + Review"}
            </Btn>
          </div>
        </div>

        <div style={S.field}>
          <div style={S.label}>Manual Unified Diff</div>
          <textarea
            value={form.diff}
            onChange={(event) => set("diff", event.target.value)}
            placeholder="Paste a unified diff here..."
            style={{
              ...S.input,
              minHeight: 260,
              resize: "vertical",
              lineHeight: 1.5,
              whiteSpace: "pre",
            }}
          />
        </div>

        <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
          <Btn onClick={onRun} disabled={running || !form.repo.trim() || !form.diff.trim()}>
            {running ? "Reviewing..." : "Review Pasted Diff"}
          </Btn>
          <Btn onClick={() => setShowDiff(true)} disabled={!form.diff.trim()} color="var(--blue)">
            View Diff
          </Btn>
          <Btn
            onClick={() => {
              setForm({
                repo: "",
                ai_source: "Codex",
                diff: "",
                pr_number: "",
                publish_status: true,
              });
              setReview(null);
            }}
            color="var(--text-dim)"
          >
            Clear
          </Btn>
        </div>
      </div>

      {review ? (
        <>
          <div style={{ ...S.panel, display: "grid", gap: 14 }}>
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                gap: 12,
                flexWrap: "wrap",
                alignItems: "center",
              }}
            >
              <div style={{ display: "grid", gap: 6 }}>
                <div style={{ fontSize: 18, fontWeight: 700 }}>Recommendation</div>
                <div style={{ color: "var(--text-dim)", fontSize: 12 }}>{review.summary}</div>
              </div>
              <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                <Tag color={recommendationColor(review.recommendation)}>{recommendationText}</Tag>
                <ScoreBadge score={review.risk_score} />
              </div>
            </div>

            <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
              <Tag color="var(--blue)">{sourceLabel(review.source_kind)}</Tag>
              <Tag color="var(--blue)">{review.metrics.files_changed} files</Tag>
              <Tag color="var(--green)">{review.metrics.additions} additions</Tag>
              <Tag color="var(--accent)">{review.metrics.deletions} deletions</Tag>
              <Tag color="var(--gold)">{review.metrics.tests_changed} test files</Tag>
              <Tag color="var(--gold)">{review.metrics.generated_files} generated files</Tag>
              <Tag color="var(--accent)">{review.metrics.blocked_findings} blockers</Tag>
              <Tag color="var(--gold)">{review.metrics.warning_findings} warnings</Tag>
            </div>

            {review.github && (
              <div
                style={{
                  border: "1px solid var(--border)",
                  borderRadius: 8,
                  padding: 12,
                  display: "grid",
                  gap: 8,
                }}
              >
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
                  <Tag color="var(--blue)">PR #{review.github.pr_number}</Tag>
                  <Tag color="var(--text-dim)">{review.github.repo}</Tag>
                  {review.github.base_ref && <Tag color="var(--text-dim)">base {review.github.base_ref}</Tag>}
                  {review.github.head_ref && <Tag color="var(--text-dim)">head {review.github.head_ref}</Tag>}
                </div>
                {review.github.pr_title && (
                  <div style={{ fontSize: 13, fontWeight: 700 }}>{review.github.pr_title}</div>
                )}
                {review.github.pr_url && (
                  <a
                    href={review.github.pr_url}
                    target="_blank"
                    rel="noreferrer"
                    style={{ color: "var(--blue)", fontSize: 12 }}
                  >
                    Open pull request
                  </a>
                )}
              </div>
            )}

            {review.github_report && (
              <div
                style={{
                  border: `1px solid ${review.github_report.delivered ? "var(--green)" : "var(--gold)"}33`,
                  background: `${review.github_report.delivered ? "var(--green)" : "var(--gold)"}10`,
                  borderRadius: 8,
                  padding: 12,
                  display: "grid",
                  gap: 8,
                }}
              >
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
                  <div style={{ fontWeight: 700 }}>GitHub Report</div>
                  <Tag color={review.github_report.delivered ? "var(--green)" : "var(--gold)"}>
                    {review.github_report.method || "none"}
                  </Tag>
                  <Tag color={review.github_report.delivered ? "var(--green)" : "var(--gold)"}>
                    {review.github_report.state || "skipped"}
                  </Tag>
                </div>
                <div style={{ color: "var(--text-dim)", fontSize: 12 }}>{review.github_report.message}</div>
                {(review.github_report.check_url ||
                  review.github_report.status_url ||
                  review.github_report.comment_url ||
                  review.github_report.comment_mode) && (
                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
                    {review.github_report.comment_mode && (
                      <Tag color="var(--blue)">{review.github_report.comment_mode} PR comment</Tag>
                    )}
                    {review.github_report.check_url && (
                      <a
                        href={review.github_report.check_url}
                        target="_blank"
                        rel="noreferrer"
                        style={{ color: "var(--blue)", fontSize: 12 }}
                      >
                        Open check run
                      </a>
                    )}
                    {review.github_report.status_url && (
                      <a
                        href={review.github_report.status_url}
                        target="_blank"
                        rel="noreferrer"
                        style={{ color: "var(--blue)", fontSize: 12 }}
                      >
                        Open commit status
                      </a>
                    )}
                    {review.github_report.comment_url && (
                      <a
                        href={review.github_report.comment_url}
                        target="_blank"
                        rel="noreferrer"
                        style={{ color: "var(--blue)", fontSize: 12 }}
                      >
                        Open PR comment
                      </a>
                    )}
                  </div>
                )}
                {review.github_report.details?.length > 0 && (
                  <div style={{ display: "grid", gap: 6 }}>
                    {review.github_report.details.map((item) => (
                      <div key={item} style={{ fontSize: 11, color: "var(--text)" }}>
                        {item}
                      </div>
                    ))}
                  </div>
                )}
                {review.github_report.report_markdown && (
                  <div style={{ display: "grid", gap: 8 }}>
                    <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                      <Btn onClick={() => setShowReport((prev) => !prev)} color="var(--blue)">
                        {showReport ? "Hide report preview" : "Show report preview"}
                      </Btn>
                      <Btn
                        onClick={async () => {
                          try {
                            await navigator.clipboard.writeText(review.github_report.report_markdown);
                          } catch (_) {}
                        }}
                        color="var(--text-dim)"
                      >
                        Copy report markdown
                      </Btn>
                    </div>
                    {showReport && (
                      <textarea
                        readOnly
                        value={review.github_report.report_markdown}
                        style={{
                          ...S.input,
                          minHeight: 220,
                          resize: "vertical",
                          lineHeight: 1.5,
                          whiteSpace: "pre-wrap",
                        }}
                      />
                    )}
                  </div>
                )}
              </div>
            )}
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1.1fr 0.9fr", gap: 18 }}>
            <div style={{ ...S.panel, display: "grid", gap: 12 }}>
              <div style={{ fontSize: 15, fontWeight: 700 }}>Findings</div>
              {review.findings.length === 0 ? (
                <div style={{ color: "var(--green)", fontSize: 12 }}>
                  No active warnings. This diff is currently safe against the applied rule set.
                </div>
              ) : (
                review.findings.map((finding) => (
                  <div
                    key={`${finding.key}-${finding.label}`}
                    style={{
                      border: `1px solid ${severityColor(finding.severity)}33`,
                      background: `${severityColor(finding.severity)}10`,
                      borderRadius: 8,
                      padding: 12,
                      display: "grid",
                      gap: 8,
                    }}
                  >
                    <div style={{ display: "flex", justifyContent: "space-between", gap: 8, flexWrap: "wrap" }}>
                      <div style={{ fontWeight: 700 }}>{finding.label}</div>
                      <Tag color={severityColor(finding.severity)}>{finding.severity}</Tag>
                    </div>
                    <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.5 }}>{finding.detail}</div>
                    {finding.evidence.length > 0 && (
                      <div style={{ display: "grid", gap: 6 }}>
                        {finding.evidence.map((item) => (
                          <div key={item} style={{ color: "var(--text)", fontSize: 11, wordBreak: "break-word" }}>
                            {item}
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                ))
              )}
            </div>

            <div style={{ ...S.panel, display: "grid", gap: 12 }}>
              <div style={{ fontSize: 15, fontWeight: 700 }}>Applied Repo Rules</div>
              <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
                TrustGate used the rule set for <span style={{ color: "var(--text)" }}>{review.rules.repo}</span>.
              </div>
              <div style={{ display: "grid", gap: 8, fontSize: 11 }}>
                <div><strong>Blocked paths:</strong> {review.rules.blocked_paths.join(", ") || "none"}</div>
                <div><strong>Sensitive paths:</strong> {review.rules.warn_paths.join(", ") || "none"}</div>
                <div><strong>Require tests for:</strong> {review.rules.require_test_for_paths.join(", ") || "none"}</div>
                <div><strong>Test path markers:</strong> {review.rules.test_paths.join(", ") || "none"}</div>
                <div><strong>Suspicious terms:</strong> {review.rules.suspicious_terms.join(", ") || "none"}</div>
                <div><strong>Blocked terms:</strong> {review.rules.blocked_terms.join(", ") || "none"}</div>
                <div>
                  <strong>Scope caps:</strong> {review.rules.max_files} files, {review.rules.max_additions} additions,
                  {" "}
                  {review.rules.max_deletions} deletions
                </div>
                {review.rules.notes && <div><strong>Notes:</strong> {review.rules.notes}</div>}
              </div>
            </div>
          </div>

          <div style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div style={{ fontSize: 15, fontWeight: 700 }}>File Assessments</div>
            {review.files.map((file) => (
              <div
                key={file.path}
                style={{
                  border: `1px solid ${recommendationColor(file.status)}33`,
                  borderRadius: 8,
                  padding: 12,
                  display: "grid",
                  gap: 8,
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap" }}>
                  <div style={{ fontWeight: 700, wordBreak: "break-word" }}>{file.path}</div>
                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                    <Tag color={recommendationColor(file.status)}>{file.status}</Tag>
                    {file.generated && <Tag color="var(--gold)">generated</Tag>}
                    <Tag color="var(--green)">+{file.additions}</Tag>
                    <Tag color="var(--accent)">-{file.deletions}</Tag>
                  </div>
                </div>
                <div style={{ color: "var(--text-dim)", fontSize: 12 }}>{file.summary}</div>
                {(file.path_policy || file.matched_rules.length > 0) && (
                  <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                    {file.path_policy && <Tag color="var(--gold)">{file.path_policy}</Tag>}
                    {file.matched_rules.map((rule) => (
                      <Tag key={`${file.path}-${rule}`} color="var(--blue)">
                        {rule}
                      </Tag>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        </>
      ) : (
        <EmptyState
          icon="🛡"
          text="Fetch a GitHub PR or paste your first AI-generated diff to get a safe, warn, or block recommendation."
        />
      )}

      <DiffViewer diff={showDiff ? form.diff : ""} onClose={() => setShowDiff(false)} />
    </div>
  );
}
