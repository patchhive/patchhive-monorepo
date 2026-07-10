import { useEffect, useMemo, useState } from "react";
import { createApiFetcher, useApiKeyAuth, useProductRuntime } from "@patchhivehq/product-shell/auth";
import {
  DeckBar,
  HistoryDetailGrid,
  MetricBand,
  Panel,
  ProductV2AuthGate,
  ProductV2Shell,
  ProductRail,
  SuiteRadar,
  SuiteTopline,
  humanizeToken,
  radarWindowFromTimestamp,
  usePersistentProductTab,
} from "@patchhivehq/ui-v2";
import { API } from "./config.js";

const TABS = [
  { id: "triage", label: "Triage" },
  { id: "history", label: "Scan history" },
  { id: "checks", label: "Checks" },
];

const POSITIONS = [
  { left: "69%", top: "33%" },
  { left: "57%", top: "70%" },
  { left: "36%", top: "31%" },
  { left: "29%", top: "67%" },
  { left: "76%", top: "58%" },
  { left: "48%", top: "45%" },
  { left: "62%", top: "24%" },
  { left: "42%", top: "76%" },
];

const RADAR_WINDOWS = {
  7: { label: "7 day live pass", readoutLabel: "7 day window", outer: "7d", mid: "3d", inner: "24h" },
  14: { label: "14 day history pass", readoutLabel: "14 day window", outer: "14d", mid: "7d", inner: "3d" },
  30: { label: "30 day deep sweep", readoutLabel: "30 day window", outer: "30d", mid: "14d", inner: "7d" },
};

const DEFAULT_FORM = {
  repo: "",
  include_code_scanning: true,
  include_dependency_alerts: true,
};

const SECURITY_SCOPE_HINT = "Fine-grained PAT: select this repository and grant Metadata read, Code scanning alerts read, Dependabot alerts read; the token owner must have security-alert access. Classic PAT: security_events, or public_repo for public-only scans.";

function asCount(value) {
  const number = Number(value || 0);
  return Number.isFinite(number) ? number : 0;
}

function runtimeScoped(metrics = {}) {
  return asCount(metrics.runtime_scoped ?? metrics.runtime_exposed);
}

function timeAgo(value) {
  if (!value) return "never";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return String(value);
  const seconds = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 48) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

function githubReady(health) {
  return Boolean(health?.github_ready || health?.github?.token_configured);
}

function findingTone(finding = {}) {
  const recommendation = String(finding.recommendation || "").toLowerCase();
  const severity = String(finding.severity || "").toLowerCase();
  if (recommendation.includes("fix") || recommendation.includes("now") || severity === "critical" || severity === "high") return "red";
  if (recommendation.includes("plan") || severity === "medium") return "amber";
  if (recommendation.includes("watch") || recommendation.includes("defer") || severity === "low") return "green";
  return "signal";
}

function recommendationLabel(value) {
  return humanizeToken(value, "watch");
}

function severityRank(value) {
  const severity = String(value || "").toLowerCase();
  if (severity === "critical") return 5;
  if (severity === "high") return 4;
  if (severity === "medium" || severity === "moderate") return 3;
  if (severity === "low") return 2;
  return severity ? 1 : 0;
}

function recommendationRank(value) {
  const recommendation = String(value || "").toLowerCase();
  if (recommendation.includes("fix") || recommendation.includes("now")) return 3;
  if (recommendation.includes("plan")) return 2;
  if (recommendation.includes("watch")) return 1;
  return 0;
}

function strongerRecommendation(left, right) {
  return recommendationRank(left) >= recommendationRank(right) ? left : right;
}

function strongerSeverity(left, right) {
  return severityRank(left) >= severityRank(right) ? left : right;
}

function groupDependencyFindings(findings = []) {
  const groups = new Map();
  const directFindings = [];

  for (const finding of findings) {
    if (finding.source !== "dependency_alert") {
      directFindings.push({ type: "finding", finding });
      continue;
    }

    const packageName = finding.package_name || finding.title || "dependency";
    const ecosystem = finding.ecosystem || "unknown";
    const manifest = finding.location || "manifest";
    const key = `${packageName.toLowerCase()}::${ecosystem.toLowerCase()}::${manifest.toLowerCase()}`;
    const existing = groups.get(key) || {
      type: "dependency-group",
      key,
      packageName,
      ecosystem,
      manifest,
      findings: [],
      fixNow: 0,
      planNext: 0,
      watch: 0,
      highestSeverity: finding.severity || "unknown",
      recommendation: finding.recommendation || "watch",
      maxScore: 0,
      runtimeExposed: 0,
      patchedVersions: new Set(),
    };

    existing.findings.push(finding);
    existing.maxScore = Math.max(existing.maxScore, asCount(finding.score));
    existing.highestSeverity = strongerSeverity(existing.highestSeverity, finding.severity);
    existing.recommendation = strongerRecommendation(existing.recommendation, finding.recommendation);
    if (String(finding.recommendation || "").includes("fix")) existing.fixNow += 1;
    else if (String(finding.recommendation || "").includes("plan")) existing.planNext += 1;
    else existing.watch += 1;
    if (String(finding.reachability || "").toLowerCase().includes("runtime")) {
      existing.runtimeExposed += 1;
    }
    for (const evidence of finding.evidence || []) {
      const match = String(evidence).match(/first patched version\s+(.+)/i);
      if (match?.[1]) existing.patchedVersions.add(match[1].trim());
    }
    groups.set(key, existing);
  }

  const grouped = Array.from(groups.values()).map((group) => ({
    ...group,
    patchedVersions: Array.from(group.patchedVersions),
    findings: group.findings.sort((left, right) => (
      asCount(right.score) - asCount(left.score)
      || severityRank(right.severity) - severityRank(left.severity)
      || String(left.summary || left.title).localeCompare(String(right.summary || right.title))
    )),
  }));

  return [...grouped, ...directFindings].sort((left, right) => {
    const leftScore = left.type === "dependency-group" ? left.maxScore : asCount(left.finding?.score);
    const rightScore = right.type === "dependency-group" ? right.maxScore : asCount(right.finding?.score);
    const leftRecommendation = left.type === "dependency-group" ? left.recommendation : left.finding?.recommendation;
    const rightRecommendation = right.type === "dependency-group" ? right.recommendation : right.finding?.recommendation;
    const leftSeverity = left.type === "dependency-group" ? left.highestSeverity : left.finding?.severity;
    const rightSeverity = right.type === "dependency-group" ? right.highestSeverity : right.finding?.severity;
    return recommendationRank(rightRecommendation) - recommendationRank(leftRecommendation)
      || rightScore - leftScore
      || severityRank(rightSeverity) - severityRank(leftSeverity);
  });
}

