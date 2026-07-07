import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import {
  Btn,
  EmptyState,
  Input,
  S,
  ScoreBadge,
  Sel,
  Tag,
  timeAgo,
} from "@patchhivehq/ui";

const SORT_OPTIONS = [
  { v: "risk", l: "Risk first" },
  { v: "recommendation", l: "Recommendation" },
  { v: "severity", l: "Severity" },
  { v: "owner", l: "Owner hint" },
  { v: "source", l: "Source" },
];

const TOGGLE_OPTIONS = [
  { v: "yes", l: "Include" },
  { v: "no", l: "Skip" },
];

function recommendationColor(recommendation) {
  if (recommendation === "fix_now") {
    return "var(--accent)";
  }
  if (recommendation === "plan_next") {
    return "var(--gold)";
  }
  return "var(--text-dim)";
}

function severityColor(severity) {
  if (severity === "critical" || severity === "high") {
    return "var(--accent)";
  }
  if (severity === "medium" || severity === "moderate") {
    return "var(--gold)";
  }
  if (severity === "low") {
    return "var(--green)";
  }
  return "var(--text-dim)";
}

function recommendationRank(value) {
  if (value === "fix_now") {
    return 3;
  }
  if (value === "plan_next") {
    return 2;
  }
  return 1;
}

function severityRank(value) {
  if (value === "critical") {
    return 5;
  }
  if (value === "high") {
    return 4;
  }
  if (value === "medium" || value === "moderate") {
    return 3;
  }
  if (value === "low") {
    return 2;
  }
  return 1;
}

function sortFindings(items, sortBy) {
  return [...items].sort((left, right) => {
    if (sortBy === "recommendation") {
      return (
        recommendationRank(right.recommendation) - recommendationRank(left.recommendation) ||
        right.score - left.score ||
        severityRank(right.severity) - severityRank(left.severity)
      );
    }
    if (sortBy === "severity") {
      return (
        severityRank(right.severity) - severityRank(left.severity) ||
        right.score - left.score
      );
    }
    if (sortBy === "owner") {
      return left.owner_hint.localeCompare(right.owner_hint) || right.score - left.score;
    }
    if (sortBy === "source") {
      return left.source.localeCompare(right.source) || right.score - left.score;
    }
    return (
      right.score - left.score ||
      recommendationRank(right.recommendation) - recommendationRank(left.recommendation) ||
      severityRank(right.severity) - severityRank(left.severity)
    );
  });
}

function buildScanMarkdown(scan) {
  const lines = [
    `# VulnTriage scan for ${scan.repo}`,
    "",
    scan.summary,
    "",
    `- Tracked findings: ${scan.metrics.tracked_findings}`,
    `- Fix now: ${scan.metrics.fix_now}`,
    `- Plan next: ${scan.metrics.plan_next}`,
    `- Watch: ${scan.metrics.watch}`,
    `- Code scanning alerts: ${scan.metrics.code_scanning_alerts}`,
    `- Dependency alerts: ${scan.metrics.dependency_alerts}`,
    `- Runtime exposed: ${scan.metrics.runtime_exposed}`,
    `- Owner scoped: ${scan.metrics.owner_scoped}`,
  ];

  if (scan.findings?.length) {
    lines.push("", "## Top findings", "");
    sortFindings(scan.findings, "risk")
      .slice(0, 8)
      .forEach((item) => {
        lines.push(
          `- [${item.recommendation.replace("_", " ")}] ${item.title} — ${item.owner_hint} — ${item.next_action}`
        );
      });
  }

  if (scan.warnings?.length) {
    lines.push("", "## Warnings", "");
    scan.warnings.forEach((warning) => lines.push(`- ${warning}`));
  }

  return lines.join("\n");
}

