import { useEffect, useMemo, useState } from "react";
import { createApiFetcher, useApiKeyAuth, useProductRuntime } from "@patchhivehq/product-shell/auth";
import {
  DeckBar,
  MetricBand,
  Panel,
  ProductV2AuthGate,
  ProductV2Shell,
  ProductRail,
  SuiteRadar,
  SuiteTopline,
  usePersistentProductTab,
} from "@patchhivehq/ui-v2";
import { API } from "./config.js";

const TABS = [
  { id: "review", label: "Risk review" },
  { id: "rules", label: "Rule packs" },
  { id: "history", label: "Decision log" },
  { id: "checks", label: "Checks" },
];

const DEFAULT_FORM = {
  ai_source: "Codex",
  diff: "",
  pr_number: "",
  publish_status: true,
  repo: "",
};

const DEFAULT_RULE_FORM = {
  blocked_paths: ".github/workflows/, infra/, terraform/, migrations/, schema.sql",
  blocked_terms: "BEGIN PRIVATE KEY, PRIVATE KEY-----, ghp_, github_pat_, sk-, AKIA",
  max_additions: "400",
  max_deletions: "250",
  max_files: "12",
  notes: "",
  repo: "",
  require_test_for_paths: "src/, app/, lib/, server/, backend/",
  suspicious_terms: "TODO, FIXME, skip ci, eval(, exec(, unsafe, curl | sh, rm -rf, password, secret, token",
  test_paths: "tests/, __tests__/, .test., .spec.",
  warn_paths: "auth/, permissions, billing, Dockerfile, docker-compose",
};

const POSITIONS = [
  { left: "68%", top: "32%" },
  { left: "43%", top: "56%" },
  { left: "27%", top: "39%" },
  { left: "58%", top: "72%" },
  { left: "69%", top: "63%" },
  { left: "34%", top: "72%" },
  { left: "74%", top: "45%" },
  { left: "51%", top: "24%" },
];

function asCount(value) {
  const number = Number(value || 0);
  return Number.isFinite(number) ? number : 0;
}

function timeAgo(value) {
  if (!value) {
    return "never";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return String(value);
  }
  const seconds = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 48) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

function splitList(value) {
  return String(value || "")
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean);
}

function joinList(value = []) {
  return value.join(", ");
}

function recommendationTone(recommendation) {
  const normalized = String(recommendation || "").toLowerCase();
  if (normalized === "safe") return "green";
  if (normalized === "warn") return "amber";
  if (normalized === "block") return "red";
  return "signal";
}

function metricTone(recommendation) {
  const normalized = String(recommendation || "").toLowerCase();
  if (normalized === "safe") return "ok";
  if (normalized === "warn") return "warn";
  if (normalized === "block") return "hot";
  return "sig";
}

function findingTone(severity) {
  if (severity === "block") return "red";
  if (severity === "warn") return "amber";
  return "green";
}

function checkTone(level) {
  if (level === "error") return "red";
  if (level === "warn") return "amber";
  return "green";
}

function githubReady(health) {
  return Boolean(health?.github?.token_configured);
}

function plural(value, singular, pluralValue = `${singular}s`) {
  return asCount(value) === 1 ? singular : pluralValue;
}

function toRulePayload(form) {
  return {
    blocked_paths: splitList(form.blocked_paths),
    blocked_terms: splitList(form.blocked_terms),
    max_additions: Number(form.max_additions) || 400,
    max_deletions: Number(form.max_deletions) || 250,
    max_files: Number(form.max_files) || 12,
    notes: String(form.notes || "").trim(),
    repo: String(form.repo || "").trim(),
    require_test_for_paths: splitList(form.require_test_for_paths),
    suspicious_terms: splitList(form.suspicious_terms),
    test_paths: splitList(form.test_paths),
    warn_paths: splitList(form.warn_paths),
  };
}

function fromRules(ruleSet = {}) {
  return {
    blocked_paths: joinList(ruleSet.blocked_paths || DEFAULT_RULE_FORM.blocked_paths.split(", ")),
    blocked_terms: joinList(ruleSet.blocked_terms || DEFAULT_RULE_FORM.blocked_terms.split(", ")),
    max_additions: String(ruleSet.max_additions ?? 400),
    max_deletions: String(ruleSet.max_deletions ?? 250),
    max_files: String(ruleSet.max_files ?? 12),
    notes: ruleSet.notes || "",
    repo: ruleSet.repo || "",
    require_test_for_paths: joinList(ruleSet.require_test_for_paths || DEFAULT_RULE_FORM.require_test_for_paths.split(", ")),
    suspicious_terms: joinList(ruleSet.suspicious_terms || DEFAULT_RULE_FORM.suspicious_terms.split(", ")),
    test_paths: joinList(ruleSet.test_paths || DEFAULT_RULE_FORM.test_paths.split(", ")),
    warn_paths: joinList(ruleSet.warn_paths || DEFAULT_RULE_FORM.warn_paths.split(", ")),
  };
}

async function parseJsonResponse(response, fallbackError) {
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(data.error || data.message || fallbackError);
  }
  return data;
}

function buildTopline(review, health) {
  const recommendation = review?.recommendation || "ready";
  return [
    { label: "TrustGate", value: "Policy gate", tone: "sig" },
    { label: "System", value: health?.status || "checking", tone: health?.status === "ok" ? "ok" : "warn" },
    { label: "Mode", value: "Review only" },
    { label: "GitHub", value: githubReady(health) ? "PR review" : "optional", tone: githubReady(health) ? "sig" : "warn" },
    { label: "FailGuard", value: "armed", tone: "sig" },
    { label: "Last decision", value: review?.created_at ? timeAgo(review.created_at) : recommendation },
  ];
}

