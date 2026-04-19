import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import {
  Btn,
  EmptyState,
  Input,
  S,
  ScoreBadge,
  Tag,
  timeAgo,
} from "@patchhivehq/ui";

function buildScanMarkdown(scan) {
  const lines = [
    `# RefactorScout scan for ${scan.repo_name || scan.repo_path}`,
    "",
    scan.summary,
    "",
    `- Repo path: ${scan.repo_path}`,
    `- Files scanned: ${scan.metrics.files_scanned}`,
    `- Files skipped: ${scan.metrics.files_skipped}`,
    `- Opportunities: ${scan.metrics.opportunities}`,
    `- High-safety: ${scan.metrics.high_safety}`,
    `- Medium-safety: ${scan.metrics.medium_safety}`,
  ];

  if (scan.opportunities?.length) {
    lines.push("", "## Ranked opportunities", "");
    scan.opportunities.slice(0, 10).forEach((opportunity, index) => {
      lines.push(
        `${index + 1}. [${opportunity.kind}] ${opportunity.title} — ${opportunity.summary}`
      );
    });
  }

  if (scan.warnings?.length) {
    lines.push("", "## Warnings", "");
    scan.warnings.forEach((warning) => lines.push(`- ${warning}`));
  }

  return lines.join("\n");
}

