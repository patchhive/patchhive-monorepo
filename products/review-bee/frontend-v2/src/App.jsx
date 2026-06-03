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
  { id: "threads", label: "Thread map" },
  { id: "history", label: "Review history" },
  { id: "checks", label: "Checks" },
];

const POSITIONS = [
  { left: "48%", top: "22%" },
  { left: "72%", top: "46%" },
  { left: "33%", top: "48%" },
  { left: "60%", top: "73%" },
  { left: "24%", top: "70%" },
  { left: "39%", top: "30%" },
  { left: "70%", top: "66%" },
  { left: "52%", top: "53%" },
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

function statusTone(status) {
  const normalized = String(status || "").toLowerCase();
  if (normalized === "clear" || normalized === "resolved" || normalized === "fresh") {
    return "green";
  }
  if (normalized === "attention" || normalized === "open" || normalized === "mixed") {
    return "amber";
  }
  if (normalized === "blocked" || normalized === "failed" || normalized === "error") {
    return "red";
  }
  return "signal";
}

function itemTone(item) {
  if (asCount(item.open_threads) > 0 || item.status === "open" || item.status === "mixed") {
    return "amber";
  }
  if (item.status === "resolved") {
    return "green";
  }
  return "signal";
}

function checkTone(level) {
  if (level === "error") return "red";
  if (level === "warn") return "amber";
  return "green";
}

function reportTone(report) {
  if (!report) return "signal";
  if (report.delivered) return "green";
  return report.attempted ? "amber" : "signal";
}

function healthReady(health) {
  return Boolean(health?.github_ready || health?.github?.token_configured);
}

async function parseJsonResponse(response, fallbackError) {
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(data.error || data.message || fallbackError);
  }
  return data;
}

function buildTopline(review, overview, health) {
  const openItems = review?.metrics?.open_items ?? overview?.counts?.open_items ?? 0;
  const status = review?.status || (openItems > 0 ? "attention" : "ready");
  return [
    { label: "ReviewBee", value: "Review queue", tone: "sig" },
    { label: "System", value: health?.status || "checking", tone: health?.status === "ok" ? "ok" : "warn" },
    { label: "Mode", value: "Review only" },
    { label: "GitHub", value: healthReady(health) ? "PR reads" : "token missing", tone: healthReady(health) ? "sig" : "warn" },
    { label: "Status", value: status, tone: statusTone(status) },
    { label: "Last refresh", value: review?.created_at ? timeAgo(review.created_at) : "none" },
  ];
}

function buildMetrics(review, overview, history) {
  if (review) {
    return [
      { label: "Threads read", value: String(asCount(review.metrics?.thread_count)), tone: "sig", sub: `${asCount(review.metrics?.review_count)} reviews` },
      { label: "Open asks", value: String(asCount(review.metrics?.open_items)), tone: asCount(review.metrics?.open_items) ? "warn" : "ok", sub: "active checklist" },
      { label: "Resolved", value: String(asCount(review.metrics?.resolved_items)), tone: "ok", sub: "appears addressed" },
      { label: "Reviewers", value: String(asCount(review.metrics?.reviewer_count)), tone: "sig", sub: "humans in play" },
      { label: "Checklist", value: String(review.checklist?.length || 0), tone: review.checklist?.length ? "warn" : "ok", sub: review.github_report?.state || "local result" },
    ];
  }

  return [
    { label: "Stored reviews", value: String(asCount(overview?.counts?.reviews)), tone: "sig", sub: `${history.length} loaded` },
    { label: "Repos seen", value: String(asCount(overview?.counts?.repos)), tone: "sig", sub: "review history" },
    { label: "Open asks", value: String(asCount(overview?.counts?.open_items)), tone: asCount(overview?.counts?.open_items) ? "warn" : "ok", sub: "across saved runs" },
    { label: "Latest runs", value: String(history.length), tone: "ok", sub: "history window" },
    { label: "Checklist", value: overview?.recent_reviews?.length ? "ready" : "empty", tone: overview?.recent_reviews?.length ? "sig" : "warn", sub: "run one PR" },
  ];
}

