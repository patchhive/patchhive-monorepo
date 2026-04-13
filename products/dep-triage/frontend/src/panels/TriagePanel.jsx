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
  { v: "stale", l: "Stalest first" },
  { v: "package", l: "Package name" },
];

const ALERT_OPTIONS = [
  { v: "yes", l: "Include alerts" },
  { v: "no", l: "PRs only" },
];

function recommendationColor(recommendation) {
  if (recommendation === "update_now") {
    return "var(--accent)";
  }
  if (recommendation === "watch") {
    return "var(--gold)";
  }
  return "var(--text-dim)";
}

function impactColor(impact) {
  if (impact === "runtime" || impact === "mixed") {
    return "var(--accent)";
  }
  if (impact === "ci") {
    return "var(--blue)";
  }
  if (impact === "tooling") {
    return "var(--green)";
  }
  return "var(--text-dim)";
}

function recommendationRank(recommendation) {
  if (recommendation === "update_now") {
    return 3;
  }
  if (recommendation === "watch") {
    return 2;
  }
  return 1;
}

function sortItems(items, sortBy) {
  return [...items].sort((left, right) => {
    if (sortBy === "recommendation") {
      return (
        recommendationRank(right.recommendation) - recommendationRank(left.recommendation) ||
        right.score - left.score ||
        right.stale_days - left.stale_days
      );
    }
    if (sortBy === "stale") {
      return right.stale_days - left.stale_days || right.score - left.score;
    }
    if (sortBy === "package") {
      return (
        left.package_name.localeCompare(right.package_name) ||
        right.score - left.score
      );
    }
    return (
      right.score - left.score ||
      recommendationRank(right.recommendation) - recommendationRank(left.recommendation) ||
      right.stale_days - left.stale_days
    );
  });
}