function buildMetrics(review, history, health) {
  if (review) {
    const riskScore = asCount(review.risk_score);
    const filesChanged = asCount(review.metrics?.files_changed);
    const riskyFiles = asCount(review.metrics?.risky_files);
    const blockedFindings = asCount(review.metrics?.blocked_findings);
    const warningFindings = asCount(review.metrics?.warning_findings);
    const testsChanged = asCount(review.metrics?.tests_changed);
    const ruleHits = blockedFindings + warningFindings;

    return [
      { label: "Decision", value: String(review.recommendation || "ready").toUpperCase(), tone: metricTone(review.recommendation), sub: review.summary || "review complete" },
      { label: "Risk score", value: String(riskScore), tone: riskScore >= 70 ? "hot" : riskScore >= 40 ? "warn" : "ok", unit: "/ 100", sub: "policy risk" },
      { label: "Files touched", value: String(filesChanged), tone: "sig", unit: plural(filesChanged, "file"), sub: `${riskyFiles} risky ${plural(riskyFiles, "file")}` },
      { label: "Rule hits", value: String(ruleHits), tone: blockedFindings ? "hot" : ruleHits ? "warn" : "ok", unit: plural(ruleHits, "hit"), sub: `${blockedFindings} blocking` },
      { label: "Tests found", value: String(testsChanged), tone: testsChanged ? "ok" : "warn", unit: plural(testsChanged, "test file"), sub: "matched test paths" },
    ];
  }

  return [
    { label: "Decision", value: "READY", tone: "sig", sub: "load or run review" },
    { label: "Saved reviews", value: String(history.length || asCount(health?.review_count)), tone: "sig", sub: `${asCount(health?.repo_count)} repos` },
    { label: "Rule sets", value: String(asCount(health?.rules_count)), tone: "sig", sub: "repo memory" },
    { label: "GitHub", value: githubReady(health) ? "READY" : "LOCAL", tone: githubReady(health) ? "ok" : "warn", sub: "PR review" },
    { label: "Templates", value: String(asCount(health?.template_count)), tone: "sig", sub: "report voice" },
  ];
}

function buildRailSections(review, rules) {
  const ruleNames = review?.rules
    ? [
      { label: "blocked paths", active: true, badge: String(review.rules.blocked_paths?.length || 0), badgeTone: "red" },
      { label: "sensitive paths", badge: String(review.rules.warn_paths?.length || 0), badgeTone: "amber" },
      { label: "test required", badge: String(review.rules.require_test_for_paths?.length || 0), badgeTone: "signal" },
      { label: "scope caps", badge: String(review.rules.max_files || 0), badgeTone: "signal" },
    ]
    : rules.slice(0, 4).map((item, index) => ({
      active: index === 0,
      label: item.repo,
      value: `${item.rules?.blocked_paths?.length || 0} blocked`,
    }));

  return [
    {
      title: "Rule packs",
      items: ruleNames.length ? ruleNames : [{ label: "default policy", active: true, value: "ready" }],
    },
    {
      title: "Sinks",
      items: [
        { label: "github checks", badge: review?.github_report?.check_url ? "sent" : "ready", badgeTone: review?.github_report?.delivered ? "green" : "amber" },
        { label: "pr comment", badge: review?.github_report?.comment_url ? "sent" : "ready", badgeTone: review?.github_report?.comment_url ? "green" : "signal" },
        { label: "failguard", active: true, badge: review?.recommendation === "safe" ? "quiet" : "armed", badgeTone: review?.recommendation === "safe" ? "green" : "amber" },
      ],
    },
  ];
}

function buildRailStats(review, history) {
  const latest = history[0] || {};
  const recommendation = review?.recommendation || latest.recommendation || "ready";
  return {
    title: "Current diff",
    items: [
      { label: "Repository", value: review?.repo || latest.repo || "none" },
      { label: "Risk", value: String(recommendation).toUpperCase(), large: true, tone: metricTone(recommendation) },
      { label: "Source", value: review?.source_kind || latest.source_kind || "manual" },
    ],
  };
}

function countDecisions(history, recommendation) {
  return history.filter((item) => item.recommendation === recommendation).length;
}

function tabRailSections({ checks = [], health = {}, history = [], packs = [], review = null, rules = [], ruleForm = DEFAULT_RULE_FORM }) {
  health = health || {};
  return {
    checks: [
      {
        title: "Runtime",
        items: [
          { label: "backend", active: true, badge: health.status || "unknown", badgeTone: health.status === "ok" ? "green" : "amber" },
          { label: "github", badge: githubReady(health) ? "ready" : "optional", badgeTone: githubReady(health) ? "green" : "amber" },
          { label: "webhook", badge: health.github?.webhook_secret_configured ? "ready" : "missing", badgeTone: health.github?.webhook_secret_configured ? "green" : "amber" },
          { label: "auth", badge: health.auth_enabled ? "on" : "open", badgeTone: health.auth_enabled ? "green" : "amber" },
        ],
      },
      {
        title: "Contracts",
        items: [
          { label: "startup checks", badge: String(checks.length), badgeTone: checks.length ? "signal" : "green" },
          { label: "rules", badge: String(asCount(health.rules_count)), badgeTone: "signal" },
          { label: "reviews", badge: String(asCount(health.review_count)), badgeTone: "signal" },
        ],
      },
    ],
    history: [
      {
        title: "Decisions",
        items: [
          { label: "safe", active: review?.recommendation === "safe", badge: String(countDecisions(history, "safe")), badgeTone: "green" },
          { label: "warn", active: review?.recommendation === "warn", badge: String(countDecisions(history, "warn")), badgeTone: "amber" },
          { label: "block", active: review?.recommendation === "block", badge: String(countDecisions(history, "block")), badgeTone: "red" },
        ],
      },
      {
        title: "Sources",
        items: [
          { label: "manual", badge: String(history.filter((item) => (item.source_kind || "manual") === "manual").length), badgeTone: "signal" },
          { label: "github pr", badge: String(history.filter((item) => item.source_kind === "github_pr").length), badgeTone: "signal" },
        ],
      },
    ],
    rules: [
      {
        title: "Rule Memory",
        items: [
          { label: "saved repos", active: true, badge: String(rules.length), badgeTone: "signal" },
          { label: "starter packs", badge: String(packs.length), badgeTone: "signal" },
          { label: "current repo", value: ruleForm.repo || "none" },
        ],
      },
      {
        title: "Default Caps",
        items: [
          { label: "max files", badge: ruleForm.max_files || "12", badgeTone: "signal" },
          { label: "max additions", badge: ruleForm.max_additions || "400", badgeTone: "signal" },
          { label: "max deletions", badge: ruleForm.max_deletions || "250", badgeTone: "signal" },
        ],
      },
    ],
  };
}