function buildRailSections(review, history, overview) {
  const pullRequests = review
    ? [{ label: `${review.repo}#${review.pr_number}`, active: true, pin: true }]
    : history.slice(0, 4).map((item, index) => ({
      label: `${item.repo}#${item.pr_number}`,
      active: index === 0,
      value: item.status || "saved",
    }));
  const latest = history[0] || {};
  const openItems = review?.metrics?.open_items ?? overview?.counts?.open_items ?? 0;
  const resolvedItems = review?.metrics?.resolved_items ?? latest.resolved_items ?? 0;
  const threadCount = review?.metrics?.thread_count ?? history.length;
  const reviewerCount = review?.metrics?.reviewer_count ?? latest.reviewer_count ?? 0;

  return [
    {
      title: "Pull requests",
      items: pullRequests.length ? pullRequests : [{ label: "No review runs yet", active: true, value: "standby" }],
    },
    {
      title: "Review state",
      items: [
        { label: "open asks", active: true, badge: String(asCount(openItems)), badgeTone: asCount(openItems) ? "amber" : "green" },
        { label: "resolved", badge: String(asCount(resolvedItems)), badgeTone: "green" },
        { label: "threads", badge: String(asCount(threadCount)), badgeTone: "signal" },
        { label: "reviewers", badge: String(asCount(reviewerCount)), badgeTone: "signal" },
      ],
    },
  ];
}

function buildRailStats(review, overview) {
  return {
    title: "Active PR",
    items: [
      { label: "Repository", value: review?.repo || overview?.recent_reviews?.[0]?.repo || "none" },
      { label: "Pressure", value: review?.status ? review.status.toUpperCase() : "READY", large: true, tone: statusTone(review?.status || "ready") },
      { label: "Comment", value: review?.github_report?.state || "local" },
    ],
  };
}

function countHistoryStatus(history, status) {
  return history.filter((item) => String(item.status || "").toLowerCase() === status).length;
}

function tabRailSections({ checks = [], health = {}, history = [], overview = null, review = null }) {
  health = health || {};
  return {
    checks: [
      {
        title: "Runtime",
        items: [
          { label: "backend", active: true, badge: health.status || "unknown", badgeTone: health.status === "ok" ? "green" : "amber" },
          { label: "github", badge: healthReady(health) ? "ready" : "missing", badgeTone: healthReady(health) ? "green" : "red" },
          { label: "webhook", badge: health.github?.webhook_ready ? "ready" : "optional", badgeTone: health.github?.webhook_ready ? "green" : "amber" },
          { label: "auth", badge: health.auth_enabled ? "on" : "open", badgeTone: health.auth_enabled ? "green" : "amber" },
        ],
      },
      {
        title: "Review memory",
        items: [
          { label: "startup checks", badge: String(checks.length), badgeTone: checks.length ? "signal" : "green" },
          { label: "saved reviews", badge: String(asCount(health.review_count || overview?.counts?.reviews)), badgeTone: "signal" },
          { label: "repos", badge: String(asCount(health.repo_count || overview?.counts?.repos)), badgeTone: "signal" },
        ],
      },
    ],
    history: [
      {
        title: "Review state",
        items: [
          { label: "clear", active: review?.status === "clear", badge: String(countHistoryStatus(history, "clear")), badgeTone: "green" },
          { label: "attention", active: review?.status === "attention", badge: String(countHistoryStatus(history, "attention")), badgeTone: "amber" },
          { label: "blocked", active: review?.status === "blocked", badge: String(countHistoryStatus(history, "blocked")), badgeTone: "red" },
        ],
      },
      {
        title: "Saved pressure",
        items: [
          { label: "reviews", badge: String(history.length), badgeTone: "signal" },
          { label: "open asks", badge: String(asCount(overview?.counts?.open_items)), badgeTone: asCount(overview?.counts?.open_items) ? "amber" : "green" },
          { label: "current PR", value: review ? `${review.repo}#${review.pr_number}` : "none" },
        ],
      },
    ],
  };
}