function patchedVersionSummary(group) {
  const versions = group?.patchedVersions || [];
  if (!versions.length) return "";
  if (versions.length === 1) return ` · fixed in ${versions[0]}`;
  return ` · fixes vary across ${versions.length} advisories`;
}

function uniqueValues(values = []) {
  return Array.from(new Set(values.filter(Boolean)));
}

function findingLinks(findings = []) {
  const links = [];
  for (const finding of findings) {
    if (finding.html_url) {
      links.push({ href: finding.html_url, label: "GitHub alert" });
    }
    for (const reference of finding.references || []) {
      links.push({ href: reference, label: "Advisory reference" });
    }
  }
  const seen = new Set();
  return links.filter((link) => {
    if (!link.href || seen.has(link.href)) return false;
    seen.add(link.href);
    return true;
  });
}

function buildScanMarkdown(scan) {
  if (!scan) return "";
  const lines = [
    `# VulnTriage scan for ${scan.repo}`,
    "",
    scanSummary(scan),
    "",
    `- Tracked findings: ${asCount(scan.metrics?.tracked_findings)}`,
    `- Fix now: ${asCount(scan.metrics?.fix_now)}`,
    `- Plan next: ${asCount(scan.metrics?.plan_next)}`,
    `- Watch: ${asCount(scan.metrics?.watch)}`,
    `- Code scanning alerts: ${asCount(scan.metrics?.code_scanning_alerts)}`,
    `- Dependabot alerts: ${asCount(scan.metrics?.dependency_alerts)}`,
    `- Runtime scoped (heuristic): ${runtimeScoped(scan.metrics)}`,
    `- Owner scoped: ${asCount(scan.metrics?.owner_scoped)}`,
  ];

  const groupedFindings = groupDependencyFindings(scan.findings || []);
  if (groupedFindings.length) {
    lines.push("", "## Top remediation groups", "");
    groupedFindings.slice(0, 8).forEach((item) => {
      if (item.type === "dependency-group") {
        const top = item.findings[0] || {};
        lines.push(
          `- [${recommendationLabel(item.recommendation)}] ${item.packageName} / ${item.manifest} — ${item.findings.length} alerts — ${top.summary || top.title || "security advisory"}`,
        );
        return;
      }
      const finding = item.finding || {};
      lines.push(
        `- [${recommendationLabel(finding.recommendation)}] ${finding.title || finding.key} — ${finding.summary || finding.next_action || finding.location || "security finding"}`,
      );
    });
  }

  if (scan.warnings?.length) {
    lines.push("", "## Warnings", "");
    scan.warnings.forEach((warning) => lines.push(`- ${warningLabel(warning)}`));
  }

  return lines.join("\n");
}

function warningLabel(warning) {
  const value = String(warning || "");
  if (value.includes("BOT_GITHUB_TOKEN is not set") || value.includes("GITHUB_TOKEN is not set")) {
    return "Security feeds were skipped because GitHub token access is not configured.";
  }
  if (value.includes("/code-scanning/alerts") && value.includes("403 Forbidden")) {
    return `Code scanning alerts could not be read. ${SECURITY_SCOPE_HINT}`;
  }
  if (value.includes("/dependabot/alerts") && value.includes("Dependabot alerts are disabled")) {
    return "Dependabot alerts are disabled for this repository, so dependency vulnerability pressure is unavailable.";
  }
  if (value.includes("/dependabot/alerts") && value.includes("403 Forbidden")) {
    return `Dependabot alerts could not be read. ${SECURITY_SCOPE_HINT}`;
  }
  return value;
}

function warningFeatureDisabled(warning) {
  const value = String(warning || "");
  const lower = value.toLowerCase();
  return value.includes("[feature_disabled]")
    || lower.includes("dependabot alerts are disabled")
    || lower.includes("code scanning is not enabled");
}

function warningNeedsSecurityAccess(warning) {
  const value = String(warning || "");
  const lower = value.toLowerCase();
  if (warningFeatureDisabled(value)) return false;
  return value.includes("[missing_token_scope]")
    || value.includes("[forbidden]")
    || lower.includes("403 forbidden")
    || lower.includes("resource not accessible");
}

function warningMissingSecurityToken(warning) {
  const value = String(warning || "");
  return value.includes("BOT_GITHUB_TOKEN is not set") || value.includes("GITHUB_TOKEN is not set");
}

function scanSummary(scan) {
  const repo = scan?.repo || "this repository";
  const hasFindings = asCount(scan?.metrics?.tracked_findings) > 0 || Boolean(scan?.findings?.length);
  const warnings = scan?.warnings || [];
  const needsSecurityAccess = warnings.some(warningNeedsSecurityAccess);
  const featureDisabled = warnings.some(warningFeatureDisabled);
  if (!hasFindings && needsSecurityAccess && featureDisabled) {
    return `VulnTriage could not fully read GitHub security alerts for \`${repo}\` because one feed needs token access and another feed is disabled for this repository. Grant the missing security-feed permission (${SECURITY_SCOPE_HINT}) and enable the disabled feed, then rerun the scan.`;
  }
  if (!hasFindings && needsSecurityAccess) {
    return `VulnTriage could not read GitHub security alerts for \`${repo}\` with the current token. Grant security-feed access (${SECURITY_SCOPE_HINT}), then rerun the scan.`;
  }
  if (!hasFindings && warnings.some(warningMissingSecurityToken)) {
    return `VulnTriage could not read GitHub security alerts for \`${repo}\` because security-feed token access is not configured.`;
  }
  if (!hasFindings && featureDisabled) {
    return `VulnTriage could not read one or more GitHub security feeds for \`${repo}\` because the feature is disabled for this repository. Enable code scanning or Dependabot alerts, or use a future public vulnerability fallback.`;
  }
  if (!hasFindings && warnings.length) {
    return `VulnTriage did not receive actionable findings from the readable feeds for \`${repo}\`, but one or more GitHub security feeds could not be read.`;
  }
  return scan?.summary || "Saved VulnTriage scan.";
}