function tabRailStats({ health = {}, history = [], review = null, ruleForm = DEFAULT_RULE_FORM }, tab) {
  health = health || {};
  const latest = history[0] || {};
  if (tab === "rules") {
    return {
      title: "Policy target",
      items: [
        { label: "Repository", value: ruleForm.repo || "none" },
        { label: "Mode", value: "RULES", large: true, tone: "sig" },
        { label: "Publish", value: "review only" },
      ],
    };
  }
  if (tab === "checks") {
    return {
      title: "Backend",
      items: [
        { label: "Status", value: health.status || "unknown", large: true, tone: health.status === "ok" ? "ok" : "warn" },
        { label: "Database", value: health.db_ok ? "ok" : "check" },
        { label: "GitHub", value: githubReady(health) ? "ready" : "optional" },
      ],
    };
  }
  return {
    title: "Selected decision",
    items: [
      { label: "Repository", value: review?.repo || latest.repo || "none" },
      { label: "Decision", value: String(review?.recommendation || latest.recommendation || "ready").toUpperCase(), large: true, tone: metricTone(review?.recommendation || latest.recommendation) },
      { label: "Age", value: review?.created_at ? timeAgo(review.created_at) : latest.created_at ? timeAgo(latest.created_at) : "none" },
    ],
  };
}

function TabFrame({ children, health, railSections, railStats, review }) {
  const topline = buildTopline(review, health);
  return (
    <>
      <SuiteTopline cells={topline} />
      <div className="main-grid hive-workspace-grid">
        <ProductRail sections={railSections} stats={railStats} />
        <main className="workspace">{children}</main>
      </div>
    </>
  );
}

function decisionRadarItem(review) {
  const recommendation = review?.recommendation || "ready";
  return {
    detail: review?.repo || "Current review",
    gain: recommendation,
    gainMeta: `${asCount(review?.metrics?.files_changed)} files`,
    id: review?.id || "current-decision",
    label: recommendation,
    minWindow: 7,
    position: { left: "50%", top: "44%" },
    stats: [
      { label: "Decision", value: recommendation },
      { label: "Risk", value: String(asCount(review?.risk_score)) },
      { label: "Block", value: String(asCount(review?.metrics?.blocked_findings)) },
      { label: "Warn", value: String(asCount(review?.metrics?.warning_findings)) },
      { label: "Tests", value: String(asCount(review?.metrics?.tests_changed)) },
    ],
    summary: review?.summary || "TrustGate decision for the current diff.",
    title: `${String(recommendation).toUpperCase()} decision`,
    tone: recommendationTone(recommendation),
    vector: recommendation,
    vectorTone: recommendation === "warn" || recommendation === "block" ? "warn" : "",
  };
}

function buildRadarItems(review, history) {
  if (review) {
    const items = [decisionRadarItem(review)];
    const findingItems = (review.findings || []).map((finding, index) => ({
      detail: finding.label || "Finding",
      gain: finding.severity || "info",
      gainMeta: finding.evidence?.[0] || finding.key,
      id: finding.key || `finding-${index + 1}`,
      label: finding.severity || `F${index + 1}`,
      minWindow: index < 3 ? 7 : index < 6 ? 14 : 30,
      position: POSITIONS[(index + 1) % POSITIONS.length],
      stats: [
        { label: "Severity", value: finding.severity || "info" },
        { label: "Key", value: finding.key || "policy" },
        { label: "Evidence", value: String(finding.evidence?.length || 0) },
        { label: "Repo", value: review.repo || "repo" },
        { label: "Decision", value: review.recommendation || "review" },
      ],
      summary: finding.detail || "TrustGate policy finding.",
      title: finding.label || finding.key || `Finding ${index + 1}`,
      tone: findingTone(finding.severity),
      vector: finding.severity || finding.key,
      vectorTone: finding.severity === "block" || finding.severity === "warn" ? "warn" : "",
    }));
    const riskyFileItems = (review.files || [])
      .filter((file) => file.status === "blocked" || file.status === "warn")
      .slice(0, 5)
      .map((file, index) => ({
        detail: file.path,
        gain: file.status || "file",
        gainMeta: `+${asCount(file.additions)} -${asCount(file.deletions)}`,
        id: file.path || `file-${index + 1}`,
        label: file.status || `F${index + 1}`,
        minWindow: index < 3 ? 7 : index < 6 ? 14 : 30,
        position: POSITIONS[(index + findingItems.length + 1) % POSITIONS.length],
        stats: [
          { label: "Status", value: file.status || "review" },
          { label: "Add", value: String(asCount(file.additions)) },
          { label: "Del", value: String(asCount(file.deletions)) },
          { label: "Generated", value: file.generated ? "yes" : "no" },
          { label: "Rules", value: String(file.matched_rules?.length || 0) },
        ],
        summary: file.summary || file.path,
        title: file.path || `File ${index + 1}`,
        tone: file.status === "blocked" ? "red" : file.status === "warn" ? "amber" : "green",
        vector: file.path_policy || file.status,
        vectorTone: file.status === "blocked" || file.status === "warn" ? "warn" : "",
      }));
    return [...items, ...findingItems, ...riskyFileItems];
  }

  if (history.length) {
    return history.slice(0, 8).map((item, index) => ({
      detail: item.repo,
      gain: item.recommendation || "saved",
      gainMeta: `${asCount(item.files_changed)} files`,
      id: item.id || `history-${index + 1}`,
      label: item.recommendation || `R${index + 1}`,
      minWindow: index < 3 ? 7 : index < 6 ? 14 : 30,
      position: POSITIONS[index % POSITIONS.length],
      stats: [
        { label: "Repo", value: item.repo },
        { label: "Source", value: item.source_kind || "manual" },
        { label: "Risk", value: String(asCount(item.risk_score)) },
        { label: "Files", value: String(asCount(item.files_changed)) },
        { label: "Age", value: timeAgo(item.created_at) },
      ],
      summary: item.summary || "Saved TrustGate decision.",
      title: item.repo,
      tone: recommendationTone(item.recommendation),
      vector: item.recommendation || "saved",
      vectorTone: item.recommendation === "warn" || item.recommendation === "block" ? "warn" : "",
    }));
  }

  return [{
    detail: "No diff reviewed yet",
    gain: "standby",
    gainMeta: "manual or GitHub PR",
    id: "trustgate-ready",
    label: "TG",
    position: { left: "50%", top: "44%" },
    stats: [
      { label: "Mode", value: "review only" },
      { label: "Manual", value: "ready" },
      { label: "GitHub", value: "optional" },
      { label: "Rules", value: "default" },
      { label: "Action", value: "review" },
    ],
    summary: "Paste a diff or review a GitHub PR to populate TrustGate's risk radar.",
    title: "TrustGate ready",
    tone: "signal",
    vector: "READY",
  }];
}