function tabRailStats({ health = {}, history = [], overview = null, review = null }, tab) {
  health = health || {};
  const latest = history[0] || overview?.recent_reviews?.[0] || {};
  if (tab === "checks") {
    return {
      title: "Backend",
      items: [
        { label: "Status", value: health.status || "unknown", large: true, tone: health.status === "ok" ? "ok" : "warn" },
        { label: "Database", value: health.db_ok ? "ok" : "check" },
        { label: "GitHub", value: healthReady(health) ? "ready" : "missing" },
      ],
    };
  }
  return {
    title: "Selected review",
    items: [
      { label: "Repository", value: review?.repo || latest.repo || "none" },
      { label: "Pressure", value: String(review?.status || latest.status || "ready").toUpperCase(), large: true, tone: statusTone(review?.status || latest.status || "ready") },
      { label: "Age", value: review?.created_at ? timeAgo(review.created_at) : latest.created_at ? timeAgo(latest.created_at) : "none" },
    ],
  };
}

function TabFrame({ children, health, overview, railSections, railStats, review }) {
  return (
    <>
      <SuiteTopline cells={buildTopline(review, overview, health)} />
      <div className="main-grid hive-workspace-grid">
        <ProductRail sections={railSections} stats={railStats} />
        <main className="workspace">{children}</main>
      </div>
    </>
  );
}

function currentReviewRadarItem(review) {
  const status = review?.status || (asCount(review?.metrics?.open_items) ? "attention" : "clear");
  return {
    id: review?.id || "current-review",
    title: `${String(status).toUpperCase()} review`,
    detail: review?.repo ? `${review.repo}#${review.pr_number}` : "Current PR review",
    gain: status,
    gainMeta: `${asCount(review?.metrics?.open_items)} open / ${asCount(review?.metrics?.resolved_items)} resolved`,
    label: status,
    minWindow: 7,
    position: { left: "50%", top: "44%" },
    stats: [
      { label: "Repo", value: review?.repo || "repo" },
      { label: "PR", value: review?.pr_number ? `#${review.pr_number}` : "none" },
      { label: "Open", value: String(asCount(review?.metrics?.open_items)) },
      { label: "Resolved", value: String(asCount(review?.metrics?.resolved_items)) },
      { label: "Reviewers", value: String(asCount(review?.metrics?.reviewer_count)) },
    ],
    summary: review?.summary || "ReviewBee current PR result.",
    tone: statusTone(status),
    vector: review?.pr_number ? `PR-${review.pr_number}` : "CURRENT",
    vectorTone: statusTone(status) === "amber" ? "warn" : "",
  };
}

function buildRadarItems(review, history, overview) {
  if (review) {
    const items = [currentReviewRadarItem(review)];
    const checklistItems = (review.checklist || []).map((item, index) => ({
      id: item.key || `checklist-${index + 1}`,
      title: item.title || `Checklist item ${index + 1}`,
      detail: item.title || item.category || "Checklist item",
      gain: item.status || "open",
      gainMeta: `${asCount(item.open_threads)} open / ${asCount(item.resolved_threads)} resolved`,
      label: item.key || `C${index + 1}`,
      minWindow: index < 3 ? 7 : index < 6 ? 14 : 30,
      position: POSITIONS[(index + 1) % POSITIONS.length],
      stats: [
        { label: "Category", value: item.category || "review" },
        { label: "Status", value: item.status || "open" },
        { label: "Open", value: String(asCount(item.open_threads)) },
        { label: "Resolved", value: String(asCount(item.resolved_threads)) },
        { label: "Comments", value: String(asCount(item.comment_count) || item.evidence?.length || 0) },
      ],
      summary: item.summary || item.prompt_hint || "ReviewBee grouped this review feedback into a checklist item.",
      tone: itemTone(item),
      vector: item.key || `ITEM-${index + 1}`,
      vectorTone: itemTone(item) === "amber" ? "warn" : "",
    }));
    return [...items, ...checklistItems];
  }

  if (history.length) {
    return history.slice(0, 8).map((item, index) => ({
      id: item.id || `history-${index + 1}`,
      title: `${item.repo}#${item.pr_number}`,
      detail: item.pr_title || `${item.repo} PR #${item.pr_number}`,
      gain: item.status || "saved",
      gainMeta: `${asCount(item.open_items)} open / ${asCount(item.resolved_items)} resolved`,
      label: `PR${item.pr_number}`,
      minWindow: index < 3 ? 7 : index < 6 ? 14 : 30,
      position: POSITIONS[index % POSITIONS.length],
      stats: [
        { label: "Repo", value: item.repo },
        { label: "PR", value: `#${item.pr_number}` },
        { label: "Open", value: String(asCount(item.open_items)) },
        { label: "Resolved", value: String(asCount(item.resolved_items)) },
        { label: "Reviewers", value: String(asCount(item.reviewer_count)) },
      ],
      summary: item.summary || "Saved ReviewBee run.",
      tone: statusTone(item.status),
      vector: item.status || "saved",
      vectorTone: statusTone(item.status) === "amber" ? "warn" : "",
    }));
  }

  return [{
    id: "reviewbee-ready",
    title: "ReviewBee ready",
    detail: "No PR loaded yet",
    gain: "standby",
    gainMeta: `${asCount(overview?.counts?.reviews)} saved runs`,
    label: "RB",
    position: { left: "50%", top: "44%" },
    stats: [
      { label: "Reviews", value: String(asCount(overview?.counts?.reviews)) },
      { label: "Repos", value: String(asCount(overview?.counts?.repos)) },
      { label: "Open asks", value: String(asCount(overview?.counts?.open_items)) },
      { label: "Mode", value: "review only" },
      { label: "Action", value: "run PR" },
    ],
    summary: overview?.tagline || "Run ReviewBee on a GitHub pull request to populate the review queue.",
    tone: "signal",
    vector: "READY",
  }];
}