function metricTone(value, hotTone = "hot") {
  return asCount(value) ? hotTone : "ok";
}

function historyMetrics(item = {}) {
  return {
    code_scanning_alerts: item.code_scanning_alerts,
    dependency_alerts: item.dependency_alerts,
    fix_now: item.fix_now,
    owner_scoped: item.owner_scoped,
    plan_next: item.plan_next,
    runtime_scoped: item.runtime_scoped ?? item.runtime_exposed,
    tracked_findings: item.tracked_findings,
    watch: item.watch,
  };
}

function activeScanSource(scan, history) {
  if (scan) {
    return {
      created_at: scan.created_at,
      detailLoaded: true,
      metrics: scan.metrics || {},
      repo: scan.repo,
    };
  }
  const latest = history[0] || {};
  if (latest.id || latest.repo) {
    return {
      created_at: latest.created_at,
      detailLoaded: false,
      metrics: historyMetrics(latest),
      repo: latest.repo,
    };
  }
  return {
    created_at: "",
    detailLoaded: false,
    metrics: {},
    repo: "",
  };
}

async function parseJsonResponse(response, fallbackError) {
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(data.error || data.message || fallbackError);
  }
  return data;
}

function buildTopline(health, overview, scan, history) {
  const counts = overview?.counts || {};
  const active = activeScanSource(scan, history);
  const fixNow = active.metrics.fix_now ?? counts.fix_now ?? health?.fix_now_count;
  return [
    { label: "VulnTriage", value: "Security queue", tone: "sig" },
    { label: "System", value: health?.status || "checking", tone: health?.status === "ok" ? "ok" : "warn" },
    { label: "Mode", value: "Read only" },
    { label: "GitHub", value: githubReady(health) ? "Security read" : "token missing", tone: githubReady(health) ? "sig" : "warn" },
    { label: "Fix now", value: `${asCount(fixNow)} active`, tone: asCount(fixNow) ? "warn" : "ok" },
    { label: "Last scan", value: active.created_at ? timeAgo(active.created_at) : counts.scans ? "loaded" : "none" },
  ];
}

function buildMetrics(scan, overview, health) {
  if (scan) {
    const metrics = scan.metrics || {};
    return [
      { label: "Fix now", value: String(asCount(metrics.fix_now)), tone: metricTone(metrics.fix_now), sub: "highest urgency" },
      { label: "Plan next", value: String(asCount(metrics.plan_next)), tone: metricTone(metrics.plan_next, "warn"), sub: "owner follow-up" },
      { label: "Watch", value: String(asCount(metrics.watch)), tone: "ok", sub: "low exposure" },
      { label: "Runtime scoped", value: String(runtimeScoped(metrics)), tone: metricTone(runtimeScoped(metrics), "warn"), sub: "scope heuristic, not exploit proof" },
      { label: "Tracked", value: String(asCount(metrics.tracked_findings)), tone: "sig", sub: `${asCount(metrics.code_scanning_alerts)} code / ${asCount(metrics.dependency_alerts)} deps` },
    ];
  }
  const counts = overview?.counts || {};
  return [
    { label: "Scans", value: String(asCount(counts.scans || health?.scan_count)), tone: "sig", sub: `${asCount(counts.repos || health?.repo_count)} repos` },
    { label: "Fix now", value: String(asCount(counts.fix_now || health?.fix_now_count)), tone: metricTone(counts.fix_now || health?.fix_now_count), sub: "saved scans" },
    { label: "Plan next", value: String(asCount(counts.plan_next || health?.plan_next_count)), tone: metricTone(counts.plan_next || health?.plan_next_count, "warn"), sub: "saved scans" },
    { label: "Watch", value: String(asCount(counts.watch || health?.watch_count)), tone: "ok", sub: "saved scans" },
    { label: "Tracked", value: String(asCount(counts.tracked_findings || health?.tracked_finding_count)), tone: "sig", sub: "findings" },
  ];
}

function buildRail(scan, history, overview, health) {
  const active = activeScanSource(scan, history);
  const metrics = active.metrics || {};
  return {
    sections: [
      {
        title: "Feeds",
        items: [
          { label: "Code scanning", active: true, badge: String(asCount(metrics.code_scanning_alerts)), badgeTone: "signal" },
          { label: "Dependabot alerts", badge: String(asCount(metrics.dependency_alerts)), badgeTone: "amber" },
          { label: "Owner scoped", badge: String(asCount(metrics.owner_scoped)), badgeTone: "signal" },
          { label: "Runtime scoped", badge: String(runtimeScoped(metrics)), badgeTone: runtimeScoped(metrics) ? "red" : "green" },
        ],
      },
      {
        title: "Buckets",
        items: [
          { label: "fix now", active: true, badge: String(asCount(metrics.fix_now ?? overview?.counts?.fix_now)), badgeTone: "red" },
          { label: "plan next", badge: String(asCount(metrics.plan_next ?? overview?.counts?.plan_next)), badgeTone: "amber" },
          { label: "watch", badge: String(asCount(metrics.watch ?? overview?.counts?.watch)), badgeTone: "green" },
          { label: "saved scans", badge: String(history.length), badgeTone: "signal" },
        ],
      },
    ],
    stats: {
      title: "Active repo",
      items: [
        { label: "Repository", value: active.repo || "none" },
        { label: "Decision", value: metrics.fix_now ? "HOLD" : metrics.plan_next ? "PLAN" : "READY", large: true, tone: metrics.fix_now ? "hot" : metrics.plan_next ? "warn" : "ok" },
        { label: "Findings", value: String(asCount(metrics.tracked_findings)) },
      ],
    },
  };
}