function buildRadarFeed(review, health) {
  if (review) {
    const report = review.github_report;
    return [
      { text: review.summary || "TrustGate completed the review.", tone: recommendationTone(review.recommendation) },
      { text: `${asCount(review.metrics?.blocked_findings)} blocking findings and ${asCount(review.metrics?.warning_findings)} warnings are active.`, tone: asCount(review.metrics?.blocked_findings) ? "red" : asCount(review.metrics?.warning_findings) ? "amber" : "green" },
      { text: report?.message || "GitHub publishing is optional for this decision.", tone: report?.delivered ? "green" : "signal" },
    ];
  }
  return [
    { text: "TrustGate reviews diffs against repo-specific policy before automation moves forward.", tone: "signal" },
    { text: githubReady(health) ? "GitHub token is ready for PR-backed reviews." : "Manual pasted-diff review is available without GitHub token access.", tone: githubReady(health) ? "green" : "amber" },
    { text: "Saved rule packs become the policy memory for future reviews.", tone: "signal" },
  ];
}

function ReviewInput({ error, form, onChange, onClear, onRunGitHub, onRunManual, running }) {
  return (
    <Panel eyebrow="Review intake" title="Diff or GitHub PR" action={<span className="chip signal">review only</span>}>
      <div className="panelbody control-stack">
        <div className="form-grid">
          <label className="v2-field">
            Repository
            <input className="v2-input" onChange={(event) => onChange((current) => ({ ...current, repo: event.target.value }))} placeholder="owner/repo" value={form.repo} />
          </label>
          <label className="v2-field">
            AI source
            <input className="v2-input" onChange={(event) => onChange((current) => ({ ...current, ai_source: event.target.value }))} placeholder="Codex" value={form.ai_source} />
          </label>
          <label className="v2-field">
            PR number
            <input className="v2-input" inputMode="numeric" onChange={(event) => onChange((current) => ({ ...current, pr_number: event.target.value }))} placeholder="123" value={form.pr_number} />
          </label>
        </div>
        <label className="rowline" style={{ alignItems: "flex-start", justifyContent: "flex-start" }}>
          <input
            checked={Boolean(form.publish_status)}
            onChange={(event) => onChange((current) => ({ ...current, publish_status: event.target.checked }))}
            style={{ marginTop: 3 }}
            type="checkbox"
          />
          <span>
            <span className="repo-name" style={{ display: "block", fontSize: "0.8rem" }}>Publish GitHub status/check</span>
            <span className="feed-meta">When reviewing a GitHub PR, send TrustGate's result back to the PR flow.</span>
          </span>
        </label>
        <label className="v2-field">
          Manual unified diff
          <textarea
            className="v2-input"
            onChange={(event) => onChange((current) => ({ ...current, diff: event.target.value }))}
            placeholder="Paste a unified diff here..."
            style={{ fontFamily: "var(--mono)", lineHeight: 1.45, minHeight: 180, paddingBottom: 10, paddingTop: 10, resize: "vertical", whiteSpace: "pre" }}
            value={form.diff}
          />
        </label>
        <div className="actions">
          <button className="btn primary" disabled={running || !form.repo.trim() || !form.diff.trim()} onClick={onRunManual} type="button">
            {running ? "Reviewing..." : "Review pasted diff"}
          </button>
          <button className="btn" disabled={running || !form.repo.trim() || !String(form.pr_number).trim()} onClick={onRunGitHub} type="button">
            {running ? "Reviewing..." : "Fetch PR + review"}
          </button>
          <button className="btn" onClick={onClear} type="button">Clear</button>
        </div>
        {error && <div className="status-banner red">{error}</div>}
      </div>
    </Panel>
  );
}

function DecisionGauge({ health, history, review }) {
  const items = useMemo(() => buildRadarItems(review, history), [review, history]);
  const feed = useMemo(() => buildRadarFeed(review, health), [review, health]);
  return (
    <SuiteRadar
      ariaLabel="TrustGate policy risk radar"
      detailLabel={review ? "Risk detail" : "Decision detail"}
      feed={feed}
      gainLabel={review ? "Severity" : "Decision"}
      itemQueryParam="risk"
      items={items}
      signalLabel={review ? "findings" : "decisions"}
      vectorLabel={review ? "Primary reason" : "Selected decision"}
    />
  );
}

function RuleHitPanel({ review }) {
  const findings = review?.findings || [];
  return (
    <Panel eyebrow="Policy" title="Rule hits" action={<span className={`chip ${findings.length ? "amber" : "green"}`}>{findings.length} hits</span>}>
      <div className="panelbody repo-list">
        {findings.length ? findings.map((finding) => (
          <div className="feed-item" key={`${finding.key}-${finding.label}`}>
            <div>
              <div className="feed-title">{finding.label || finding.key}</div>
              <div className="feed-meta">{finding.detail}</div>
            </div>
            <span className={`chip ${findingTone(finding.severity)}`}>{finding.severity || "info"}</span>
          </div>
        )) : (
          <div className="empty-v2">
            <strong>No findings</strong>
            <span>{review ? "The applied rule set did not find active risk." : "Run a review to see policy findings."}</span>
          </div>
        )}
      </div>
    </Panel>
  );
}