function buildRadarFeed(review, overview, health) {
  if (review) {
    const report = review.github_report;
    return [
      { text: review.summary || "ReviewBee completed the PR review pass.", tone: statusTone(review.status) },
      { text: `${asCount(review.metrics?.open_items)} open asks and ${asCount(review.metrics?.resolved_items)} resolved items are visible in the checklist.`, tone: asCount(review.metrics?.open_items) ? "amber" : "green" },
      { text: report?.message || "GitHub comment output is local until publishing is enabled.", tone: reportTone(report) },
    ];
  }

  return [
    { text: overview?.tagline || "ReviewBee turns PR review threads into a concrete checklist.", tone: "signal" },
    { text: `${asCount(overview?.counts?.open_items)} open review items across saved runs.`, tone: asCount(overview?.counts?.open_items) ? "amber" : "green" },
    { text: healthReady(health) ? "GitHub token is ready for PR review reads." : "Configure GitHub token access before reviewing live PRs.", tone: healthReady(health) ? "green" : "amber" },
  ];
}

function ReviewForm({ error, form, onChange, onRun, running }) {
  return (
    <Panel eyebrow="Review input" title="Review a GitHub PR" action={<span className="chip signal">review only</span>}>
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
            <input
              className="v2-input"
              onChange={(event) => onChange((current) => ({ ...current, repo: event.target.value }))}
              placeholder="owner/repo"
              value={form.repo}
            />
          </label>
          <label className="v2-field">
            PR number
            <input
              className="v2-input"
              inputMode="numeric"
              onChange={(event) => onChange((current) => ({ ...current, pr_number: event.target.value }))}
              placeholder="123"
              value={form.pr_number}
            />
          </label>
          <div className="v2-field">
            Action
            <button className="btn primary" disabled={running} type="submit">
              {running ? "Reading reviews..." : "Run ReviewBee"}
            </button>
          </div>
        </div>
        <label className="rowline" style={{ alignItems: "flex-start", justifyContent: "flex-start" }}>
          <input
            checked={Boolean(form.publish_comment)}
            onChange={(event) => onChange((current) => ({ ...current, publish_comment: event.target.checked }))}
            style={{ marginTop: 3 }}
            type="checkbox"
          />
          <span>
            <span className="repo-name" style={{ display: "block", fontSize: "0.8rem" }}>Maintain PR comment</span>
            <span className="feed-meta">Optional write-back. Requires comment permission on this repository.</span>
          </span>
        </label>
        {error && <div className="status-banner red">{error}</div>}
      </form>
    </Panel>
  );
}

function ThreadMap({ health, history, overview, review }) {
  const items = useMemo(() => buildRadarItems(review, history, overview), [review, history, overview]);
  const feed = useMemo(() => buildRadarFeed(review, overview, health), [review, overview, health]);

  return (
    <SuiteRadar
      ariaLabel="ReviewBee thread radar"
      detailLabel={review ? "Checklist detail" : "Review detail"}
      feed={feed}
      gainLabel={review ? "Item state" : "Run state"}
      itemQueryParam="review"
      items={items}
      signalLabel={review ? "items" : "runs"}
      vectorLabel={review ? "Selected item" : "Selected run"}
    />
  );
}