function buildRadarItems(scan, history) {
  if (scan?.findings?.length) {
    return scan.findings.slice(0, 8).map((finding, index) => {
      const tone = findingTone(finding);
      return {
        detail: finding.location || finding.package_name || finding.source || finding.title,
        gain: recommendationLabel(finding.recommendation),
        gainMeta: `${asCount(finding.score)} score`,
        id: finding.key || `finding-${index + 1}`,
        label: finding.source || finding.severity || `V${index + 1}`,
        minWindow: index < 3 ? 7 : index < 6 ? 14 : 30,
        position: POSITIONS[index % POSITIONS.length],
        stats: [
          { label: "Severity", value: finding.severity || "unknown" },
          { label: "Decision", value: recommendationLabel(finding.recommendation) },
          { label: "Score", value: String(asCount(finding.score)) },
          { label: "Source", value: humanizeToken(finding.source || finding.tool_name, "github") },
          { label: "Owner", value: finding.owner_hint || "unrouted" },
        ],
        summary: finding.summary || finding.evidence?.[0] || finding.next_action || "Vulnerability triage finding.",
        title: finding.title || finding.package_name || finding.key || `Finding ${index + 1}`,
        tone,
        vector: humanizeToken(finding.reachability || finding.ecosystem || finding.source, "security"),
        vectorTone: tone === "red" || tone === "amber" ? "warn" : "",
      };
    });
  }
  if (history.length) {
    return history.map((item, index) => {
      const minWindow = radarWindowFromTimestamp(item.created_at);
      if (!minWindow) {
        return null;
      }
      const tracked = asCount(item.tracked_findings);
      return {
        detail: item.repo,
        gain: item.fix_now ? "fix now" : item.plan_next ? "plan next" : tracked ? "watch" : "clear",
        gainMeta: `${tracked} findings`,
        id: item.id || `history-${index + 1}`,
        label: item.repo?.split("/").pop() || `S${index + 1}`,
        minWindow,
        position: POSITIONS[index % POSITIONS.length],
        stats: [
          { label: "Repo", value: item.repo },
          { label: "Tracked", value: String(asCount(item.tracked_findings)) },
          { label: "Fix", value: String(asCount(item.fix_now)) },
          { label: "Plan", value: String(asCount(item.plan_next)) },
          { label: "Age", value: timeAgo(item.created_at) },
        ],
        summary: scan?.id === item.id ? scanSummary(scan) : item.summary || "Saved VulnTriage scan.",
        title: item.repo,
        tone: item.fix_now ? "red" : item.plan_next ? "amber" : "green",
        vector: "saved",
        vectorTone: item.fix_now || item.plan_next ? "warn" : "",
      };
    }).filter(Boolean);
  }
  return [{
    detail: "No security scan yet",
    gain: "standby",
    gainMeta: "GitHub repo",
    id: "vuln-triage-ready",
    label: "VT",
    position: { left: "50%", top: "44%" },
    stats: [
      { label: "Mode", value: "read only" },
      { label: "Code scan", value: "optional" },
      { label: "Dependabot", value: "optional" },
      { label: "History", value: "empty" },
      { label: "Action", value: "scan" },
    ],
    summary: "Scan a repository to rank security findings into fix, plan, and watch decisions.",
    title: "VulnTriage ready",
    tone: "signal",
    vector: "READY",
  }];
}

function buildRadarFeed(scan, history, health) {
  if (scan) {
    const hasFindings = Boolean(scan.findings?.length);
    const firstWarning = warningLabel(scan.warnings?.[0]);
    return [
      hasFindings ? { text: scanSummary(scan), tone: scan.metrics?.fix_now ? "red" : scan.metrics?.plan_next ? "amber" : "green" } : null,
      { text: `${asCount(scan.metrics?.fix_now)} fix-now and ${asCount(scan.metrics?.plan_next)} plan-next findings are active.`, tone: scan.metrics?.fix_now ? "red" : scan.metrics?.plan_next ? "amber" : "green" },
      firstWarning ? { text: firstWarning, tone: "amber" } : null,
      hasFindings && !firstWarning ? { text: "Security alerts are ranked into engineering decisions.", tone: "signal" } : null,
    ].filter(Boolean);
  }
  return [
    { text: history.length ? `${history.length} saved security scans are available.` : "VulnTriage is waiting for a repository scan.", tone: "signal" },
    { text: githubReady(health) ? "GitHub token is ready for security reads." : "Add a GitHub token to scan code scanning and Dependabot security alerts.", tone: githubReady(health) ? "green" : "amber" },
    { text: "The radar fills with vulnerability findings once a scan completes.", tone: "signal" },
  ];
}

function historyRadarWindows(historyCount) {
  if (!historyCount) return RADAR_WINDOWS;
  const count = `${historyCount} saved`;
  return Object.fromEntries(
    Object.entries(RADAR_WINDOWS).map(([days, window]) => [days, { ...window, count }]),
  );
}

function StatusBanner({ tone = "signal", children }) {
  if (!children) return null;
  return <div className={`status-banner ${tone}`}>{children}</div>;
}

function VulnerabilityMap({ health, history, scan }) {
  const items = useMemo(() => buildRadarItems(scan, history), [scan, history]);
  const feed = useMemo(() => buildRadarFeed(scan, history, health), [scan, history, health]);
  const hasFindings = Boolean(scan?.findings?.length);
  const savedScanCount = hasFindings ? 0 : history.length;
  const windows = useMemo(() => historyRadarWindows(savedScanCount), [savedScanCount]);
  return (
    <SuiteRadar
      ariaLabel="VulnTriage vulnerability pressure radar"
      detailLabel={hasFindings ? "Triage reason" : "Scan target"}
      feed={feed}
      gainLabel={hasFindings ? "Decision" : "Scan state"}
      itemQueryParam="finding"
      items={items}
      signalLabel={hasFindings ? "findings" : "saved scans"}
      vectorLabel={hasFindings ? "Selected finding" : "Selected scan"}
      windows={windows}
    />
  );
}