function FileRiskPanel({ review }) {
  const files = review?.files || [];
  return (
    <Panel eyebrow="Diff" title="File risk matrix" action={<span className="chip signal">{files.length} files</span>}>
      <div className="panelbody repo-list">
        {files.length ? files.slice(0, 8).map((file) => (
          <div className="ledger-row" key={file.path}>
            <div className="rank">{file.status || "file"}</div>
            <div>
              <div className="repo-name">{file.path}</div>
              <div className="feed-meta">{file.summary || `+${asCount(file.additions)} -${asCount(file.deletions)}`}</div>
            </div>
            <span className={`chip ${file.status === "blocked" ? "red" : file.status === "warn" ? "amber" : "green"}`}>+{asCount(file.additions)} -{asCount(file.deletions)}</span>
          </div>
        )) : (
          <div className="empty-v2">
            <strong>No file matrix</strong>
            <span>Reviewed files will appear here after a diff review.</span>
          </div>
        )}
      </div>
    </Panel>
  );
}

function OutputPanel({ health, review }) {
  const report = review?.github_report;
  return (
    <Panel eyebrow="Output" title="Publish posture">
      <div className="panelbody repo-list">
        <div className="rowline"><span className="muted">GitHub token</span><span className={`chip ${githubReady(health) ? "green" : "amber"}`}>{githubReady(health) ? "ready" : "optional"}</span></div>
        <div className="rowline"><span className="muted">GitHub report</span><span className={`chip ${report?.delivered ? "green" : "amber"}`}>{report?.state || "local"}</span></div>
        <div className="rowline"><span className="muted">FailGuard candidate</span><span className={`chip ${review && review.recommendation !== "safe" ? "amber" : "green"}`}>{review && review.recommendation !== "safe" ? "prepared" : "quiet"}</span></div>
        {review?.github?.pr_url && <button className="btn" onClick={() => window.open(review.github.pr_url, "_blank", "noreferrer")} type="button">Open PR</button>}
        {report?.check_url && <button className="btn" onClick={() => window.open(report.check_url, "_blank", "noreferrer")} type="button">Open check</button>}
        {report?.comment_url && <button className="btn" onClick={() => window.open(report.comment_url, "_blank", "noreferrer")} type="button">Open comment</button>}
        {report?.report_markdown && (
          <button
            className="btn"
            onClick={() => {
              if (typeof navigator !== "undefined" && navigator.clipboard) {
                navigator.clipboard.writeText(report.report_markdown);
              }
            }}
            type="button"
          >
            Copy report
          </button>
        )}
      </div>
    </Panel>
  );
}

function ReviewSurface({
  error,
  form,
  health,
  history,
  onChangeForm,
  onClear,
  onClearReview,
  onRefreshData,
  onRunGitHub,
  onRunManual,
  review,
  rules,
  running,
}) {
  const metrics = useMemo(() => buildMetrics(review, history, health), [review, history, health]);
  const railSections = useMemo(() => buildRailSections(review, rules), [review, rules]);
  const railStats = useMemo(() => buildRailStats(review, history), [review, history]);
  const topline = useMemo(() => buildTopline(review, health), [review, health]);

  return (
    <>
      <SuiteTopline cells={topline} />
      <div className="main-grid">
        <ProductRail sections={railSections} stats={railStats} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - risk gate</div>
              <h1>Trust Review</h1>
              <p className="subline">Review pasted diffs or live GitHub PRs against repo-specific safety rules before automation moves forward.</p>
            </div>
            <div className="actions">
              <span className={`chip ${recommendationTone(review?.recommendation || "ready")}`}>{review?.recommendation || "ready"}</span>
              <span className={`chip ${githubReady(health) ? "green" : "amber"}`}>{githubReady(health) ? "github ready" : "manual ready"}</span>
              {review && <button className="btn" onClick={onClearReview} type="button">Clear review</button>}
              <button className="btn" onClick={onRefreshData} type="button">Refresh data</button>
            </div>
          </div>
          <ReviewInput error={error} form={form} onChange={onChangeForm} onClear={onClear} onRunGitHub={onRunGitHub} onRunManual={onRunManual} running={running} />
          <MetricBand metrics={metrics} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Decision" title="Safety recommendation" action={<span className={`chip ${recommendationTone(review?.recommendation || "ready")}`}>risk radar</span>}>
              <DecisionGauge health={health} history={history} review={review} />
            </Panel>
            <RuleHitPanel review={review} />
          </div>
        </main>
        <aside className="side">
          <FileRiskPanel review={review} />
          <OutputPanel health={health} review={review} />
        </aside>
      </div>
    </>
  );
}

