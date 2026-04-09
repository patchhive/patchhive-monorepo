import { useEffect, useMemo, useRef, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { Btn, EmptyState, ScoreBadge, Tag } from "@patchhivehq/ui";
import { API } from "../config.js";

function recommendationColor(recommendation) {
  if (recommendation === "safe") {
    return "#15945c";
  }
  if (recommendation === "warn") {
    return "#d1a12b";
  }
  return "#d94b6c";
}

function recommendationText(recommendation) {
  return (recommendation || "unknown").toUpperCase();
}

function sourceLabel(review) {
  if (review?.source_kind === "github_pr") {
    return review?.github?.pr_number ? `GitHub PR #${review.github.pr_number}` : "GitHub PR";
  }
  return "Manual Diff";
}

function nextMove(review) {
  if (review?.recommendation === "safe") {
    return "This patch is within the current repo rules. Review normally, but TrustGate did not find a reason to stop it.";
  }
  if (review?.recommendation === "warn") {
    return "A human should look at the flagged areas before merge. The patch may still be fine, but it no longer looks routine.";
  }
  return "Do not move this patch forward yet. The repo rules say the current risk profile is too high without intervention.";
}

function escapeHtml(value) {
  return String(value || "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function buildDecisionHtml(review) {
  const color = recommendationColor(review.recommendation);
  const findings = review.findings?.length
    ? review.findings
        .map((finding) => {
          const evidence = finding.evidence?.length
            ? `<div class="evidence">${finding.evidence.map((item) => escapeHtml(item)).join("<br/>")}</div>`
            : "";
          return `
            <div class="finding finding-${escapeHtml(finding.severity)}">
              <div class="finding-head">
                <strong>${escapeHtml(finding.label)}</strong>
                <span class="chip chip-${escapeHtml(finding.severity)}">${escapeHtml(finding.severity)}</span>
              </div>
              <div class="finding-detail">${escapeHtml(finding.detail)}</div>
              ${evidence}
            </div>
          `;
        })
        .join("")
    : `<div class="empty">No active warnings. This diff is currently safe against the applied rule set.</div>`;

  const files = review.files?.length
    ? review.files
        .slice(0, 12)
        .map(
          (file) => `
            <div class="file-row">
              <div>
                <div class="file-path">${escapeHtml(file.path)}</div>
                <div class="file-summary">${escapeHtml(file.summary || "")}</div>
              </div>
              <div class="file-meta">
                <span class="chip chip-${escapeHtml(file.status)}">${escapeHtml(file.status)}</span>
                <span>+${escapeHtml(file.additions)}</span>
                <span>-${escapeHtml(file.deletions)}</span>
              </div>
            </div>
          `
        )
        .join("")
    : `<div class="empty">No file-level assessments were recorded.</div>`;

  const githubBlock = review.github
    ? `
      <section class="card">
        <h2>GitHub Context</h2>
        <div class="meta-grid">
          <div><span class="meta-label">Repo</span><div>${escapeHtml(review.github.repo)}</div></div>
          <div><span class="meta-label">PR</span><div>#${escapeHtml(review.github.pr_number)}</div></div>
          <div><span class="meta-label">Base</span><div>${escapeHtml(review.github.base_ref || "-")}</div></div>
          <div><span class="meta-label">Head</span><div>${escapeHtml(review.github.head_ref || "-")}</div></div>
        </div>
        ${review.github.pr_title ? `<div class="pr-title">${escapeHtml(review.github.pr_title)}</div>` : ""}
      </section>
    `
    : "";

  const reportBlock = review.github_report
    ? `
      <section class="card">
        <h2>GitHub Report</h2>
        <div class="meta-grid">
          <div><span class="meta-label">Method</span><div>${escapeHtml(review.github_report.method || "none")}</div></div>
          <div><span class="meta-label">State</span><div>${escapeHtml(review.github_report.state || "skipped")}</div></div>
          <div><span class="meta-label">Template</span><div>${escapeHtml(
            review.github_report.template_scope === "repo" ? "custom template" : "default template"
          )}</div></div>
          <div><span class="meta-label">Comment Mode</span><div>${escapeHtml(review.github_report.comment_mode || "-")}</div></div>
        </div>
        <div class="report-message">${escapeHtml(review.github_report.message || "")}</div>
      </section>
    `
    : "";

  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>TrustGate Decision ${escapeHtml(review.id)}</title>
    <style>
      :root {
        color-scheme: light;
      }
      * { box-sizing: border-box; }
      body {
        margin: 0;
        font-family: "SF Mono", "Fira Mono", monospace;
        background: #f6f4ef;
        color: #19171b;
      }
      .page {
        max-width: 1080px;
        margin: 0 auto;
        padding: 32px 24px 48px;
      }
      .hero {
        border: 1px solid #d7d1c5;
        background: #fffdfa;
        border-radius: 16px;
        padding: 24px;
        margin-bottom: 18px;
      }
      .hero-top, .tag-row, .metric-grid, .meta-grid, .finding-head, .file-row, .file-meta {
        display: flex;
        gap: 12px;
        flex-wrap: wrap;
        align-items: center;
      }
      .hero-top { justify-content: space-between; align-items: flex-start; }
      .title {
        font-size: 30px;
        font-weight: 800;
        margin: 0 0 6px;
      }
      .subtitle, .meta-label, .file-summary, .report-message {
        color: #5e5962;
        font-size: 12px;
        line-height: 1.6;
      }
      .card {
        border: 1px solid #d7d1c5;
        background: #fff;
        border-radius: 14px;
        padding: 18px;
        margin-bottom: 16px;
      }
      h2 {
        margin: 0 0 14px;
        font-size: 16px;
      }
      .chip {
        display: inline-flex;
        align-items: center;
        border-radius: 999px;
        padding: 4px 10px;
        font-size: 11px;
        border: 1px solid #d7d1c5;
        background: #f4f1ea;
      }
      .chip-safe { color: #116d46; border-color: #a8d8c0; background: #e8f7ef; }
      .chip-warn { color: #8d6a09; border-color: #ead59f; background: #fff6da; }
      .chip-block { color: #a2334e; border-color: #f1b2c0; background: #fff0f4; }
      .metric-grid > div, .meta-grid > div {
        min-width: 150px;
      }
      .metric-value {
        font-size: 22px;
        font-weight: 800;
      }
      .finding {
        border: 1px solid #e6dfd5;
        border-radius: 10px;
        padding: 12px;
        display: grid;
        gap: 8px;
        margin-bottom: 10px;
      }
      .finding-safe { background: #f4faf7; }
      .finding-warn { background: #fff9ea; }
      .finding-block { background: #fff3f6; }
      .finding-detail, .evidence {
        font-size: 12px;
        line-height: 1.6;
      }
      .file-row {
        justify-content: space-between;
        align-items: flex-start;
        border-top: 1px solid #eee7dc;
        padding: 10px 0;
      }
      .file-row:first-child { border-top: 0; padding-top: 0; }
      .file-path {
        font-weight: 700;
        font-size: 12px;
        word-break: break-word;
      }
      .empty {
        color: #5e5962;
        font-size: 12px;
      }
      .footer {
        margin-top: 24px;
        color: #5e5962;
        font-size: 11px;
      }
      @media print {
        body { background: white; }
        .page { max-width: none; padding: 0; }
        .hero, .card { break-inside: avoid; }
      }
    </style>
  </head>
  <body>
    <div class="page">
      <section class="hero">
        <div class="hero-top">
          <div>
            <div class="title">TrustGate Decision</div>
            <div class="subtitle">${escapeHtml(review.summary || "")}</div>
          </div>
          <div class="chip" style="border-color:${color};color:${color};font-weight:700">${escapeHtml(
            recommendationText(review.recommendation)
          )}</div>
        </div>
        <div class="tag-row" style="margin-top:14px">
          <span class="chip">${escapeHtml(review.repo)}</span>
          <span class="chip">${escapeHtml(sourceLabel(review))}</span>
          <span class="chip">${escapeHtml(review.ai_source || "unknown")}</span>
          <span class="chip">${escapeHtml(new Date(review.created_at).toLocaleString())}</span>
          <span class="chip">Review ID ${escapeHtml(review.id)}</span>
        </div>
      </section>

      <section class="card">
        <h2>Risk Snapshot</h2>
        <div class="metric-grid">
          <div><span class="meta-label">Risk Score</span><div class="metric-value">${escapeHtml(review.risk_score)}</div></div>
          <div><span class="meta-label">Files Changed</span><div class="metric-value">${escapeHtml(review.metrics?.files_changed)}</div></div>
          <div><span class="meta-label">Additions</span><div class="metric-value">+${escapeHtml(review.metrics?.additions)}</div></div>
          <div><span class="meta-label">Deletions</span><div class="metric-value">-${escapeHtml(review.metrics?.deletions)}</div></div>
          <div><span class="meta-label">Tests Changed</span><div class="metric-value">${escapeHtml(review.metrics?.tests_changed)}</div></div>
          <div><span class="meta-label">Generated Files</span><div class="metric-value">${escapeHtml(review.metrics?.generated_files)}</div></div>
          <div><span class="meta-label">Blocking Findings</span><div class="metric-value">${escapeHtml(review.metrics?.blocked_findings)}</div></div>
          <div><span class="meta-label">Warning Findings</span><div class="metric-value">${escapeHtml(review.metrics?.warning_findings)}</div></div>
        </div>
      </section>

      ${githubBlock}
      ${reportBlock}

      <section class="card">
        <h2>Recommended Next Move</h2>
        <div class="subtitle">${escapeHtml(nextMove(review))}</div>
      </section>

      <section class="card">
        <h2>Findings</h2>
        ${findings}
      </section>

      <section class="card">
        <h2>File Hotspots</h2>
        ${files}
      </section>

      ${
        review.rules?.notes
          ? `<section class="card"><h2>Rule Notes</h2><div class="subtitle">${escapeHtml(review.rules.notes)}</div></section>`
          : ""
      }

      <div class="footer">TrustGate by PatchHive</div>
    </div>
  </body>
</html>`;
}

export default function DecisionViewPage({ apiKey, reviewId, printMode = false }) {
  const [review, setReview] = useState(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const didAutoPrint = useRef(false);
  const fetch_ = createApiFetcher(apiKey);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError("");

    fetch_(`${API}/history/${encodeURIComponent(reviewId)}`)
      .then((res) => res.json().then((data) => ({ ok: res.ok, data })))
      .then(({ ok, data }) => {
        if (!active) {
          return;
        }
        if (!ok) {
          throw new Error(data.error || "TrustGate could not load that decision.");
        }
        setReview(data);
      })
      .catch((err) => {
        if (active) {
          setError(err.message || "TrustGate could not load that decision.");
        }
      })
      .finally(() => {
        if (active) {
          setLoading(false);
        }
      });

    return () => {
      active = false;
    };
  }, [apiKey, reviewId]);

  useEffect(() => {
    if (printMode && review && !didAutoPrint.current) {
      didAutoPrint.current = true;
      window.setTimeout(() => window.print(), 80);
    }
  }, [printMode, review]);

  const color = useMemo(() => recommendationColor(review?.recommendation), [review]);

  if (loading) {
    return (
      <div style={{ minHeight: "100vh", display: "grid", placeItems: "center", background: "var(--bg)", color: "var(--text-dim)" }}>
        Loading TrustGate decision...
      </div>
    );
  }

  if (error) {
    return (
      <div style={{ minHeight: "100vh", display: "grid", placeItems: "center", background: "var(--bg)", padding: 24 }}>
        <EmptyState icon="!" text={error} />
      </div>
    );
  }

  if (!review) {
    return (
      <div style={{ minHeight: "100vh", display: "grid", placeItems: "center", background: "var(--bg)", padding: 24 }}>
        <EmptyState icon="?" text="TrustGate could not find that review." />
      </div>
    );
  }

  const exportHtml = () => {
    const html = buildDecisionHtml(review);
    const blob = new Blob([html], { type: "text/html;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `trustgate-decision-${review.id}.html`;
    link.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div
      style={{
        minHeight: "100vh",
        background: "#f5f0e7",
        color: "#18151a",
        fontFamily: "'SF Mono','Fira Mono',monospace",
      }}
    >
      <style>{`
        @media print {
          .trustgate-print-actions {
            display: none !important;
          }
          body {
            background: white !important;
          }
        }
      `}</style>

      <div style={{ maxWidth: 1120, margin: "0 auto", padding: "28px 20px 48px", display: "grid", gap: 18 }}>
        <div
          style={{
            border: "1px solid #d9cfbe",
            background: "#fffaf3",
            borderRadius: 18,
            padding: 22,
            display: "grid",
            gap: 14,
          }}
        >
          <div style={{ display: "flex", justifyContent: "space-between", gap: 16, flexWrap: "wrap", alignItems: "flex-start" }}>
            <div style={{ display: "grid", gap: 6 }}>
              <div style={{ fontSize: 30, fontWeight: 800 }}>TrustGate Decision</div>
              <div style={{ color: "#5f5963", fontSize: 13, lineHeight: 1.6 }}>{review.summary}</div>
            </div>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
              <Tag color={color}>{recommendationText(review.recommendation)}</Tag>
              <ScoreBadge score={review.risk_score} />
            </div>
          </div>

          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
            <Tag color="var(--blue)">{review.repo}</Tag>
            <Tag color="var(--blue)">{sourceLabel(review)}</Tag>
            <Tag color="var(--text-dim)">{review.ai_source || "unknown"}</Tag>
            <Tag color="var(--text-dim)">{new Date(review.created_at).toLocaleString()}</Tag>
            <Tag color="var(--text-dim)">ID {review.id}</Tag>
          </div>

          <div className="trustgate-print-actions" style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
            <Btn onClick={() => window.print()}>Print / Save PDF</Btn>
            <Btn onClick={exportHtml} color="var(--blue)">
              Export HTML
            </Btn>
            <Btn onClick={() => window.open(`${window.location.origin}/`, "_self")} color="var(--text-dim)">
              Back To App
            </Btn>
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(150px, 1fr))", gap: 12 }}>
          {[
            ["Files Changed", review.metrics.files_changed],
            ["Additions", `+${review.metrics.additions}`],
            ["Deletions", `-${review.metrics.deletions}`],
            ["Tests Changed", review.metrics.tests_changed],
            ["Generated Files", review.metrics.generated_files],
            ["Blockers", review.metrics.blocked_findings],
            ["Warnings", review.metrics.warning_findings],
          ].map(([label, value]) => (
            <div key={label} style={{ border: "1px solid #d9cfbe", background: "#fff", borderRadius: 14, padding: 14 }}>
              <div style={{ color: "#5f5963", fontSize: 11, marginBottom: 6 }}>{label}</div>
              <div style={{ fontSize: 24, fontWeight: 800 }}>{value}</div>
            </div>
          ))}
        </div>

        {review.github && (
          <div style={{ border: "1px solid #d9cfbe", background: "#fff", borderRadius: 14, padding: 18, display: "grid", gap: 10 }}>
            <div style={{ fontSize: 16, fontWeight: 700 }}>GitHub Context</div>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <Tag color="var(--blue)">PR #{review.github.pr_number}</Tag>
              <Tag color="var(--text-dim)">{review.github.repo}</Tag>
              {review.github.base_ref && <Tag color="var(--text-dim)">base {review.github.base_ref}</Tag>}
              {review.github.head_ref && <Tag color="var(--text-dim)">head {review.github.head_ref}</Tag>}
            </div>
            {review.github.pr_title && <div style={{ fontSize: 14, fontWeight: 700 }}>{review.github.pr_title}</div>}
            {review.github.pr_url && (
              <a href={review.github.pr_url} target="_blank" rel="noreferrer" style={{ color: "var(--blue)", fontSize: 12 }}>
                Open pull request
              </a>
            )}
          </div>
        )}

        {review.github_report && (
          <div style={{ border: "1px solid #d9cfbe", background: "#fff", borderRadius: 14, padding: 18, display: "grid", gap: 10 }}>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
              <div style={{ fontSize: 16, fontWeight: 700 }}>GitHub Report</div>
              <Tag color={review.github_report.delivered ? "var(--green)" : "var(--gold)"}>
                {review.github_report.method || "none"}
              </Tag>
              <Tag color={review.github_report.delivered ? "var(--green)" : "var(--gold)"}>
                {review.github_report.state || "skipped"}
              </Tag>
              {review.github_report.template_scope && (
                <Tag color="var(--blue)">
                  {review.github_report.template_scope === "repo" ? "custom template" : "default template"}
                </Tag>
              )}
            </div>
            <div style={{ color: "#5f5963", fontSize: 12, lineHeight: 1.6 }}>{review.github_report.message}</div>
            {(review.github_report.check_url || review.github_report.status_url || review.github_report.comment_url) && (
              <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
                {review.github_report.check_url && (
                  <a href={review.github_report.check_url} target="_blank" rel="noreferrer" style={{ color: "var(--blue)", fontSize: 12 }}>
                    Open check run
                  </a>
                )}
                {review.github_report.status_url && (
                  <a href={review.github_report.status_url} target="_blank" rel="noreferrer" style={{ color: "var(--blue)", fontSize: 12 }}>
                    Open commit status
                  </a>
                )}
                {review.github_report.comment_url && (
                  <a href={review.github_report.comment_url} target="_blank" rel="noreferrer" style={{ color: "var(--blue)", fontSize: 12 }}>
                    Open PR comment
                  </a>
                )}
              </div>
            )}
          </div>
        )}

        <div style={{ border: "1px solid #d9cfbe", background: "#fff", borderRadius: 14, padding: 18, display: "grid", gap: 10 }}>
          <div style={{ fontSize: 16, fontWeight: 700 }}>Recommended Next Move</div>
          <div style={{ color: "#5f5963", fontSize: 13, lineHeight: 1.7 }}>{nextMove(review)}</div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1.05fr 0.95fr", gap: 16 }}>
          <div style={{ border: "1px solid #d9cfbe", background: "#fff", borderRadius: 14, padding: 18, display: "grid", gap: 10 }}>
            <div style={{ fontSize: 16, fontWeight: 700 }}>Findings</div>
            {review.findings.length === 0 ? (
              <div style={{ color: "#1b7a4d", fontSize: 12 }}>No active warnings. This diff is currently safe against the applied rule set.</div>
            ) : (
              review.findings.map((finding) => (
                <div
                  key={`${finding.key}-${finding.label}`}
                  style={{
                    border: `1px solid ${recommendationColor(finding.severity === "block" ? "block" : finding.severity)}33`,
                    background: `${recommendationColor(finding.severity === "block" ? "block" : finding.severity)}10`,
                    borderRadius: 10,
                    padding: 12,
                    display: "grid",
                    gap: 8,
                  }}
                >
                  <div style={{ display: "flex", justifyContent: "space-between", gap: 8, flexWrap: "wrap" }}>
                    <div style={{ fontWeight: 700 }}>{finding.label}</div>
                    <Tag color={recommendationColor(finding.severity === "block" ? "block" : finding.severity)}>
                      {finding.severity}
                    </Tag>
                  </div>
                  <div style={{ color: "#5f5963", fontSize: 12, lineHeight: 1.6 }}>{finding.detail}</div>
                  {finding.evidence?.length > 0 && (
                    <div style={{ display: "grid", gap: 4 }}>
                      {finding.evidence.map((item) => (
                        <div key={item} style={{ fontSize: 11, color: "#1f1a22", wordBreak: "break-word" }}>
                          {item}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              ))
            )}
          </div>

          <div style={{ border: "1px solid #d9cfbe", background: "#fff", borderRadius: 14, padding: 18, display: "grid", gap: 10 }}>
            <div style={{ fontSize: 16, fontWeight: 700 }}>File Hotspots</div>
            {review.files.length === 0 ? (
              <div style={{ color: "#5f5963", fontSize: 12 }}>No file-level assessments were recorded.</div>
            ) : (
              review.files.slice(0, 12).map((file) => (
                <div key={file.path} style={{ borderTop: "1px solid #ede4d6", paddingTop: 10, display: "grid", gap: 6 }}>
                  <div style={{ display: "flex", justifyContent: "space-between", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
                    <div style={{ fontWeight: 700, fontSize: 12, wordBreak: "break-word" }}>{file.path}</div>
                    <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                      <Tag color={recommendationColor(file.status)}>{file.status}</Tag>
                      <Tag color="var(--green)">+{file.additions}</Tag>
                      <Tag color="var(--accent)">-{file.deletions}</Tag>
                    </div>
                  </div>
                  <div style={{ color: "#5f5963", fontSize: 11, lineHeight: 1.6 }}>{file.summary}</div>
                  {(file.generated || file.path_policy) && (
                    <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                      {file.generated && <Tag color="var(--gold)">generated</Tag>}
                      {file.path_policy && <Tag color="var(--blue)">{file.path_policy}</Tag>}
                    </div>
                  )}
                </div>
              ))
            )}
          </div>
        </div>

        {review.rules?.notes && (
          <div style={{ border: "1px solid #d9cfbe", background: "#fff", borderRadius: 14, padding: 18, display: "grid", gap: 8 }}>
            <div style={{ fontSize: 16, fontWeight: 700 }}>Rule Notes</div>
            <div style={{ color: "#5f5963", fontSize: 12, lineHeight: 1.7 }}>{review.rules.notes}</div>
          </div>
        )}
      </div>
    </div>
  );
}