function ScanForm({ error, form, health, onChange, onRun, running }) {
  const securityFeedsReady = githubReady(health);
  return (
    <Panel eyebrow="Scan" title="GitHub security intake" action={<span className="chip signal">read only</span>}>
      <form
        className="panelbody control-stack"
        onSubmit={(event) => {
          event.preventDefault();
          onRun();
        }}
      >
        <div className="form-grid">
          <label className="v2-field">
            Repository
            <input className="v2-input" onChange={(event) => onChange((current) => ({ ...current, repo: event.target.value }))} placeholder="owner/repo" value={form.repo} />
          </label>
          <label className="rowline" style={{ alignItems: "flex-start", justifyContent: "flex-start" }}>
            <input
              checked={securityFeedsReady && Boolean(form.include_code_scanning)}
              disabled={!securityFeedsReady}
              onChange={(event) => onChange((current) => ({ ...current, include_code_scanning: securityFeedsReady && event.target.checked }))}
              style={{ marginTop: 3 }}
              type="checkbox"
            />
            <span>
              <span className="repo-name" style={{ display: "block", fontSize: "0.8rem" }}>Code scanning</span>
              <span className="feed-meta">{securityFeedsReady ? "Include GitHub code scanning alerts." : "Add a GitHub token to read code scanning alerts."}</span>
            </span>
          </label>
          <label className="rowline" style={{ alignItems: "flex-start", justifyContent: "flex-start" }}>
            <input
              checked={securityFeedsReady && Boolean(form.include_dependency_alerts)}
              disabled={!securityFeedsReady}
              onChange={(event) => onChange((current) => ({ ...current, include_dependency_alerts: securityFeedsReady && event.target.checked }))}
              style={{ marginTop: 3 }}
              type="checkbox"
            />
            <span>
              <span className="repo-name" style={{ display: "block", fontSize: "0.8rem" }}>Dependabot alerts</span>
              <span className="feed-meta">{securityFeedsReady ? "Include dependency vulnerability alerts." : "Add a GitHub token to read dependency vulnerability alerts."}</span>
            </span>
          </label>
          <div className="v2-field">
            Action
            <button className="btn primary" disabled={running || !form.repo.trim()} type="submit">
              {running ? "Scanning..." : securityFeedsReady ? "Scan findings" : "Save empty scan"}
            </button>
          </div>
        </div>
        {error && <StatusBanner tone="red">{error}</StatusBanner>}
      </form>
    </Panel>
  );
}

function FixQueuePanel({ history, onLoadScan, scan }) {
  if (scan) {
    const groupedFindings = groupDependencyFindings(scan.findings || []);
    const dependencyGroupCount = groupedFindings.filter((item) => item.type === "dependency-group").length;
    const directFindingCount = groupedFindings.filter((item) => item.type === "finding").length;
    return (
      <Panel
        eyebrow="Queue"
        title="Remediation groups"
        action={<span className="chip red">{asCount(scan.metrics?.fix_now)} fix</span>}
      >
        <div className="panelbody repo-list queue-grid">
          {groupedFindings.length ? (
            <>
              <div className="feed-meta" style={{ marginBottom: 8 }}>
                {dependencyGroupCount ? `${dependencyGroupCount} dependency remediation group${dependencyGroupCount === 1 ? "" : "s"}` : "No dependency groups"}
                {directFindingCount ? ` · ${directFindingCount} code scanning finding${directFindingCount === 1 ? "" : "s"}` : ""}
              </div>
              {groupedFindings.slice(0, 8).map((item, index) => {
                if (item.type === "dependency-group") {
                  const tone = item.fixNow ? "red" : item.planNext ? "amber" : "green";
                  const top = item.findings[0] || {};
                  const alertCount = item.findings.length;
                  const patched = patchedVersionSummary(item);
                  const links = findingLinks(item.findings).slice(0, 3);
                  const identifiers = uniqueValues(item.findings.flatMap((finding) => finding.identifiers || [])).slice(0, 4);
                  const evidence = uniqueValues(item.findings.flatMap((finding) => finding.evidence || [])).slice(0, 2);
                  return (
                    <div className="ledger-row" key={item.key}>
                      <div className="rank">{String(index + 1).padStart(2, "0")}</div>
                      <div>
                        <div className="repo-name">{item.packageName} / {item.manifest}</div>
                        <div className="feed-meta">
                          {alertCount} Dependabot alert{alertCount === 1 ? "" : "s"} collapsed into one package decision. Top: {top.summary || top.title || "security advisory"}{patched}
                        </div>
                        <div className="repo-meta">
                          <span className={`chip ${tone}`}>{item.fixNow ? "fix now" : item.planNext ? "plan next" : "watch"}</span>
                          <span className="chip signal">{humanizeToken(item.highestSeverity, "severity")}</span>
                          <span className="chip">{item.ecosystem}</span>
                          <span className="chip">{alertCount} alerts</span>
                          {item.runtimeExposed ? <span className="chip amber">{item.runtimeExposed} runtime</span> : null}
                        </div>
                        <div className="feed-meta" style={{ marginTop: 8 }}>
                          {item.findings.slice(0, 3).map((finding) => finding.summary || finding.title).join(" · ")}
                        </div>
                        {identifiers.length ? (
                          <div className="repo-meta" style={{ marginTop: 8 }}>
                            {identifiers.map((identifier) => <span className="chip" key={identifier}>{identifier}</span>)}
                          </div>
                        ) : null}
                        {evidence.length ? (
                          <div className="feed-meta" style={{ marginTop: 8 }}>
                            {evidence.join(" · ")}
                          </div>
                        ) : null}
                        {links.length ? (
                          <div className="repo-meta" style={{ marginTop: 8 }}>
                            {links.map((link, linkIndex) => (
                              <a className="chip signal" href={link.href} key={`${link.href}-${linkIndex}`} rel="noreferrer" target="_blank">
                                {link.label}
                              </a>
                            ))}
                          </div>
                        ) : null}
                      </div>
                      <span className={`chip ${tone}`}>{item.maxScore}</span>
                    </div>
                  );
                }
                const finding = item.finding;
                const links = findingLinks([finding]).slice(0, 3);
                return (
                  <div className="ledger-row" key={finding.key || index}>
                    <div className="rank">{String(index + 1).padStart(2, "0")}</div>
                    <div>
                      <div className="repo-name">{finding.title || finding.package_name || finding.key}</div>
                      <div className="feed-meta">{finding.summary || finding.next_action || finding.location}</div>
                      <div className="repo-meta">
                        <span className={`chip ${findingTone(finding)}`}>{recommendationLabel(finding.recommendation)}</span>
                        <span className="chip signal">{humanizeToken(finding.severity || finding.source, "security")}</span>
                        {finding.location && <span className="chip">{finding.location}</span>}
                      </div>
                      {finding.next_action ? (
                        <div className="feed-meta" style={{ marginTop: 8 }}>
                          Next action: {finding.next_action}
                        </div>
                      ) : null}
                      {finding.identifiers?.length ? (
                        <div className="repo-meta" style={{ marginTop: 8 }}>
                          {finding.identifiers.slice(0, 4).map((identifier) => <span className="chip" key={identifier}>{identifier}</span>)}
                        </div>
                      ) : null}
                      {finding.evidence?.length ? (
                        <div className="feed-meta" style={{ marginTop: 8 }}>
                          {finding.evidence.slice(0, 2).join(" · ")}
                        </div>
                      ) : null}
                      {links.length ? (
                        <div className="repo-meta" style={{ marginTop: 8 }}>
                          {links.map((link, linkIndex) => (
                            <a className="chip signal" href={link.href} key={`${link.href}-${linkIndex}`} rel="noreferrer" target="_blank">
                              {link.label}
                            </a>
                          ))}
                        </div>
                      ) : null}
                    </div>
                    <span className={`chip ${findingTone(finding)}`}>{asCount(finding.score)}</span>
                  </div>
                );
              })}
            </>
          ) : (
            <div className="empty-v2">
              <strong>No findings</strong>
              <span>This scan did not return actionable vulnerability findings.</span>
            </div>
          )}
        </div>
      </Panel>
    );
  }
  return (
    <Panel eyebrow="Queue" title="Recent scans" action={<span className="chip signal">{history.length} saved</span>}>
      <div className="panelbody repo-list queue-grid">
        {history.length ? history.slice(0, 5).map((item, index) => (
          <div className="ledger-row" key={item.id}>
            <div className="rank">{String(index + 1).padStart(2, "0")}</div>
            <div>
              <div className="repo-name">{item.repo}</div>
              <div className="feed-meta">{scan?.id === item.id ? scanSummary(scan) : item.summary || "Saved VulnTriage scan."}</div>
              <div className="repo-meta">
                <span className="chip red">{asCount(item.fix_now)} fix</span>
                <span className="chip amber">{asCount(item.plan_next)} plan</span>
                <span className="chip green">{asCount(item.watch)} watch</span>
                <span className="chip">{timeAgo(item.created_at)}</span>
              </div>
            </div>
            <button className="btn" onClick={() => onLoadScan(item.id)} type="button">Load</button>
          </div>
        )) : (
          <div className="empty-v2">
            <strong>No scans yet</strong>
            <span>Scan a GitHub repo to populate the security queue.</span>
          </div>
        )}
      </div>
    </Panel>
  );
}