function HistorySurface({ activeReviewId, health, history, loading, onClearReview, onLoadReview, onRefresh, review }) {
  const railSections = useMemo(() => tabRailSections({ health, history, review }).history, [health, history, review]);
  const railStats = useMemo(() => tabRailStats({ health, history, review }, "history"), [health, history, review]);

  return (
    <TabFrame health={health} railSections={railSections} railStats={railStats} review={review}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// TrustGate decision log</div>
          <h1>Decision Log</h1>
          <p className="subline">Saved diff decisions, risk scores, source type, and rule outcomes over time.</p>
        </div>
        <div className="actions">
          {review && <button className="btn" onClick={onClearReview} type="button">Clear review</button>}
          <button className="btn" onClick={onRefresh} type="button">{loading ? "Refreshing..." : "Refresh"}</button>
        </div>
      </div>
      <Panel eyebrow="History" title="Prior reviews" action={<span className="chip signal">{history.length} saved</span>}>
        <div className="panelbody repo-list queue-grid">
          {history.length ? history.map((item) => (
            <div className="ledger-row" key={item.id || `${item.repo}-${item.created_at}`}>
              <div className="rank">{item.id === activeReviewId ? "SEL" : item.recommendation}</div>
              <div>
                <div className="repo-name">{item.repo}</div>
                <div className="feed-meta">{item.summary || "Saved TrustGate decision."}</div>
                <div className="repo-meta">
                  <span className={`chip ${recommendationTone(item.recommendation)}`}>{item.recommendation}</span>
                  <span className="chip signal">{item.source_kind || "manual"}</span>
                  <span className="chip amber">{asCount(item.risk_score)} risk</span>
                  <span className="chip">{timeAgo(item.created_at)}</span>
                </div>
              </div>
              <button className="btn" onClick={() => onLoadReview(item.id)} type="button">Load</button>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>No decisions yet</strong>
              <span>Review a pasted diff or GitHub PR and TrustGate will save the decision here.</span>
            </div>
          )}
        </div>
      </Panel>
      {review && (
        <div className="atlas-layout suite-four-layout">
          <Panel eyebrow="Decision" title="Selected decision" action={<span className={`chip ${recommendationTone(review.recommendation)}`}>{review.recommendation}</span>}>
            <DecisionGauge health={health} history={history} review={review} />
          </Panel>
          <RuleHitPanel review={review} />
        </div>
      )}
      {review && (
        <div className="atlas-layout suite-four-layout">
          <FileRiskPanel review={review} />
          <OutputPanel health={health} review={review} />
        </div>
      )}
    </TabFrame>
  );
}

function RulesSurface({
  busy,
  error,
  health,
  history,
  onApplyPack,
  onClearReview,
  onClearRuleForm,
  onDeleteRules,
  onRefresh,
  onSaveRules,
  onSetRuleForm,
  packs,
  review,
  ruleForm,
  rules,
}) {
  const setField = (key, value) => onSetRuleForm((current) => ({ ...current, [key]: value }));
  const railSections = useMemo(
    () => tabRailSections({ health, history, packs, review, rules, ruleForm }).rules,
    [health, history, packs, review, rules, ruleForm],
  );
  const railStats = useMemo(() => tabRailStats({ health, history, review, ruleForm }, "rules"), [health, history, review, ruleForm]);

  return (
    <TabFrame health={health} railSections={railSections} railStats={railStats} review={review}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// TrustGate rule memory</div>
          <h1>Rule Packs</h1>
          <p className="subline">Tune repo-specific blocked paths, sensitive paths, suspicious terms, and scope caps used by live reviews.</p>
        </div>
        <div className="actions">
          {review && <button className="btn" onClick={onClearReview} type="button">Clear review</button>}
          <button className="btn" onClick={onClearRuleForm} type="button">Clear rules form</button>
          <button className="btn" onClick={onRefresh} type="button">Refresh</button>
        </div>
      </div>
      {error && <div className="status-banner red">{error}</div>}
      <div className="atlas-layout suite-four-layout">
        <Panel eyebrow="Rules" title="Repo rule set" action={<button className="btn primary" disabled={busy} onClick={onSaveRules} type="button">{busy ? "Saving..." : "Save rules"}</button>}>
          <div className="panelbody control-stack">
            <label className="v2-field">
              Repository
              <input className="v2-input" onChange={(event) => setField("repo", event.target.value)} placeholder="owner/repo" value={ruleForm.repo} />
            </label>
            <div className="form-grid">
              <label className="v2-field">
                Blocked paths
                <textarea className="v2-input" onChange={(event) => setField("blocked_paths", event.target.value)} style={{ minHeight: 88, paddingTop: 10, resize: "vertical" }} value={ruleForm.blocked_paths} />
              </label>
              <label className="v2-field">
                Sensitive paths
                <textarea className="v2-input" onChange={(event) => setField("warn_paths", event.target.value)} style={{ minHeight: 88, paddingTop: 10, resize: "vertical" }} value={ruleForm.warn_paths} />
              </label>
            </div>
            <div className="form-grid">
              <label className="v2-field">
                Require tests for
                <textarea className="v2-input" onChange={(event) => setField("require_test_for_paths", event.target.value)} style={{ minHeight: 88, paddingTop: 10, resize: "vertical" }} value={ruleForm.require_test_for_paths} />
              </label>
              <label className="v2-field">
                Suspicious terms
                <textarea className="v2-input" onChange={(event) => setField("suspicious_terms", event.target.value)} style={{ minHeight: 88, paddingTop: 10, resize: "vertical" }} value={ruleForm.suspicious_terms} />
              </label>
            </div>
            <div className="form-grid compact">
              <label className="v2-field">
                Max files
                <input className="v2-input" onChange={(event) => setField("max_files", event.target.value)} type="number" value={ruleForm.max_files} />
              </label>
              <label className="v2-field">
                Max additions
                <input className="v2-input" onChange={(event) => setField("max_additions", event.target.value)} type="number" value={ruleForm.max_additions} />
              </label>
              <label className="v2-field">
                Max deletions
                <input className="v2-input" onChange={(event) => setField("max_deletions", event.target.value)} type="number" value={ruleForm.max_deletions} />
              </label>
            </div>
            <label className="v2-field">
              Notes
              <textarea className="v2-input" onChange={(event) => setField("notes", event.target.value)} placeholder="Why this repo needs stricter review..." style={{ minHeight: 80, paddingTop: 10, resize: "vertical" }} value={ruleForm.notes} />
            </label>
          </div>
        </Panel>
        <Panel eyebrow="Saved" title="Rule memory" action={<span className="chip signal">{rules.length} repos</span>}>
          <div className="panelbody repo-list">
            {rules.length ? rules.map((item) => (
              <div className="feed-item" key={item.repo}>
                <div>
                  <div className="feed-title">{item.repo}</div>
                  <div className="feed-meta">{item.rules?.notes || `${item.rules?.blocked_paths?.length || 0} blocked paths, ${item.rules?.warn_paths?.length || 0} sensitive paths`}</div>
                </div>
                <div className="actions">
                  <button className="btn" onClick={() => onSetRuleForm(fromRules(item.rules))} type="button">Load</button>
                  <button className="btn" disabled={busy} onClick={() => onDeleteRules(item.repo)} type="button">Delete</button>
                </div>
              </div>
            )) : (
              <div className="empty-v2">
                <strong>No saved rules</strong>
                <span>Save a repo rule set and future reviews will pick it up automatically.</span>
              </div>
            )}
          </div>
        </Panel>
      </div>
      <Panel eyebrow="Starter packs" title="Policy templates" action={<span className="chip signal">{packs.length} packs</span>}>
        <div className="panelbody repo-list queue-grid">
          {packs.length ? packs.map((pack) => (
            <div className="ledger-row" key={pack.id}>
              <div className="rank">{pack.id}</div>
              <div>
                <div className="repo-name">{pack.label}</div>
                <div className="feed-meta">{pack.description}</div>
              </div>
              <button className="btn" onClick={() => onApplyPack(pack)} type="button">Apply</button>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>No packs</strong>
              <span>The backend did not return starter policy packs.</span>
            </div>
          )}
        </div>
      </Panel>
    </TabFrame>
  );
}

function ChecksSurface({ history, onClearReview, review, runtime }) {
  const health = runtime.health || {};
  const checks = runtime.checks || [];
  const checkWarnings = checks.filter((check) => check.level === "warn" || check.level === "error").length;
  const railSections = useMemo(() => tabRailSections({ checks, health, history, review }).checks, [checks, health, history, review]);
  const railStats = useMemo(() => tabRailStats({ health, history, review }, "checks"), [health, history, review]);
  const metrics = [
    { label: "Status", value: health.status || "unknown", tone: health.status === "ok" ? "ok" : "warn", sub: health.version || "backend" },
    { label: "GitHub", value: githubReady(health) ? "ready" : "optional", tone: githubReady(health) ? "ok" : "warn", sub: "PR review" },
    { label: "Reviews", value: String(asCount(health.review_count)), tone: "sig", sub: `${asCount(health.repo_count)} repos` },
    { label: "Rules", value: String(asCount(health.rules_count)), tone: "sig", sub: "saved sets" },
    { label: "Checks", value: checkWarnings ? String(checkWarnings) : "clear", tone: checkWarnings ? "warn" : "ok", sub: "startup" },
  ];

  return (
    <TabFrame health={health} railSections={railSections} railStats={railStats} review={review}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// TrustGate readiness</div>
          <h1>Checks</h1>
          <p className="subline">Backend health, auth posture, GitHub readiness, rule memory, and startup checks.</p>
        </div>
        <div className="actions">
          {review && <button className="btn" onClick={onClearReview} type="button">Clear review</button>}
          <button className="btn" onClick={runtime.refresh} type="button">{runtime.loading ? "Refreshing..." : "Refresh"}</button>
        </div>
      </div>
      {runtime.error && <div className="status-banner red">{runtime.error}</div>}
      <MetricBand metrics={metrics} />
      <div className="atlas-layout suite-four-layout">
        <Panel eyebrow="Health" title="Backend status" action={<span className={`chip ${health.status === "ok" ? "green" : "amber"}`}>{health.status || "unknown"}</span>}>
          <div className="panelbody repo-list">
            <div className="rowline"><span className="muted">Auth enabled</span><span className={`chip ${health.auth_enabled ? "green" : "amber"}`}>{health.auth_enabled ? "yes" : "no"}</span></div>
            <div className="rowline"><span className="muted">Webhook secret</span><span className={`chip ${health.github?.webhook_secret_configured ? "green" : "amber"}`}>{health.github?.webhook_secret_configured ? "configured" : "missing"}</span></div>
            <div className="rowline"><span className="muted">Public URL</span><span className={`chip ${health.github?.public_url_configured ? "green" : "amber"}`}>{health.github?.public_url_configured ? "configured" : "missing"}</span></div>
            <div className="rowline"><span className="muted">Templates</span><span className="chip signal">{asCount(health.template_count)}</span></div>
            <div className="feed-item">
              <div>
                <div className="feed-title">Database</div>
                <div className="feed-meta break-all">{health.db_path || "unknown"}</div>
              </div>
              <span className={`chip ${health.db_ok ? "green" : "red"}`}>{health.db_ok ? "ok" : "check"}</span>
            </div>
          </div>
        </Panel>
        <Panel eyebrow="Startup" title="Startup checks" action={<span className={`chip ${checkWarnings ? "amber" : "green"}`}>{checkWarnings ? `${checkWarnings} warnings` : "clear"}</span>}>
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
    </TabFrame>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = usePersistentProductTab("trust-gate", TABS, "review");
  const [error, setError] = useState("");
  const [form, setForm] = useState(DEFAULT_FORM);
  const [history, setHistory] = useState([]);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [packs, setPacks] = useState([]);
  const [review, setReview] = useState(null);
  const [ruleError, setRuleError] = useState("");
  const [ruleForm, setRuleForm] = useState(DEFAULT_RULE_FORM);
  const [rules, setRules] = useState([]);
  const [rulesBusy, setRulesBusy] = useState(false);
  const [running, setRunning] = useState(false);
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "trust-gate_api_key" });
  const fetch_ = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const ready = auth.checked && !auth.needsAuth;
  const runtime = useProductRuntime({ apiBase: API, fetcher: fetch_, ready });
  const authConfigured = Boolean(runtime.authStatus?.auth_configured || runtime.health?.auth_enabled);

  async function fetchJson(path, options, fallbackError) {
    const response = await fetch_(`${API}${path}`, options);
    return parseJsonResponse(response, fallbackError);
  }

  async function refreshTrustData() {
    if (!ready) {
      return;
    }
    setLoadingHistory(true);
    const [historyResult, rulesResult, packsResult] = await Promise.allSettled([
      fetchJson("/history", undefined, "TrustGate could not load history."),
      fetchJson("/rules", undefined, "TrustGate could not load rules."),
      fetchJson("/rule-packs", undefined, "TrustGate could not load rule packs."),
    ]);
    setHistory(historyResult.status === "fulfilled" ? historyResult.value.reviews || [] : []);
    setRules(rulesResult.status === "fulfilled" ? rulesResult.value.rules || [] : []);
    setPacks(packsResult.status === "fulfilled" ? packsResult.value.packs || [] : []);
    setLoadingHistory(false);
  }

  useEffect(() => {
    refreshTrustData();
  }, [ready, fetch_]);

  async function runManualReview() {
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(
        "/review",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            ai_source: form.ai_source,
            diff: form.diff,
            repo: form.repo,
          }),
        },
        "TrustGate could not review this diff.",
      );
      setReview(result);
      setActiveTab("review");
      await refreshTrustData();
      await runtime.refresh();
    } catch (err) {
      setError(err.message || "TrustGate could not review this diff.");
    } finally {
      setRunning(false);
    }
  }

  async function runGitHubReview() {
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(
        "/review/github/pr",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            ai_source: form.ai_source,
            pr_number: Number(form.pr_number) || 0,
            publish_status: Boolean(form.publish_status),
            repo: form.repo,
          }),
        },
        "TrustGate could not fetch and review that PR.",
      );
      setReview(result);
      setForm((current) => ({
        ...current,
        diff: result.diff || current.diff,
        pr_number: result.github?.pr_number ? String(result.github.pr_number) : current.pr_number,
        repo: result.repo || current.repo,
      }));
      setActiveTab("review");
      await refreshTrustData();
      await runtime.refresh();
    } catch (err) {
      setError(err.message || "TrustGate could not fetch and review that PR.");
    } finally {
      setRunning(false);
    }
  }

  async function loadHistoryReview(id) {
    if (!id) {
      return;
    }
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(`/history/${id}`, undefined, "TrustGate could not load that review.");
      setReview(result);
      setForm({
        ai_source: result.ai_source || "unknown",
        diff: result.diff || "",
        pr_number: result.github?.pr_number ? String(result.github.pr_number) : "",
        publish_status: true,
        repo: result.repo || "",
      });
      if (result.rules) {
        setRuleForm(fromRules(result.rules));
      }
    } catch (err) {
      setError(err.message || "TrustGate could not load that review.");
    } finally {
      setRunning(false);
    }
  }

  async function saveRules() {
    if (!ruleForm.repo.trim()) {
      setRuleError("TrustGate needs an owner/repo before it can save rules.");
      return;
    }
    setRulesBusy(true);
    setRuleError("");
    try {
      const result = await fetchJson(
        "/rules",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(toRulePayload(ruleForm)),
        },
        "TrustGate could not save these rules.",
      );
      setRuleForm((current) => ({ ...current, repo: result.repo || current.repo }));
      await refreshTrustData();
      await runtime.refresh();
    } catch (err) {
      setRuleError(err.message || "TrustGate could not save these rules.");
    } finally {
      setRulesBusy(false);
    }
  }

  async function deleteRules(repo) {
    if (!repo) {
      return;
    }
    setRulesBusy(true);
    setRuleError("");
    try {
      await fetchJson(
        `/rules/${encodeURIComponent(repo)}`,
        { method: "DELETE" },
        "TrustGate could not delete this rule set.",
      );
      if (ruleForm.repo === repo) {
        setRuleForm(DEFAULT_RULE_FORM);
      }
      await refreshTrustData();
      await runtime.refresh();
    } catch (err) {
      setRuleError(err.message || "TrustGate could not delete this rule set.");
    } finally {
      setRulesBusy(false);
    }
  }

  function applyPack(pack) {
    const next = fromRules(pack.rules || {});
    next.repo = ruleForm.repo || form.repo;
    setRuleForm(next);
  }

  function clearRuleForm() {
    setRuleError("");
    setRuleForm(DEFAULT_RULE_FORM);
  }

  function clearReview() {
    setError("");
    setForm(DEFAULT_FORM);
    setReview(null);
  }

  function unloadReview() {
    setError("");
    setReview(null);
  }

  if (!ready) {
    return (
      <ProductV2AuthGate
        apiBase={API}
        auth={auth}
        keyPrefix="tg-"
        productKey="trust-gate"
        productName="TrustGate"
      />
    );
  }

  return (
    <ProductV2Shell authConfigured={authConfigured} productKey="trust-gate" productName="TrustGate" runtime={runtime}>
      <DeckBar
        activeTab={activeTab}
        brandEyebrow="PatchHive"
        brandName="TrustGate"
        navLabel="TrustGate navigation"
        onTabChange={setActiveTab}
        productKey="trust-gate"
        tabs={TABS}
      />
      {activeTab === "review" && (
        <ReviewSurface
          error={error}
          form={form}
          health={runtime.health || {}}
          history={history}
          onChangeForm={setForm}
          onClear={clearReview}
          onClearReview={unloadReview}
          onRefreshData={() => {
            refreshTrustData();
            runtime.refresh();
          }}
          onRunGitHub={runGitHubReview}
          onRunManual={runManualReview}
          review={review}
          rules={rules}
          running={running}
        />
      )}
      {activeTab === "rules" && (
        <RulesSurface
          busy={rulesBusy}
          error={ruleError}
          health={runtime.health || {}}
          history={history}
          onApplyPack={applyPack}
          onClearReview={unloadReview}
          onClearRuleForm={clearRuleForm}
          onDeleteRules={deleteRules}
          onRefresh={refreshTrustData}
          onSaveRules={saveRules}
          onSetRuleForm={setRuleForm}
          packs={packs}
          review={review}
          ruleForm={ruleForm}
          rules={rules}
        />
      )}
      {activeTab === "history" && (
        <HistorySurface
          activeReviewId={review?.id || ""}
          health={runtime.health || {}}
          history={history}
          loading={loadingHistory}
          onClearReview={unloadReview}
          onLoadReview={loadHistoryReview}
          onRefresh={refreshTrustData}
          review={review}
        />
      )}
      {activeTab === "checks" && <ChecksSurface history={history} onClearReview={unloadReview} review={review} runtime={runtime} />}
    </ProductV2Shell>
  );
}