function ChecklistPanel({ history, onLoadReview, review }) {
  const rows = review?.checklist || [];
  if (review) {
    return (
      <Panel eyebrow="Checklist" title="Action groups" action={<span className={`chip ${asCount(review.metrics?.open_items) ? "amber" : "green"}`}>{asCount(review.metrics?.open_items)} open</span>}>
        <div className="panelbody repo-list queue-grid">
          {rows.length ? rows.map((item, index) => (
            <div className="ledger-row" key={item.key || item.title || index}>
              <div className="rank">{String(index + 1).padStart(2, "0")}</div>
              <div>
                <div className="repo-name">{item.title || "Review ask"}</div>
                <div className="feed-meta">{item.summary || item.prompt_hint || "Grouped reviewer feedback."}</div>
                <div className="repo-meta">
                  <span className="chip signal">{item.category || "review"}</span>
                  <span className="chip amber">{asCount(item.open_threads)} open</span>
                  <span className="chip green">{asCount(item.resolved_threads)} resolved</span>
                </div>
              </div>
              <span className={`chip ${itemTone(item)}`}>{item.status || "open"}</span>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>Clear review</strong>
              <span>ReviewBee did not find actionable checklist items for this PR.</span>
            </div>
          )}
        </div>
      </Panel>
    );
  }

  return (
    <Panel eyebrow="Checklist" title="Recent review runs" action={<span className="chip signal">{history.length} saved</span>}>
      <div className="panelbody repo-list queue-grid">
        {history.length ? history.slice(0, 5).map((item, index) => (
          <div className="ledger-row" key={item.id || `${item.repo}-${item.pr_number}`}>
            <div className="rank">{String(index + 1).padStart(2, "0")}</div>
            <div>
              <div className="repo-name">{item.repo}#{item.pr_number}</div>
              <div className="feed-meta">{item.summary || item.pr_title || "Saved ReviewBee run."}</div>
              <div className="repo-meta">
                <span className={`chip ${statusTone(item.status)}`}>{item.status || "saved"}</span>
                <span className="chip amber">{asCount(item.open_items)} open</span>
                <span className="chip green">{asCount(item.resolved_items)} resolved</span>
              </div>
            </div>
            <button className="btn" onClick={() => onLoadReview(item.id)} type="button">Load</button>
          </div>
        )) : (
          <div className="empty-v2">
            <strong>No reviews yet</strong>
            <span>Run ReviewBee on a pull request and the checklist history will appear here.</span>
          </div>
        )}
      </div>
    </Panel>
  );
}

function SidePanels({ health, onCopyReport, onCopyPrompts, review }) {
  const reviewers = review?.reviewers || [];
  const report = review?.github_report;
  return (
    <aside className="side">
      <Panel eyebrow="Evidence" title="Review pressure">
        <div className="panelbody repo-list">
          <div className="feed-item">
            <div>
              <div className="feed-title">Status</div>
              <div className="feed-meta">{review?.summary || "Run a PR review to see current pressure."}</div>
            </div>
            <span className={`chip ${statusTone(review?.status || "ready")}`}>{review?.status || "ready"}</span>
          </div>
          <div className="feed-item">
            <div>
              <div className="feed-title">Reviewers</div>
              <div className="feed-meta">{reviewers.length ? reviewers.join(", ") : "No review result loaded."}</div>
            </div>
            <span className="chip signal">{reviewers.length}</span>
          </div>
          <div className="feed-item">
            <div>
              <div className="feed-title">Requested changes</div>
              <div className="feed-meta">Review decisions from the PR thread history.</div>
            </div>
            <span className="chip amber">{asCount(review?.metrics?.requested_changes_reviews)}</span>
          </div>
        </div>
      </Panel>
      <Panel eyebrow="Output" title="Comment posture">
        <div className="panelbody repo-list">
          <div className="rowline"><span className="muted">Maintained comment</span><span className={`chip ${reportTone(report)}`}>{report?.state || "local"}</span></div>
          <div className="rowline"><span className="muted">Webhook refresh</span><span className={`chip ${health?.github?.webhook_ready ? "green" : "amber"}`}>{health?.github?.webhook_ready ? "ready" : "optional"}</span></div>
          <div className="rowline"><span className="muted">GitHub token</span><span className={`chip ${healthReady(health) ? "green" : "red"}`}>{healthReady(health) ? "ready" : "missing"}</span></div>
          {review?.pr_url && <button className="btn" onClick={() => window.open(review.pr_url, "_blank", "noreferrer")} type="button">Open PR</button>}
          {report?.comment_url && <button className="btn" onClick={() => window.open(report.comment_url, "_blank", "noreferrer")} type="button">Open comment</button>}
          {report?.report_markdown && <button className="btn" onClick={onCopyReport} type="button">Copy report</button>}
        </div>
      </Panel>
      {review?.prompt_suggestions?.length > 0 && (
        <Panel eyebrow="Prompts" title="Follow-up prompts" action={<button className="btn" onClick={onCopyPrompts} type="button">Copy</button>}>
          <div className="panelbody repo-list">
            {review.prompt_suggestions.slice(0, 3).map((prompt, index) => (
              <div className="feed-item" key={`${prompt}-${index}`}>
                <div>
                  <div className="feed-title">Prompt {index + 1}</div>
                  <div className="feed-meta">{prompt}</div>
                </div>
              </div>
            ))}
          </div>
        </Panel>
      )}
    </aside>
  );
}

function ThreadSurface({
  error,
  form,
  health,
  history,
  onChangeForm,
  onClearReview,
  onCopyPrompts,
  onCopyReport,
  onLoadReview,
  onRefreshData,
  onRunReview,
  overview,
  review,
  running,
}) {
  const metrics = useMemo(() => buildMetrics(review, overview, history), [review, overview, history]);
  const railSections = useMemo(() => buildRailSections(review, history, overview), [review, history, overview]);
  const railStats = useMemo(() => buildRailStats(review, overview), [review, overview]);
  const topline = useMemo(() => buildTopline(review, overview, health), [review, overview, health]);

  return (
    <>
      <SuiteTopline cells={topline} />
      <div className="main-grid">
        <ProductRail sections={railSections} stats={railStats} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - review resolution</div>
              <h1>Thread Map</h1>
              <p className="subline">Run a GitHub PR review, collapse review thread noise, and keep the actionable checklist visible.</p>
            </div>
            <div className="actions">
              <span className={`chip ${statusTone(review?.status || "ready")}`}>{review?.status || "ready"}</span>
              <span className={`chip ${healthReady(health) ? "green" : "amber"}`}>{healthReady(health) ? "github ready" : "github missing"}</span>
              {review && <button className="btn" onClick={onClearReview} type="button">Clear review</button>}
              <button className="btn" onClick={onRefreshData} type="button">Refresh data</button>
            </div>
          </div>
          <ReviewForm error={error} form={form} onChange={onChangeForm} onRun={onRunReview} running={running} />
          <MetricBand metrics={metrics} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Threads" title="Resolution map" action={<span className="chip signal">thread radar</span>}>
              <ThreadMap health={health} history={history} overview={overview} review={review} />
            </Panel>
            <ChecklistPanel history={history} onLoadReview={onLoadReview} review={review} />
          </div>
        </main>
        <SidePanels health={health} onCopyPrompts={onCopyPrompts} onCopyReport={onCopyReport} review={review} />
      </div>
    </>
  );
}

function HistorySurface({ activeReviewId, health, history, loading, onClearReview, onLoadReview, onRefresh, overview, review }) {
  const railSections = useMemo(() => tabRailSections({ health, history, overview, review }).history, [health, history, overview, review]);
  const railStats = useMemo(() => tabRailStats({ health, history, overview, review }, "history"), [health, history, overview, review]);

  return (
    <TabFrame health={health} overview={overview} railSections={railSections} railStats={railStats} review={review}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// ReviewBee review queue</div>
          <h1>Review History</h1>
          <p className="subline">Saved PR review runs, unresolved pressure, and checklist outcomes over time.</p>
        </div>
        <div className="actions">
          {review && <button className="btn" onClick={onClearReview} type="button">Clear review</button>}
          <button className="btn" onClick={onRefresh} type="button">{loading ? "Refreshing..." : "Refresh"}</button>
        </div>
      </div>
      <Panel eyebrow="Recent" title="Review runs" action={<span className="chip signal">{history.length} saved</span>}>
        <div className="panelbody repo-list queue-grid">
          {history.length ? history.map((item) => (
            <div className="ledger-row" key={item.id || `${item.repo}-${item.pr_number}`}>
              <div className="rank">{item.id === activeReviewId ? "SEL" : `#${item.pr_number}`}</div>
              <div>
                <div className="repo-name">{item.repo} - PR #{item.pr_number}</div>
                <div className="feed-meta">{item.summary || item.pr_title || "Saved ReviewBee run."}</div>
                <div className="repo-meta">
                  <span className={`chip ${statusTone(item.status)}`}>{item.status || "saved"}</span>
                  <span className="chip amber">{asCount(item.open_items)} open</span>
                  <span className="chip green">{asCount(item.resolved_items)} resolved</span>
                  <span className="chip signal">{asCount(item.reviewer_count)} reviewers</span>
                  <span className="chip">{timeAgo(item.created_at)}</span>
                </div>
              </div>
              <button className="btn" onClick={() => onLoadReview(item.id)} type="button">Load</button>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>No review history</strong>
              <span>Run ReviewBee on a pull request and saved checklists will appear here.</span>
            </div>
          )}
        </div>
      </Panel>
    </TabFrame>
  );
}

function ChecksSurface({ history, onClearReview, overview, review, runtime }) {
  const health = runtime.health || {};
  const checks = runtime.checks || [];
  const checkWarnings = checks.filter((check) => check.level === "warn" || check.level === "error").length;
  const railSections = useMemo(() => tabRailSections({ checks, health, history, overview, review }).checks, [checks, health, history, overview, review]);
  const railStats = useMemo(() => tabRailStats({ health, history, overview, review }, "checks"), [health, history, overview, review]);
  const metrics = [
    { label: "Status", value: health.status || "unknown", tone: health.status === "ok" ? "ok" : "warn", sub: health.version || "backend" },
    { label: "GitHub", value: healthReady(health) ? "ready" : "missing", tone: healthReady(health) ? "ok" : "hot", sub: "PR reads" },
    { label: "Webhook", value: health.github?.webhook_ready ? "ready" : "optional", tone: health.github?.webhook_ready ? "ok" : "warn", sub: "auto refresh" },
    { label: "Reviews", value: String(asCount(health.review_count)), tone: "sig", sub: `${asCount(health.repo_count)} repos` },
    { label: "Checks", value: checkWarnings ? String(checkWarnings) : "clear", tone: checkWarnings ? "warn" : "ok", sub: "startup" },
  ];

  return (
    <TabFrame health={health} overview={overview} railSections={railSections} railStats={railStats} review={review}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// ReviewBee readiness</div>
          <h1>Checks</h1>
          <p className="subline">Backend health, GitHub review access, webhook posture, and startup checks before a PR review run.</p>
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
            <div className="rowline"><span className="muted">Comment publish</span><span className={`chip ${health.github?.comment_publish_ready ? "green" : "amber"}`}>{health.github?.comment_publish_ready ? "ready" : "limited"}</span></div>
            <div className="rowline"><span className="muted">Webhook secret</span><span className={`chip ${health.github?.webhook_secret_configured ? "green" : "amber"}`}>{health.github?.webhook_secret_configured ? "configured" : "missing"}</span></div>
            <div className="rowline"><span className="muted">Public URL</span><span className={`chip ${health.github?.public_url_configured ? "green" : "amber"}`}>{health.github?.public_url_configured ? "configured" : "missing"}</span></div>
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
  const [activeTab, setActiveTab] = usePersistentProductTab("review-bee", TABS, "threads");
  const [error, setError] = useState("");
  const [form, setForm] = useState({ repo: "", pr_number: "", publish_comment: false });
  const [history, setHistory] = useState([]);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [overview, setOverview] = useState(null);
  const [review, setReview] = useState(null);
  const [running, setRunning] = useState(false);
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "review-bee_api_key" });
  const fetch_ = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const ready = auth.checked && !auth.needsAuth;
  const runtime = useProductRuntime({ apiBase: API, fetcher: fetch_, ready });
  const authConfigured = Boolean(runtime.authStatus?.auth_configured || runtime.health?.auth_enabled);

  async function fetchJson(path, options, fallbackError) {
    const response = await fetch_(`${API}${path}`, options);
    return parseJsonResponse(response, fallbackError);
  }

  async function refreshProductData() {
    if (!ready) {
      return;
    }
    setLoadingHistory(true);
    const [overviewResult, historyResult] = await Promise.allSettled([
      fetchJson("/overview", undefined, "ReviewBee could not load the overview."),
      fetchJson("/history", undefined, "ReviewBee could not load history."),
    ]);
    setOverview(overviewResult.status === "fulfilled" ? overviewResult.value : null);
    setHistory(historyResult.status === "fulfilled" ? historyResult.value : []);
    setLoadingHistory(false);
  }

  useEffect(() => {
    refreshProductData();
  }, [ready, fetch_]);

  async function runReview() {
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(
        "/review/github/pr",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            repo: form.repo,
            pr_number: Number(form.pr_number) || 0,
            publish_comment: Boolean(form.publish_comment),
          }),
        },
        "ReviewBee could not review that pull request.",
      );
      setReview(result);
      setForm({
        repo: result.repo || form.repo,
        pr_number: result.pr_number ? String(result.pr_number) : form.pr_number,
        publish_comment: Boolean(form.publish_comment),
      });
      setActiveTab("threads");
      await refreshProductData();
      await runtime.refresh();
    } catch (err) {
      setError(err.message || "ReviewBee could not review that pull request.");
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
      const result = await fetchJson(`/history/${id}`, undefined, "ReviewBee could not load that review.");
      setReview(result);
      setForm({
        repo: result.repo || "",
        pr_number: result.pr_number ? String(result.pr_number) : "",
        publish_comment: false,
      });
      setActiveTab("threads");
    } catch (err) {
      setError(err.message || "ReviewBee could not load that review.");
    } finally {
      setRunning(false);
    }
  }

  function clearReview() {
    setReview(null);
    setError("");
  }

  async function copyPrompts() {
    if (!review?.prompt_suggestions?.length || !navigator?.clipboard) {
      return;
    }
    await navigator.clipboard.writeText(review.prompt_suggestions.join("\n"));
  }

  async function copyReport() {
    const report = review?.github_report?.report_markdown;
    if (!report || !navigator?.clipboard) {
      return;
    }
    await navigator.clipboard.writeText(report);
  }

  if (!ready) {
    return (
      <ProductV2AuthGate
        apiBase={API}
        auth={auth}
        keyPrefix="review-bee-"
        productKey="review-bee"
        productName="ReviewBee"
      />
    );
  }

  return (
    <ProductV2Shell authConfigured={authConfigured} runtime={runtime}>
      <DeckBar
        activeTab={activeTab}
        brandEyebrow="PatchHive"
        brandName="ReviewBee"
        navLabel="ReviewBee navigation"
        onTabChange={setActiveTab}
        productKey="review-bee"
        tabs={TABS}
      />
      {activeTab === "threads" && (
        <ThreadSurface
          error={error}
          form={form}
          health={runtime.health || {}}
          history={history}
          onChangeForm={setForm}
          onClearReview={clearReview}
          onCopyPrompts={copyPrompts}
          onCopyReport={copyReport}
          onLoadReview={loadHistoryReview}
          onRefreshData={() => {
            refreshProductData();
            runtime.refresh();
          }}
          onRunReview={runReview}
          overview={overview}
          review={review}
          running={running}
        />
      )}
      {activeTab === "history" && (
        <HistorySurface
          activeReviewId={review?.id || ""}
          health={runtime.health || {}}
          history={history}
          loading={loadingHistory}
          onClearReview={clearReview}
          onLoadReview={loadHistoryReview}
          onRefresh={refreshProductData}
          overview={overview}
          review={review}
        />
      )}
      {activeTab === "checks" && <ChecksSurface history={history} onClearReview={clearReview} overview={overview} review={review} runtime={runtime} />}
    </ProductV2Shell>
  );
}