function SidePanels({ scan }) {
  const warnings = scan?.warnings || [];
  const metrics = scan?.metrics || {};
  const tracked = asCount(metrics.tracked_findings);
  const fixNow = asCount(metrics.fix_now);
  const planNext = asCount(metrics.plan_next);
  const evidenceTitle = tracked ? "Why it ranks high" : "Security evidence";
  const humanAction = fixNow ? "fix now" : planNext ? "plan next" : tracked ? "watch" : "none";
  const humanActionTone = fixNow ? "red" : planNext ? "amber" : tracked ? "signal" : "green";
  return (
    <aside className="side">
      <Panel eyebrow="Evidence" title={evidenceTitle}>
        <div className="panelbody repo-list">
          {warnings.length ? warnings.slice(0, 3).map((warning) => (
            <div className="feed-item" key={warning}>
              <div>
                <div className="feed-title">Scan warning</div>
                <div className="feed-meta">{warningLabel(warning)}</div>
              </div>
              <span className="chip amber">warn</span>
            </div>
          )) : (
            <>
              <div className="rowline"><span className="muted">Runtime scoped</span><span className={`chip ${runtimeScoped(scan?.metrics) ? "red" : "green"}`}>{runtimeScoped(scan?.metrics)} findings</span></div>
              <div className="rowline"><span className="muted">Owner scoped</span><span className="chip signal">{asCount(scan?.metrics?.owner_scoped)} routed</span></div>
              <div className="rowline"><span className="muted">Warnings</span><span className="chip green">clear</span></div>
            </>
          )}
        </div>
      </Panel>
      <Panel eyebrow="Consumers" title="Security handoff">
        <div className="panelbody repo-list">
          <div className="rowline"><span className="muted">ReleaseSentry</span><span className={`chip ${fixNow ? "red" : planNext ? "amber" : "green"}`}>{fixNow ? "hold" : planNext ? "watch" : "ready"}</span></div>
          <div className="rowline"><span className="muted">TrustGate</span><span className="chip amber">rules</span></div>
          <div className="rowline"><span className="muted">Human action</span><span className={`chip ${humanActionTone}`}>{humanAction}</span></div>
        </div>
      </Panel>
    </aside>
  );
}