export default function TriagePanel({
  apiKey,
  form,
  setForm,
  running,
  onRun,
  scan,
}) {
  const [overview, setOverview] = useState(null);
  const [sortBy, setSortBy] = useState("risk");
  const [copyState, setCopyState] = useState("");
  const fetch_ = createApiFetcher(apiKey);

  useEffect(() => {
    fetch_(`${API}/overview`)
      .then((res) => res.json())
      .then(setOverview)
      .catch(() => setOverview(null));
  }, [apiKey, scan?.id]);

  const sortedFindings = scan?.findings?.length ? sortFindings(scan.findings, sortBy) : [];

  async function copySummary() {
    if (!scan || !navigator?.clipboard?.writeText) {
      return;
    }
    try {
      await navigator.clipboard.writeText(buildScanMarkdown(scan));
      setCopyState("Copied");
      window.setTimeout(() => setCopyState(""), 1800);
    } catch {
      setCopyState("Copy failed");
      window.setTimeout(() => setCopyState(""), 1800);
    }
  }

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div style={{ ...S.panel, display: "grid", gap: 14 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center" }}>
          <div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>Rank open security findings</div>
            <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
              VulnTriage ingests GitHub code scanning and dependency alerts, then turns them into a queue of what matters most, where it lives, who should care, and what should happen next.
            </div>
          </div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <Tag color="var(--accent)">fix now</Tag>
            <Tag color="var(--gold)">plan next</Tag>
            <Tag color="var(--text-dim)">watch</Tag>
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "minmax(280px, 2fr) minmax(160px, 1fr) minmax(160px, 1fr) auto", gap: 12, alignItems: "end" }}>
          <div>
            <div style={S.label}>Repository</div>
            <Input
              value={form.repo}
              onChange={(value) => setForm((prev) => ({ ...prev, repo: value }))}
              placeholder="owner/repo"
            />
          </div>
          <div>
            <div style={S.label}>Code scanning</div>
            <Sel
              value={form.include_code_scanning}
              onChange={(value) => setForm((prev) => ({ ...prev, include_code_scanning: value }))}
              opts={TOGGLE_OPTIONS}
            />
          </div>
          <div>
            <div style={S.label}>Dependency alerts</div>
            <Sel
              value={form.include_dependency_alerts}
              onChange={(value) => setForm((prev) => ({ ...prev, include_dependency_alerts: value }))}
              opts={TOGGLE_OPTIONS}
            />
          </div>
          <Btn onClick={onRun} disabled={running}>
            {running ? "Scanning..." : "Run VulnTriage"}
          </Btn>
        </div>
      </div>

      {overview && (
        <div style={{ ...S.panel, display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(150px, 1fr))", gap: 12 }}>
          <Metric label="Saved scans" value={overview.counts.scans} />
          <Metric label="Repos seen" value={overview.counts.repos} />
          <Metric label="Tracked findings" value={overview.counts.tracked_findings} />
          <Metric label="Fix now" value={overview.counts.fix_now} color="var(--accent)" />
          <Metric label="Plan next" value={overview.counts.plan_next} color="var(--gold)" />
          <Metric label="Watch" value={overview.counts.watch} color="var(--text-dim)" />
        </div>
      )}

      {scan ? (
        <div style={{ display: "grid", gap: 16 }}>
          <div style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "start" }}>
              <div style={{ display: "grid", gap: 8 }}>
                <div style={{ fontSize: 18, fontWeight: 700 }}>{scan.repo}</div>
                <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{scan.summary}</div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  <Tag color="var(--blue)">{scan.repo}</Tag>
                  <Tag color="var(--text-dim)">{timeAgo(scan.created_at)}</Tag>
                  <Tag color="var(--accent)">{scan.metrics.fix_now} fix now</Tag>
                  <Tag color="var(--gold)">{scan.metrics.plan_next} plan next</Tag>
                  <Tag color="var(--text-dim)">{scan.metrics.watch} watch</Tag>
                </div>
              </div>

              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                <Btn onClick={copySummary}>{copyState || "Copy summary"}</Btn>
                <Sel value={sortBy} onChange={setSortBy} opts={SORT_OPTIONS} style={{ minWidth: 150 }} />
              </div>
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: 12 }}>
              <Metric label="Tracked findings" value={scan.metrics.tracked_findings} />
              <Metric label="Code scanning" value={scan.metrics.code_scanning_alerts} color="var(--blue)" />
              <Metric label="Dependency alerts" value={scan.metrics.dependency_alerts} color="var(--gold)" />
              <Metric label="Runtime exposed" value={scan.metrics.runtime_exposed} color="var(--accent)" />
              <Metric label="Owner scoped" value={scan.metrics.owner_scoped} color="var(--green)" />
            </div>
          </div>

          {scan.warnings?.length > 0 && (
            <div style={{ ...S.panel, display: "grid", gap: 8, borderColor: "var(--gold)" }}>
              <div style={{ fontSize: 15, fontWeight: 700 }}>Scan warnings</div>
              {scan.warnings.map((warning) => (
                <div key={warning} style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>
                  {warning}
                </div>
              ))}
            </div>
          )}

          {sortedFindings.length === 0 ? (
            <EmptyState icon="🛡" text="This scan did not surface any open security findings worth ranking." />
          ) : (
            sortedFindings.map((item) => (
              <div key={item.key} style={{ ...S.panel, display: "grid", gap: 12, borderColor: recommendationColor(item.recommendation) }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "start" }}>
                  <div style={{ display: "grid", gap: 6 }}>
                    <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
                      <div style={{ fontSize: 16, fontWeight: 700 }}>{item.title}</div>
                      <ScoreBadge score={item.score} />
                    </div>
                    <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{item.summary}</div>
                  </div>
                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                    <Tag color={recommendationColor(item.recommendation)}>
                      {item.recommendation.replace("_", " ")}
                    </Tag>
                    <Tag color={severityColor(item.severity)}>{item.severity}</Tag>
                    <Tag color="var(--blue)">{item.source.replace("_", " ")}</Tag>
                  </div>
                </div>

                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  {item.owner_hint && <Tag color="var(--green)">{item.owner_hint}</Tag>}
                  {item.location && <Tag color="var(--text-dim)">{item.location}</Tag>}
                  {item.package_name && <Tag color="var(--gold)">{item.package_name}</Tag>}
                  {item.ecosystem && <Tag color="var(--blue)">{item.ecosystem}</Tag>}
                  {item.reachability && <Tag color="var(--accent)">{item.reachability}</Tag>}
                  {item.tool_name && <Tag color="var(--text-dim)">{item.tool_name}</Tag>}
                  {item.created_at && <Tag color="var(--text-dim)">{timeAgo(item.created_at)}</Tag>}
                </div>

                <div style={{ color: "var(--text)", fontSize: 13, lineHeight: 1.6 }}>
                  <strong style={{ color: "var(--accent)" }}>Next action:</strong> {item.next_action}
                </div>

                {(item.identifiers?.length > 0 || item.evidence?.length > 0 || item.references?.length > 0 || item.html_url) && (
                  <div style={{ display: "grid", gap: 10 }}>
                    {item.identifiers?.length > 0 && (
                      <div>
                        <div style={S.label}>Identifiers</div>
                        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                          {item.identifiers.map((identifier) => (
                            <Tag key={identifier} color="var(--text-dim)">{identifier}</Tag>
                          ))}
                        </div>
                      </div>
                    )}
                    {item.evidence?.length > 0 && (
                      <div>
                        <div style={S.label}>Evidence</div>
                        <div style={{ display: "grid", gap: 6 }}>
                          {item.evidence.map((entry) => (
                            <div key={entry} style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.5 }}>
                              {entry}
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                    {item.references?.length > 0 && (
                      <div>
                        <div style={S.label}>References</div>
                        <div style={{ display: "grid", gap: 6 }}>
                          {item.references.map((ref) => (
                            <a
                              key={ref}
                              href={ref}
                              target="_blank"
                              rel="noreferrer"
                              style={{ color: "var(--blue)", fontSize: 12, textDecoration: "none", wordBreak: "break-all" }}
                            >
                              {ref}
                            </a>
                          ))}
                        </div>
                      </div>
                    )}
                    {item.html_url && (
                      <div>
                        <a
                          href={item.html_url}
                          target="_blank"
                          rel="noreferrer"
                          style={{ color: "var(--accent)", fontSize: 12, textDecoration: "none" }}
                        >
                          Open in GitHub
                        </a>
                      </div>
                    )}
                  </div>
                )}
              </div>
            ))
          )}
        </div>
      ) : (
        <EmptyState icon="🛡" text="Run a repo scan to turn open security findings into a ranked action queue." />
      )}
    </div>
  );
}

function Metric({ label, value, color }) {
  return (
    <div>
      <div style={S.label}>{label}</div>
      <div style={{ fontSize: 20, fontWeight: 700, color: color || "var(--text)" }}>{value}</div>
    </div>
  );
}