export default function ScoutPanel({
  apiKey,
  form,
  setForm,
  running,
  onRun,
  scan,
}) {
  const [overview, setOverview] = useState(null);
  const [copyState, setCopyState] = useState("");
  const fetch_ = createApiFetcher(apiKey);

  useEffect(() => {
    fetch_(`${API}/overview`)
      .then((res) => res.json())
      .then(setOverview)
      .catch(() => setOverview(null));
  }, [apiKey, scan?.id]);

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
            <div style={{ fontSize: 18, fontWeight: 700 }}>Scout local refactor leads</div>
            <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>
              RefactorScout walks a local repo, ranks safe structural cleanup leads, and gives you a short queue to chip away at without changing behavior first.
            </div>
          </div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <Tag color="var(--green)">high safety</Tag>
            <Tag color="var(--gold)">read only</Tag>
            <Tag color="var(--accent)">local repo scan</Tag>
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "minmax(320px, 2fr) minmax(120px, 180px) auto", gap: 12, alignItems: "end" }}>
          <div>
            <div style={S.label}>Repository path</div>
            <Input
              value={form.repo_path}
              onChange={(value) => setForm((prev) => ({ ...prev, repo_path: value }))}
              placeholder="/home/you/code/project"
            />
          </div>
          <div>
            <div style={S.label}>Max files</div>
            <Input
              value={form.max_files}
              onChange={(value) => setForm((prev) => ({ ...prev, max_files: value }))}
              placeholder="250"
            />
          </div>
          <Btn onClick={onRun} disabled={running}>
            {running ? "Scanning..." : "Run RefactorScout"}
          </Btn>
        </div>
      </div>

      {overview && (
        <div style={{ ...S.panel, display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(170px, 1fr))", gap: 12 }}>
          <Stat label="Stored scans" value={overview.scan_count} />
          <Stat label="Repos seen" value={overview.repo_count} />
          <Stat label="Opportunities" value={overview.opportunity_count} />
          <Stat label="High-safety leads" value={overview.high_safety_count} color="var(--green)" />
          <Stat label="Large files" value={overview.large_file_count} />
          <Stat label="Long functions" value={overview.long_function_count} />
          <Stat label="Repeated literals" value={overview.repeated_literal_count} />
          <div>
            <div style={S.label}>Allowed roots</div>
            <div style={{ fontSize: 12, color: "var(--text-dim)", lineHeight: 1.6 }}>
              {overview.allowed_roots?.length
                ? overview.allowed_roots.join(" · ")
                : "No roots configured"}
            </div>
          </div>
        </div>
      )}

      {!scan ? (
        <EmptyState
          icon="🧭"
          text="Point RefactorScout at a local repo path under the configured allowed roots to get a ranked queue of refactor leads."
        />
      ) : (
        <div style={{ display: "grid", gap: 16 }}>
          <div style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "start" }}>
              <div style={{ display: "grid", gap: 8 }}>
                <div style={{ fontSize: 18, fontWeight: 700 }}>
                  {scan.repo_name || scan.repo_path}
                </div>
                <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>
                  {scan.summary}
                </div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  <Tag color="var(--blue)">{scan.repo_path}</Tag>
                  <Tag color="var(--text-dim)">{timeAgo(scan.created_at)}</Tag>
                  <Tag color="var(--green)">{scan.metrics.high_safety} high safety</Tag>
                  <Tag color="var(--accent)">{scan.metrics.medium_safety} medium safety</Tag>
                </div>
              </div>

              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                <Btn onClick={copySummary}>{copyState || "Copy summary"}</Btn>
              </div>
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(150px, 1fr))", gap: 12 }}>
              <Stat label="Files scanned" value={scan.metrics.files_scanned} />
              <Stat label="Files skipped" value={scan.metrics.files_skipped} />
              <Stat label="Opportunities" value={scan.metrics.opportunities} />
              <Stat label="Large files" value={scan.metrics.large_file_count} />
              <Stat label="Long functions" value={scan.metrics.long_function_count} />
              <Stat label="Repeated literals" value={scan.metrics.repeated_literal_count} />
            </div>

            {scan.warnings?.length > 0 && (
              <div style={{ display: "grid", gap: 8 }}>
                <div style={S.label}>Warnings</div>
                {scan.warnings.map((warning, index) => (
                  <div
                    key={`${warning}-${index}`}
                    style={{
                      border: "1px solid var(--gold)44",
                      background: "var(--gold)10",
                      color: "var(--text)",
                      borderRadius: 6,
                      padding: "10px 12px",
                      fontSize: 12,
                      lineHeight: 1.5,
                    }}
                  >
                    {warning}
                  </div>
                ))}
              </div>
            )}
          </div>

          {scan.opportunities?.length === 0 ? (
            <EmptyState
              icon="◌"
              text="This scan finished cleanly, but the current heuristics did not find a clear low-risk refactor queue."
            />
          ) : (
            scan.opportunities.map((opportunity) => (
              <div
                key={opportunity.id}
                style={{ ...S.panel, display: "grid", gap: 12 }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "start" }}>
                  <div style={{ display: "grid", gap: 6 }}>
                    <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                      <div style={{ fontSize: 16, fontWeight: 700 }}>{opportunity.title}</div>
                      <ScoreBadge score={opportunity.score} />
                    </div>
                    <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>
                      {opportunity.summary}
                    </div>
                  </div>
                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                    <Tag color={opportunity.safety === "high" ? "var(--green)" : "var(--gold)"}>
                      {opportunity.safety} safety
                    </Tag>
                    <Tag color="var(--text-dim)">{opportunity.effort} effort</Tag>
                    <Tag color="var(--accent)">{opportunity.kind.replaceAll("_", " ")}</Tag>
                    <Tag color="var(--blue)">{opportunity.language}</Tag>
                  </div>
                </div>

                <div style={{ display: "grid", gap: 6 }}>
                  <div style={S.label}>Location</div>
                  <div style={{ fontSize: 12, color: "var(--text)" }}>
                    {opportunity.path}
                    {opportunity.line_start > 0 && (
                      <span style={{ color: "var(--text-dim)" }}>
                        {" "}
                        · lines {opportunity.line_start}-{opportunity.line_end}
                      </span>
                    )}
                  </div>
                </div>

                <div style={{ display: "grid", gap: 6 }}>
                  <div style={S.label}>Suggested first move</div>
                  <div style={{ fontSize: 12, color: "var(--text)", lineHeight: 1.6 }}>
                    {opportunity.suggestion}
                  </div>
                </div>

                {opportunity.evidence?.length > 0 && (
                  <div style={{ display: "grid", gap: 6 }}>
                    <div style={S.label}>Evidence</div>
                    <div style={{ display: "grid", gap: 4 }}>
                      {opportunity.evidence.map((item) => (
                        <div key={item} style={{ fontSize: 12, color: "var(--text-dim)" }}>
                          - {item}
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}

function Stat({ label, value, color }) {
  return (
    <div>
      <div style={S.label}>{label}</div>
      <div style={{ fontSize: 18, fontWeight: 700, color: color || "var(--text)" }}>
        {value}
      </div>
    </div>
  );
}