function TriageSurface({
  error,
  form,
  health,
  history,
  onChangeForm,
  onClearScan,
  onLoadScan,
  onRefresh,
  onRunScan,
  overview,
  running,
  scan,
}) {
  const rail = useMemo(() => buildRail(scan, history, overview, health), [scan, history, overview, health]);
  const metrics = useMemo(() => buildMetrics(scan, overview, health), [scan, overview, health]);
  const [copyState, setCopyState] = useState("");

  async function copySummary() {
    if (!scan || !navigator?.clipboard?.writeText) return;
    try {
      await navigator.clipboard.writeText(buildScanMarkdown(scan));
      setCopyState("Copied");
    } catch {
      setCopyState("Copy failed");
    } finally {
      window.setTimeout(() => setCopyState(""), 1800);
    }
  }

  return (
    <>
      <SuiteTopline cells={buildTopline(health, overview, scan, history)} />
      <div className="main-grid">
        <ProductRail sections={rail.sections} stats={rail.stats} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - security triage</div>
              <h1>Vulnerability Pressure</h1>
              <p className="subline">Code scanning alerts, Dependabot advisories, owner hints, and reachability proxies ranked into engineering work.</p>
            </div>
            <div className="actions">
              <span className={`chip ${githubReady(health) ? "green" : "amber"}`}>{githubReady(health) ? "github ready" : "token missing"}</span>
              {scan && <button className="btn" onClick={copySummary} type="button">{copyState || "Copy summary"}</button>}
              {scan && <button className="btn" onClick={onClearScan} type="button">Clear scan</button>}
              <button className="btn" onClick={onRefresh} type="button">Refresh</button>
            </div>
          </div>
          <ScanForm error={error} form={form} health={health} onChange={onChangeForm} onRun={onRunScan} running={running} />
          <MetricBand metrics={metrics} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Triage" title="Scan map" action={<span className="chip signal">scan radar</span>}>
              <VulnerabilityMap health={health} history={history} scan={null} />
            </Panel>
            <FixQueuePanel history={history} onLoadScan={onLoadScan} scan={scan} />
          </div>
        </main>
        <SidePanels scan={scan} />
      </div>
    </>
  );
}

function SecondaryFrame({ children, health, history, overview, scan }) {
  const rail = useMemo(() => buildRail(scan, history, overview, health), [scan, history, overview, health]);
  return (
    <>
      <SuiteTopline cells={buildTopline(health, overview, scan, history)} />
      <div className="main-grid hive-workspace-grid">
        <ProductRail sections={rail.sections} stats={rail.stats} />
        <main className="workspace">{children}</main>
      </div>
    </>
  );
}