function buildScanMarkdown(scan) {
  const lines = [
    `# DepTriage scan for ${scan.repo}`,
    "",
    scan.summary,
    "",
    `- Tracked items: ${scan.metrics.tracked_items}`,
    `- Update now: ${scan.metrics.update_now}`,
    `- Watch: ${scan.metrics.watch}`,
    `- Ignore for now: ${scan.metrics.ignore_for_now}`,
    `- Dependency PRs: ${scan.metrics.dependency_pull_requests}`,
    `- Open alerts: ${scan.metrics.open_alerts}`,
  ];

  if (scan.items?.length) {
    lines.push("", "## Top queue", "");
    sortItems(scan.items, "risk")
      .slice(0, 8)
      .forEach((item) => {
        lines.push(
          `- [${item.recommendation.replace("_", " ")}] ${item.package_name} — ${item.summary}`
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
  onLoadScan,
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

  const sortedItems = scan?.items?.length ? sortItems(scan.items, sortBy) : [];

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
            <div style={{ fontSize: 18, fontWeight: 700 }}>Scan dependency churn</div>
            <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
              DepTriage reads open dependency PRs, optionally folds in Dependabot alerts, and sorts the noise into update now, watch, or ignore for now.
            </div>
          </div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <Tag color="var(--accent)">update now</Tag>
            <Tag color="var(--gold)">watch</Tag>
            <Tag color="var(--text-dim)">ignore for now</Tag>
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "minmax(280px, 2fr) minmax(120px, 1fr) minmax(160px, 1fr) auto", gap: 12, alignItems: "end" }}>
          <div>
            <div style={S.label}>Repository</div>
            <Input
              value={form.repo}
              onChange={(value) => setForm((prev) => ({ ...prev, repo: value }))}
              placeholder="owner/repo"
            />
          </div>
          <div>
            <div style={S.label}>PR limit</div>
            <Input
              value={form.pr_limit}
              onChange={(value) => setForm((prev) => ({ ...prev, pr_limit: value }))}
              placeholder="25"
            />
          </div>
          <div>
            <div style={S.label}>Alert mode</div>
            <Sel
              value={form.include_alerts}
              onChange={(value) => setForm((prev) => ({ ...prev, include_alerts: value }))}
              opts={ALERT_OPTIONS}
            />
          </div>
          <Btn onClick={onRun} disabled={running}>
            {running ? "Scanning..." : "Run DepTriage"}
          </Btn>
        </div>
      </div>

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
                  <Tag color="var(--gold)">{scan.metrics.dependency_pull_requests} dependency PRs</Tag>
                  <Tag color="var(--accent)">{scan.metrics.open_alerts} alerts</Tag>
                </div>
              </div>

              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                <Btn onClick={copySummary}>{copyState || "Copy summary"}</Btn>
                <Sel value={sortBy} onChange={setSortBy} opts={SORT_OPTIONS} style={{ minWidth: 150 }} />
              </div>
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: 12 }}>
              <Metric label="Tracked items" value={scan.metrics.tracked_items} />
              <Metric label="Update now" value={scan.metrics.update_now} color="var(--accent)" />
              <Metric label="Watch" value={scan.metrics.watch} color="var(--gold)" />
              <Metric label="Ignore" value={scan.metrics.ignore_for_now} color="var(--text-dim)" />
              <Metric label="Runtime-heavy" value={scan.metrics.runtime_updates} color="var(--accent)" />
              <Metric label="Major jumps" value={scan.metrics.major_updates} color="var(--gold)" />
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

          {sortedItems.length === 0 ? (
            <EmptyState icon="📦" text="This scan did not surface any dependency items that need triage." />
          ) : (
            sortedItems.map((item) => (
              <div key={item.key} style={{ ...S.panel, display: "grid", gap: 12, borderColor: recommendationColor(item.recommendation) }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "start" }}>
                  <div style={{ display: "grid", gap: 6 }}>
                    <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
                      <div style={{ fontSize: 16, fontWeight: 700 }}>{item.package_name}</div>
                      <ScoreBadge score={item.score} />
                    </div>
                    <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{item.summary}</div>
                  </div>
                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                    <Tag color={recommendationColor(item.recommendation)}>
                      {item.recommendation.replaceAll("_", " ")}
                    </Tag>
                    <Tag color="var(--blue)">{item.ecosystem || "unknown"}</Tag>
                    <Tag color={impactColor(item.runtime_impact)}>{item.runtime_impact || "unknown"}</Tag>
                    <Tag color="var(--text-dim)">{item.update_kind || "unknown"}</Tag>
                    <Tag color="var(--text-dim)">{item.stale_days}d stale</Tag>
                  </div>
                </div>

                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  {item.manifests.slice(0, 6).map((manifest) => (
                    <Tag key={manifest} color="var(--text-dim)">
                      {manifest}
                    </Tag>
                  ))}
                </div>

                <div style={{ display: "grid", gap: 6 }}>
                  <div style={S.label}>Why it landed here</div>
                  {item.reasons.map((reason) => (
                    <div key={reason} style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.5 }}>
                      • {reason}
                    </div>
                  ))}
                </div>

                {!!item.pull_requests.length && (
                  <div style={{ display: "grid", gap: 6 }}>
                    <div style={S.label}>Dependency pull requests</div>
                    {item.pull_requests.map((pr) => (
                      <a
                        key={pr.number}
                        href={pr.html_url}
                        target="_blank"
                        rel="noreferrer"
                        style={{ color: "var(--text)", textDecoration: "none", border: "1px solid var(--border)", borderRadius: 6, padding: "10px 12px", display: "grid", gap: 4 }}
                      >
                        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap" }}>
                          <div style={{ fontWeight: 700 }}>#{pr.number} {pr.title}</div>
                          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                            {pr.source_tool && <Tag color="var(--blue)">{pr.source_tool}</Tag>}
                            {pr.update_kind && <Tag color="var(--text-dim)">{pr.update_kind}</Tag>}
                            {pr.to_version && <Tag color="var(--text-dim)">{pr.from_version || "?"} → {pr.to_version}</Tag>}
                          </div>
                        </div>
                        <div style={{ color: "var(--text-dim)", fontSize: 11 }}>
                          {pr.author ? `@${pr.author}` : "unknown author"} · updated {timeAgo(pr.updated_at)}
                        </div>
                      </a>
                    ))}
                  </div>
                )}

                {!!item.alerts.length && (
                  <div style={{ display: "grid", gap: 6 }}>
                    <div style={S.label}>Security alerts</div>
                    {item.alerts.map((alert) => (
                      <a
                        key={alert.number}
                        href={alert.html_url}
                        target="_blank"
                        rel="noreferrer"
                        style={{ color: "var(--text)", textDecoration: "none", border: "1px solid var(--border)", borderRadius: 6, padding: "10px 12px", display: "grid", gap: 4 }}
                      >
                        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap" }}>
                          <div style={{ fontWeight: 700 }}>Alert #{alert.number}</div>
                          <Tag color={alert.severity === "critical" || alert.severity === "high" ? "var(--accent)" : "var(--gold)"}>
                            {alert.severity}
                          </Tag>
                        </div>
                        <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{alert.summary}</div>
                        {alert.first_patched_version && (
                          <div style={{ color: "var(--text-dim)", fontSize: 11 }}>
                            first patched: {alert.first_patched_version}
                          </div>
                        )}
                      </a>
                    ))}
                  </div>
                )}
              </div>
            ))
          )}
        </div>
      ) : overview ? (
        <div style={{ display: "grid", gap: 16 }}>
          <div style={{ ...S.panel, display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 12 }}>
            <Metric label="Stored scans" value={overview.counts.scans} />
            <Metric label="Repos seen" value={overview.counts.repos} />
            <Metric label="Tracked items" value={overview.counts.tracked_items} />
            <Metric label="Update now" value={overview.counts.update_now} color="var(--accent)" />
            <Metric label="Watch" value={overview.counts.watch} color="var(--gold)" />
            <Metric label="Ignore" value={overview.counts.ignore_for_now} color="var(--text-dim)" />
          </div>

          <div style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div style={{ fontSize: 16, fontWeight: 700 }}>Recent scans</div>
            {overview.recent_scans?.length ? (
              overview.recent_scans.map((item) => (
                <div key={item.id} style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center", border: "1px solid var(--border)", borderRadius: 6, padding: "10px 12px" }}>
                  <div style={{ display: "grid", gap: 4 }}>
                    <div style={{ fontWeight: 700 }}>{item.repo}</div>
                    <div style={{ color: "var(--text-dim)", fontSize: 12 }}>{item.summary}</div>
                  </div>
                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
                    <Tag color="var(--accent)">{item.update_now} now</Tag>
                    <Tag color="var(--gold)">{item.watch} watch</Tag>
                    <Tag color="var(--text-dim)">{timeAgo(item.created_at)}</Tag>
                    <Btn onClick={() => onLoadScan(item.id)}>Load scan</Btn>
                  </div>
                </div>
              ))
            ) : (
              <EmptyState icon="◎" text="Run DepTriage once and the saved queue history will show up here." />
            )}
          </div>
        </div>
      ) : (
        <EmptyState icon="…" text="DepTriage overview is loading." />
      )}
    </div>
  );
}

function Metric({ label, value, color }) {
  return (
    <div>
      <div style={S.label}>{label}</div>
      <div style={{ fontSize: 18, fontWeight: 700, color: color || "var(--text)" }}>{value}</div>
    </div>
  );
}