function HistorySurface({ activeScanId, health, history, loading, onClearScan, onLoadScan, onRefresh, overview, scan }) {
  const selectedHasFindings = Boolean(scan?.findings?.length);
  return (
    <SecondaryFrame health={health} history={history} overview={overview} scan={scan}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// VulnTriage security queue</div>
          <h1>Scan History</h1>
          <p className="subline">Saved security snapshots with bucket movement, owner routing, and release pressure.</p>
        </div>
        <div className="actions">
          {scan && <button className="btn" onClick={onClearScan} type="button">Clear scan</button>}
          <button className="btn" onClick={onRefresh} type="button">{loading ? "Refreshing..." : "Refresh"}</button>
        </div>
      </div>
      {scan && (
        <HistoryDetailGrid>
          <Panel
            eyebrow="Triage"
            title={selectedHasFindings ? "Selected finding map" : "Selected scan map"}
            action={<span className="chip signal">{selectedHasFindings ? "finding radar" : "scan radar"}</span>}
          >
            <VulnerabilityMap health={health} history={history} scan={scan} />
          </Panel>
          <FixQueuePanel history={history} onLoadScan={onLoadScan} scan={scan} />
        </HistoryDetailGrid>
      )}
      <Panel eyebrow="Recent" title="Security scans" action={<span className="chip signal">{history.length} saved</span>}>
        <div className="panelbody repo-list queue-grid">
          {history.length ? history.map((item, index) => (
            <div className="ledger-row" key={item.id}>
              <div className="rank">{item.id === activeScanId ? "SEL" : String(index + 1).padStart(2, "0")}</div>
              <div>
                <div className="repo-name">{item.repo}</div>
                <div className="feed-meta">{scan?.id === item.id ? scanSummary(scan) : item.summary || "Saved VulnTriage scan."}</div>
                <div className="repo-meta">
                  <span className="chip red">{asCount(item.fix_now)} fix</span>
                  <span className="chip amber">{asCount(item.plan_next)} plan</span>
                  <span className="chip green">{asCount(item.watch)} watch</span>
                  <span className="chip">{timeAgo(item.created_at)}</span>
                </div>
              </div>
              <button className="btn" onClick={() => onLoadScan(item.id)} type="button">Load</button>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>No scans saved</strong>
              <span>Scan a repository to create security history.</span>
            </div>
          )}
        </div>
      </Panel>
    </SecondaryFrame>
  );
}

function checkTone(level) {
  if (level === "error") return "red";
  if (level === "warn") return "amber";
  return "green";
}

function ChecksSurface({ history, onClearScan, overview, runtime, scan }) {
  const health = runtime.health || {};
  const checks = runtime.checks || [];
  const warnings = checks.filter((check) => check.level === "warn" || check.level === "error").length;
  const metrics = [
    { label: "Status", value: health.status || "unknown", tone: health.status === "ok" ? "ok" : "warn", sub: health.version || "backend" },
    { label: "GitHub", value: githubReady(health) ? "ready" : "missing", tone: githubReady(health) ? "ok" : "warn", sub: "security reads" },
    { label: "Scans", value: String(asCount(health.scan_count || overview?.counts?.scans)), tone: "sig", sub: `${asCount(health.repo_count || overview?.counts?.repos)} repos` },
    { label: "Findings", value: String(asCount(health.tracked_finding_count || overview?.counts?.tracked_findings)), tone: "sig", sub: "tracked" },
    { label: "Checks", value: warnings ? String(warnings) : "clear", tone: warnings ? "warn" : "ok", sub: "startup" },
  ];
  return (
    <SecondaryFrame health={health} history={history} overview={overview} scan={scan}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// VulnTriage readiness</div>
          <h1>Checks</h1>
          <p className="subline">Backend health, GitHub security permissions, scan counts, and startup checks.</p>
        </div>
        <div className="actions">
          <button className="btn" onClick={runtime.refresh} type="button">{runtime.loading ? "Refreshing..." : "Refresh"}</button>
        </div>
      </div>
      {runtime.error && <StatusBanner tone="red">{runtime.error}</StatusBanner>}
      <MetricBand metrics={metrics} />
      <div className="atlas-layout suite-four-layout">
        <Panel eyebrow="Health" title="Backend status" action={<span className={`chip ${health.status === "ok" ? "green" : "amber"}`}>{health.status || "unknown"}</span>}>
          <div className="panelbody repo-list">
            <div className="rowline"><span className="muted">Auth enabled</span><span className={`chip ${health.auth_enabled ? "green" : "amber"}`}>{health.auth_enabled ? "yes" : "no"}</span></div>
            <div className="rowline"><span className="muted">GitHub ready</span><span className={`chip ${githubReady(health) ? "green" : "amber"}`}>{githubReady(health) ? "yes" : "no"}</span></div>
            <div className="feed-item">
              <div>
                <div className="feed-title">Database</div>
                <div className="feed-meta break-all">{health.db_path || "unknown"}</div>
              </div>
              <span className={`chip ${health.db_ok ? "green" : "red"}`}>{health.db_ok ? "ok" : "check"}</span>
            </div>
          </div>
        </Panel>
        <Panel eyebrow="Startup" title="Startup checks" action={<span className={`chip ${warnings ? "amber" : "green"}`}>{warnings ? `${warnings} warnings` : "clear"}</span>}>
          <div className="panelbody repo-list">
            {checks.length ? checks.map((check, index) => (
              <div className="feed-item" key={`${check.msg}-${index}`}>
                <div>
                  <div className="feed-title">{check.level || "info"}</div>
                  <div className="feed-meta">{check.msg}</div>
                </div>
                <span className={`chip ${checkTone(check.level)}`}>{check.level || "info"}</span>
              </div>
            )) : (
              <div className="empty-v2">
                <strong>No checks</strong>
                <span>No startup checks were returned by the backend.</span>
              </div>
            )}
          </div>
        </Panel>
      </div>
    </SecondaryFrame>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = usePersistentProductTab("vuln-triage", TABS, "triage");
  const [error, setError] = useState("");
  const [form, setForm] = useState(DEFAULT_FORM);
  const [history, setHistory] = useState([]);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [overview, setOverview] = useState(null);
  const [running, setRunning] = useState(false);
  const [scan, setScan] = useState(null);
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "vuln-triage_api_key" });
  const fetch_ = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const ready = auth.checked && !auth.needsAuth;
  const runtime = useProductRuntime({ apiBase: API, fetcher: fetch_, ready });
  const authConfigured = Boolean(runtime.authStatus?.auth_configured || runtime.health?.auth_enabled);

  async function fetchJson(path, options, fallbackError) {
    const response = await fetch_(`${API}${path}`, options);
    return parseJsonResponse(response, fallbackError);
  }

  async function refreshTriageData() {
    if (!ready) return;
    setLoadingHistory(true);
    const [overviewResult, historyResult] = await Promise.allSettled([
      fetchJson("/overview", undefined, "VulnTriage could not load overview."),
      fetchJson("/history", undefined, "VulnTriage could not load history."),
    ]);
    setOverview(overviewResult.status === "fulfilled" ? overviewResult.value : null);
    setHistory(historyResult.status === "fulfilled" ? historyResult.value || [] : []);
    setLoadingHistory(false);
    const failed = [overviewResult, historyResult].find((result) => result.status === "rejected");
    if (failed) {
      setError(failed.reason?.message || "VulnTriage could not load one or more backend resources.");
    }
  }

  useEffect(() => {
    refreshTriageData();
  }, [ready, fetch_]);

  async function runScan() {
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(
        "/scan/github/findings",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            repo: form.repo,
            include_code_scanning: githubReady(runtime.health) && Boolean(form.include_code_scanning),
            include_dependency_alerts: githubReady(runtime.health) && Boolean(form.include_dependency_alerts),
          }),
        },
        "VulnTriage could not scan that repository.",
      );
      setScan(result);
      setForm((current) => ({ ...current, repo: result.repo || current.repo }));
      setActiveTab("triage");
      await refreshTriageData();
      await runtime.refresh();
    } catch (err) {
      setError(err.message || "VulnTriage could not scan that repository.");
    } finally {
      setRunning(false);
    }
  }

  async function loadScan(id) {
    if (!id) return;
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(`/history/${id}`, undefined, "VulnTriage could not load that scan.");
      setScan(result);
      setForm((current) => ({ ...current, repo: result.repo || current.repo }));
    } catch (err) {
      setError(err.message || "VulnTriage could not load that scan.");
    } finally {
      setRunning(false);
    }
  }

  function clearScan() {
    setScan(null);
    setError("");
  }

  if (!ready) {
    return (
      <ProductV2AuthGate
        apiBase={API}
        auth={auth}
        keyPrefix="vuln-triage-"
        productKey="vuln-triage"
        productName="VulnTriage"
      />
    );
  }

  return (
    <ProductV2Shell authConfigured={authConfigured} productKey="vuln-triage" productName="VulnTriage" runtime={runtime}>
      <DeckBar
        activeTab={activeTab}
        brandEyebrow="PatchHive"
        brandName="VulnTriage"
        navLabel="VulnTriage navigation"
        onTabChange={setActiveTab}
        productKey="vuln-triage"
        tabs={TABS}
      />
      {activeTab === "triage" && (
        <TriageSurface
          error={error}
          form={form}
          health={runtime.health || {}}
          history={history}
          onChangeForm={setForm}
          onClearScan={clearScan}
          onLoadScan={loadScan}
          onRefresh={() => {
            refreshTriageData();
            runtime.refresh();
          }}
          onRunScan={runScan}
          overview={overview}
          running={running}
          scan={scan}
        />
      )}
      {activeTab === "history" && (
        <HistorySurface
          activeScanId={scan?.id || ""}
          health={runtime.health || {}}
          history={history}
          loading={loadingHistory}
          onClearScan={clearScan}
          onLoadScan={loadScan}
          onRefresh={refreshTriageData}
          overview={overview}
          scan={scan}
        />
      )}
      {activeTab === "checks" && <ChecksSurface history={history} onClearScan={clearScan} overview={overview} runtime={runtime} scan={scan} />}
    </ProductV2Shell>
  );
}
