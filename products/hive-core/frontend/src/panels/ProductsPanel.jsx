import { useEffect, useMemo, useState } from "react";
import { Btn, EmptyState, S, Tag } from "@patchhivehq/ui";

const payloadStyle = {
  ...S.input,
  minHeight: 118,
  resize: "vertical",
  fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
  lineHeight: 1.5,
};

function statusColor(status) {
  if (status === "online") return "var(--green)";
  if (status === "degraded") return "var(--gold)";
  if (status === "disabled") return "var(--text-dim)";
  if (status === "unconfigured") return "var(--blue)";
  return "var(--accent)";
}

function actionColor(status) {
  if (status === "dispatched") return "var(--green)";
  if (status === "failed") return "var(--accent)";
  return "var(--gold)";
}

function contractColor(check) {
  if (check?.ok) return "var(--green)";
  if (["locked", "skipped", "disabled", "unconfigured"].includes(check?.status)) return "var(--gold)";
  return "var(--accent)";
}

function runColor(status) {
  const normalized = String(status || "").toLowerCase();
  if (["completed", "success", "safe", "ready", "dispatched"].includes(normalized)) return "var(--green)";
  if (["running", "queued", "hold", "warn"].includes(normalized)) return "var(--gold)";
  if (["failed", "blocked", "block", "cancelled"].includes(normalized)) return "var(--accent)";
  return "var(--blue)";
}

function pretty(value) {
  return JSON.stringify(value, null, 2);
}

function shortJson(value, maxLength = 2200) {
  const text = pretty(value ?? null);
  return text.length > maxLength ? `${text.slice(0, maxLength)}\n...` : text;
}

function defaultActionRequest(product, action) {
  if (!action) return {};
  if (action.path?.includes("{name}")) {
    return { path_params: { name: "daily" }, payload: {} };
  }
  if (action.id === "scan") {
    return {
      search_query: "",
      topics: [],
      languages: ["rust", "typescript", "python"],
      min_stars: 25,
      max_repos: 4,
      issues_per_repo: 20,
      stale_days: 45,
    };
  }
  if (["scan_github_dependencies", "scan_github_actions", "scan_github_findings"].includes(action.id)) {
    return { repo: "owner/repo" };
  }
  if (action.id === "scan_local_repo") {
    return { repo_path: "/path/to/repo", max_files: 250 };
  }
  if (action.id === "review_diff") {
    return { repo: "owner/repo", diff: "diff --git a/file b/file\n" };
  }
  if (["review_github_pr", "assess_github_pr"].includes(action.id)) {
    return { repo: "owner/repo", pr_number: 1 };
  }
  if (action.id === "context") {
    return { repo: "owner/repo", query: "testing conventions", max_items: 8 };
  }
  if (action.id === "capture_failguard_lesson") {
    return {
      repo: "owner/repo",
      title: "Webhook endpoints must fail closed without a secret",
      outcome: "Unsigned webhook payloads could trigger protected automation.",
      lesson: "Public webhook routes must not accept unsigned payloads or silently skip signature verification.",
      prevention: "Reject webhook delivery when the signing secret is missing.",
      affected_paths: ["backend/src/routes/webhook.rs"],
      evidence: ["FailGuard lesson captured by the operator"],
      disposition: "policy",
      pinned: true,
    };
  }
  if (action.id === "suggest_failguard_candidate") {
    return {
      repo: "owner/repo",
      source_type: "repo-reaper-rejection",
      source_ref: "run-id-or-pr-url",
      title: "Generated patch skipped webhook signing",
      outcome: "Smith rejected a generated patch because webhook verification failed open.",
      affected_paths: ["backend/src/routes/webhook.rs"],
      evidence: ["RepoReaper rejection summary"],
    };
  }
  if (action.id === "ingest") {
    return { repo: "owner/repo" };
  }
  if (["run", "dry_run"].includes(action.id)) {
    return {
      query: "language:Rust label:bug",
      languages: ["rust"],
      max_repos: 3,
      max_issues: 3,
    };
  }
  return {};
}

function parsePayload(text) {
  if (!text.trim()) return {};
  return JSON.parse(text);
}

function RecentActivity({ events }) {
  return (
    <div style={{ ...S.panel, display: "grid", gap: 10 }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
        <div>
          <div style={{ fontSize: 15, fontWeight: 800 }}>Suite Activity</div>
          <div style={{ fontSize: 11, color: "var(--text-dim)" }}>
            HiveCore dispatches are recorded here as the suite control loop warms up.
          </div>
        </div>
        <Tag color="var(--blue)">{events.length} recent</Tag>
      </div>
      {events.length === 0 ? (
        <div style={{ fontSize: 11, color: "var(--text-dim)" }}>No product actions have been dispatched yet.</div>
      ) : (
        <div style={{ display: "grid", gap: 8 }}>
          {events.slice(0, 6).map((event) => (
            <div
              key={event.id}
              style={{
                display: "grid",
                gap: 4,
                padding: "9px 10px",
                border: "1px solid var(--border)",
                borderRadius: 8,
                background: "rgba(255,255,255,0.025)",
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", gap: 8, flexWrap: "wrap" }}>
                <strong style={{ fontSize: 12 }}>
                  {event.product_slug} · {event.action_label || event.action_id}
                </strong>
                <Tag color={actionColor(event.status)}>
                  {event.status}{event.remote_status ? ` · ${event.remote_status}` : ""}
                </Tag>
              </div>
              <div style={{ fontSize: 10, color: "var(--text-dim)" }}>{event.created_at}</div>
              {event.error && <div style={{ fontSize: 11, color: "var(--gold)" }}>{event.error}</div>}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function ContractChecklist({ product }) {
  const checks = product.contract_checks || [];
  if (checks.length === 0) return null;

  return (
    <div style={{ borderTop: "1px solid var(--border)", paddingTop: 12, display: "grid", gap: 8 }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center" }}>
        <div style={{ fontSize: 12, fontWeight: 800 }}>Contract</div>
        <Tag color={product.contract_drift_count > 0 ? "var(--accent)" : "var(--green)"}>
          {product.contract_drift_count || 0} drift
        </Tag>
      </div>
      <div style={{ display: "grid", gap: 6 }}>
        {checks.map((check) => (
          <div
            key={check.id}
            style={{
              display: "grid",
              gridTemplateColumns: "minmax(88px, 0.85fr) minmax(80px, 1fr) auto",
              gap: 8,
              alignItems: "center",
              fontSize: 10,
              color: "var(--text-dim)",
            }}
          >
            <strong style={{ color: "var(--text)" }}>{check.label}</strong>
            <span style={{ wordBreak: "break-all" }}>{check.path}</span>
            <Tag color={contractColor(check)}>{check.status}</Tag>
            {check.error && (
              <div style={{ gridColumn: "1 / -1", color: "var(--gold)", lineHeight: 1.45 }}>
                {check.error}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

function RunHistory({ product, onOpenRun }) {
  const runs = product.recent_runs || [];
  if (!product.enabled) {
    return (
      <div style={{ borderTop: "1px solid var(--border)", paddingTop: 12, fontSize: 11, color: "var(--text-dim)" }}>
        Product is disabled in HiveCore Settings.
      </div>
    );
  }

  return (
    <div style={{ borderTop: "1px solid var(--border)", paddingTop: 12, display: "grid", gap: 8 }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center" }}>
        <div>
          <div style={{ fontSize: 12, fontWeight: 800 }}>Recent Runs</div>
          <div style={{ fontSize: 10, color: "var(--text-dim)" }}>
            Latest product-owned work.
          </div>
        </div>
        <Tag color={product.health?.runs_ok ? "var(--green)" : "var(--gold)"}>
          {product.health?.runs_ok ? `${runs.length} visible` : "locked"}
        </Tag>
      </div>
      {!product.health?.runs_ok ? (
        <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.5 }}>
          {product.health?.runs_error || "Save this product's API key in Settings to show run history."}
        </div>
      ) : runs.length === 0 ? (
        <div style={{ fontSize: 11, color: "var(--text-dim)" }}>No run history visible yet.</div>
      ) : (
        <div style={{ display: "grid", gap: 7 }}>
          {runs.slice(0, 3).map((run) => (
            <button
              key={run.id}
              type="button"
              onClick={() => onOpenRun(product, run)}
              style={{
                display: "grid",
                gap: 3,
                padding: "8px 9px",
                border: "1px solid var(--border)",
                borderRadius: 8,
                background: "rgba(255,255,255,0.025)",
                textAlign: "left",
                cursor: "pointer",
                color: "inherit",
                font: "inherit",
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center" }}>
                <strong style={{ fontSize: 11, color: "var(--text)" }}>{run.title || run.id}</strong>
                <Tag color={runColor(run.status)}>{run.status || "unknown"}</Tag>
              </div>
              {run.summary && <div style={{ fontSize: 10, color: "var(--text-dim)", lineHeight: 1.45 }}>{run.summary}</div>}
              {(run.updated_at || run.created_at) && (
                <div style={{ fontSize: 9, color: "var(--text-muted)" }}>{run.updated_at || run.created_at}</div>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function DispatchResult({ event }) {
  return (
    <div
      style={{
        border: `1px solid ${actionColor(event.status)}55`,
        borderRadius: 8,
        padding: 10,
        display: "grid",
        gap: 8,
        fontSize: 11,
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", gap: 8, flexWrap: "wrap" }}>
        <strong style={{ color: actionColor(event.status) }}>
          {event.status}{event.remote_status ? ` · HTTP ${event.remote_status}` : ""}
        </strong>
        {event.id && <Tag color="var(--blue)">{event.id}</Tag>}
      </div>
      {event.target_url && (
        <div style={{ color: "var(--text-dim)", wordBreak: "break-all" }}>{event.target_url}</div>
      )}
      {event.error && <div style={{ color: "var(--gold)" }}>{event.error}</div>}
      {event.response_json !== undefined && event.response_json !== null && (
        <pre
          style={{
            ...payloadStyle,
            minHeight: 0,
            maxHeight: 210,
            overflow: "auto",
            margin: 0,
            whiteSpace: "pre-wrap",
          }}
        >
          {shortJson(event.response_json)}
        </pre>
      )}
    </div>
  );
}

function RunDetailPanel({ detail, loading, onClose }) {
  if (!detail && !loading) return null;
  const product = detail?.product;
  const run = detail?.run;
  const payload = detail?.data;

  return (
    <div style={{ ...S.panel, display: "grid", gap: 12, borderColor: "var(--blue)" }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start" }}>
        <div>
          <div style={{ fontSize: 16, fontWeight: 900 }}>
            {product?.title || "Product"} · {run?.title || run?.id || "Run detail"}
          </div>
          <div style={{ fontSize: 11, color: "var(--text-dim)", wordBreak: "break-all" }}>
            {payload?.detail_path || run?.detail_path || ""}
          </div>
        </div>
        <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
          {payload && (
            <Tag color={payload.detail_ok ? "var(--green)" : "var(--gold)"}>
              {payload.detail_ok ? "detail loaded" : "detail blocked"}
            </Tag>
          )}
          <Btn onClick={onClose}>Close</Btn>
        </div>
      </div>
      {loading ? (
        <div style={{ fontSize: 12, color: "var(--text-dim)" }}>Loading run detail...</div>
      ) : detail?.error ? (
        <div style={{ fontSize: 12, color: "var(--gold)" }}>{detail.error}</div>
      ) : (
        <div style={{ display: "grid", gap: 8 }}>
          {payload?.error && <div style={{ fontSize: 12, color: "var(--gold)" }}>{payload.error}</div>}
          <pre
            style={{
              ...payloadStyle,
              minHeight: 280,
              maxHeight: 520,
              overflow: "auto",
              margin: 0,
              whiteSpace: "pre-wrap",
            }}
          >
            {shortJson(payload?.detail ?? payload, 9000)}
          </pre>
        </div>
      )}
    </div>
  );
}

function ProductCard({ product, onDispatch, onOpenRun }) {
  const health = product.health || {};
  const actions = product.actions || [];
  const dispatchableActions = actions.filter((action) => !action.destructive && product.slug !== "hive-core");
  const lifecycle = product.hivecore || {};
  const [selectedActionId, setSelectedActionId] = useState(dispatchableActions[0]?.id || "");
  const selectedAction = useMemo(
    () => dispatchableActions.find((action) => action.id === selectedActionId) || dispatchableActions[0],
    [dispatchableActions, selectedActionId],
  );
  const [payloadText, setPayloadText] = useState(() => pretty(defaultActionRequest(product, selectedAction)));
  const [dispatching, setDispatching] = useState(false);
  const [localResult, setLocalResult] = useState(null);

  useEffect(() => {
    const nextAction = dispatchableActions.find((action) => action.id === selectedActionId) || dispatchableActions[0];
    if (!nextAction) return;
    setSelectedActionId(nextAction.id);
    setPayloadText(pretty(defaultActionRequest(product, nextAction)));
  }, [product.slug, actions.length]);

  function chooseAction(actionId) {
    const nextAction = dispatchableActions.find((action) => action.id === actionId);
    setSelectedActionId(actionId);
    setPayloadText(pretty(defaultActionRequest(product, nextAction)));
    setLocalResult(null);
  }

  async function dispatchSelected() {
    if (!selectedAction) return;
    setDispatching(true);
    setLocalResult(null);
    try {
      const request = parsePayload(payloadText);
      const result = await onDispatch(product, selectedAction, request);
      setLocalResult(result.event);
    } catch (err) {
      setLocalResult({ status: "failed", error: err.message || "Dispatch failed." });
    } finally {
      setDispatching(false);
    }
  }

  function resetPayload() {
    setPayloadText(pretty(defaultActionRequest(product, selectedAction)));
    setLocalResult(null);
  }

  function formatPayload() {
    try {
      setPayloadText(pretty(parsePayload(payloadText)));
      setLocalResult(null);
    } catch (err) {
      setLocalResult({ status: "failed", error: `Invalid JSON · ${err.message}` });
    }
  }

  const canDispatch =
    product.enabled &&
    product.api_url &&
    health.capabilities_ok &&
    product.api_key_configured &&
    selectedAction;

  return (
    <div
      style={{
        ...S.panel,
        display: "grid",
        gap: 12,
        borderColor: `${statusColor(product.status)}55`,
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start" }}>
        <div style={{ display: "grid", gap: 4 }}>
          <div style={{ fontSize: 16, fontWeight: 800 }}>
            {product.icon} {product.title}
          </div>
          <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.5 }}>{product.role}</div>
        </div>
        <Tag color={statusColor(product.status)}>{product.status}</Tag>
      </div>

      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        <Tag color="var(--accent)">{product.lane}</Tag>
        {health.version && <Tag color="var(--blue)">v{health.version}</Tag>}
        {health.capabilities_ok && <Tag color="var(--green)">{health.action_count} action{health.action_count === 1 ? "" : "s"}</Tag>}
        {!health.capabilities_ok && product.enabled && <Tag color="var(--gold)">contract pending</Tag>}
        {product.enabled &&
          (health.runs_ok ? (
            <Tag color="var(--blue)">{health.run_count} run{health.run_count === 1 ? "" : "s"}</Tag>
          ) : (
            <Tag color="var(--gold)">runs locked</Tag>
          ))}
        <Tag color={product.api_key_configured ? "var(--green)" : "var(--gold)"}>
          {product.api_key_configured ? "key linked" : "key missing"}
        </Tag>
        {health.startup_warns > 0 && <Tag color="var(--gold)">{health.startup_warns} warn</Tag>}
        {health.startup_errors > 0 && <Tag color="var(--accent)">{health.startup_errors} error</Tag>}
      </div>

      <div style={{ display: "grid", gap: 6, gridTemplateColumns: "repeat(auto-fit, minmax(150px, 1fr))" }}>
        <div>
          <div style={S.label}>Frontend</div>
          <div style={{ fontSize: 11, color: "var(--text-dim)", wordBreak: "break-all" }}>
            {product.frontend_url || "Not configured"}
          </div>
        </div>
        <div>
          <div style={S.label}>API</div>
          <div style={{ fontSize: 11, color: "var(--text-dim)", wordBreak: "break-all" }}>
            {product.api_url || "Not configured"}
          </div>
        </div>
        <div>
          <div style={S.label}>Lifecycle Contract</div>
          <div style={{ fontSize: 11, color: health.capabilities_ok ? "var(--green)" : "var(--gold)" }}>
            {health.capabilities_ok
              ? `${lifecycle.can_start_runs ? "start" : "view"} · ${lifecycle.can_list_runs ? "runs" : "no runs"}`
              : "Not available"}
          </div>
        </div>
        <div>
          <div style={S.label}>Database</div>
          <div style={{ fontSize: 11, color: health.db_ok === false ? "var(--accent)" : "var(--text-dim)" }}>
            {health.db_ok == null ? "Unknown" : health.db_ok ? "Healthy" : "Unhealthy"}
          </div>
        </div>
      </div>

      <ContractChecklist product={product} />

      <RunHistory product={product} onOpenRun={onOpenRun} />

      {dispatchableActions.length > 0 && (
        <div style={{ borderTop: "1px solid var(--border)", paddingTop: 12, display: "grid", gap: 10 }}>
          <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center" }}>
            <div>
              <div style={{ fontSize: 12, fontWeight: 800 }}>Command Deck</div>
              <div style={{ fontSize: 10, color: "var(--text-dim)" }}>Dispatch one advertised product action through HiveCore.</div>
            </div>
            <Tag color={selectedAction?.starts_run ? "var(--green)" : "var(--blue)"}>
              {selectedAction?.starts_run ? "starts run" : "utility"}
            </Tag>
          </div>

          <select
            value={selectedAction?.id || ""}
            onChange={(event) => chooseAction(event.target.value)}
            style={S.select}
          >
            {dispatchableActions.map((action) => (
              <option key={action.id} value={action.id}>
                {action.label}
              </option>
            ))}
          </select>

          {selectedAction && (
            <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.5 }}>
              <strong style={{ color: "var(--text)" }}>{selectedAction.method} {selectedAction.path}</strong>
              <br />
              {selectedAction.description}
            </div>
          )}

          <textarea
            value={payloadText}
            onChange={(event) => setPayloadText(event.target.value)}
            style={payloadStyle}
            spellCheck={false}
          />

          <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
            <Btn onClick={dispatchSelected} disabled={!canDispatch || dispatching} color="var(--green)">
              {dispatching ? "Dispatching..." : "Dispatch action"}
            </Btn>
            <Btn onClick={formatPayload}>Format JSON</Btn>
            <Btn onClick={resetPayload}>Reset</Btn>
            {!product.api_key_configured && <span style={{ fontSize: 11, color: "var(--gold)" }}>Save this product's API key in Settings first.</span>}
            {!health.capabilities_ok && <span style={{ fontSize: 11, color: "var(--gold)" }}>Contract check is not passing yet.</span>}
          </div>

          {localResult && <DispatchResult event={localResult} />}
        </div>
      )}

      <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
        {product.frontend_url && (
          <a href={product.frontend_url} target="_blank" rel="noreferrer" style={{ color: "var(--accent)", fontSize: 11 }}>
            Open app
          </a>
        )}
        {product.api_url && (
          <a href={`${product.api_url.replace(/\/$/, "")}/health`} target="_blank" rel="noreferrer" style={{ color: "var(--blue)", fontSize: 11 }}>
            Health
          </a>
        )}
        <a href={`https://github.com/${product.repo}`} target="_blank" rel="noreferrer" style={{ color: "var(--text-dim)", fontSize: 11 }}>
          {product.repo}
        </a>
      </div>

      {product.notes && (
        <div style={{ borderTop: "1px solid var(--border)", paddingTop: 10, fontSize: 11, color: "var(--text-dim)" }}>
          {product.notes}
        </div>
      )}

      {health.error && (
        <div
          style={{
            borderTop: "1px solid var(--border)",
            paddingTop: 10,
            fontSize: 11,
            color: "var(--gold)",
          }}
        >
          {health.error}
        </div>
      )}
    </div>
  );
}

export default function ProductsPanel({ fetchEnvelope, setRunning, setError }) {
  const [products, setProducts] = useState([]);
  const [recentActions, setRecentActions] = useState([]);
  const [runDetail, setRunDetail] = useState(null);
  const [runDetailLoading, setRunDetailLoading] = useState(false);

  async function refresh() {
    setRunning(true);
    setError("");
    try {
      const [productData, actionData] = await Promise.all([
        fetchEnvelope("/products"),
        fetchEnvelope("/actions/recent"),
      ]);
      setProducts(productData || []);
      setRecentActions(actionData || []);
    } catch (err) {
      setProducts([]);
      setRecentActions([]);
      setError(err.message || "HiveCore could not load product health.");
    } finally {
      setRunning(false);
    }
  }

  async function dispatchAction(product, action, request) {
    setRunning(true);
    setError("");
    try {
      const result = await fetchEnvelope(`/products/${product.slug}/actions/${action.id}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
      });
      if (result?.event) {
        setRecentActions((current) => [result.event, ...current].slice(0, 30));
      }
      return result;
    } catch (err) {
      setError(err.message || "HiveCore could not dispatch the product action.");
      throw err;
    } finally {
      setRunning(false);
    }
  }

  async function openRunDetail(product, run) {
    setRunning(true);
    setError("");
    setRunDetail({ product, run });
    setRunDetailLoading(true);
    try {
      const data = await fetchEnvelope(`/products/${product.slug}/runs/${encodeURIComponent(run.id)}`);
      setRunDetail({ product, run, data });
    } catch (err) {
      setRunDetail({ product, run, error: err.message || "HiveCore could not load run detail." });
      setError(err.message || "HiveCore could not load run detail.");
    } finally {
      setRunDetailLoading(false);
      setRunning(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div
        style={{
          ...S.panel,
          display: "grid",
          gap: 14,
          background:
            "radial-gradient(circle at 0% 0%, rgba(58,159,179,0.18), transparent 36%), linear-gradient(135deg, rgba(255,255,255,0.05), rgba(255,255,255,0.015))",
        }}
      >
        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
          <div>
            <div style={{ fontSize: 20, fontWeight: 900 }}>HiveCore Command Surface</div>
            <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
              Live contract polling, saved product keys, and dispatch controls for every enabled PatchHive product.
            </div>
          </div>
          <Btn onClick={refresh}>Refresh suite</Btn>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <Tag color="var(--green)">{products.filter((product) => product.status === "online").length} online</Tag>
          <Tag color="var(--gold)">{products.filter((product) => product.status === "degraded").length} degraded</Tag>
          <Tag color="var(--blue)">{products.reduce((total, product) => total + ((product.actions || []).length), 0)} advertised actions</Tag>
          <Tag color="var(--accent)">{products.reduce((total, product) => total + (product.health?.run_count || 0), 0)} visible runs</Tag>
          <Tag color="var(--accent)">{products.filter((product) => product.api_key_configured).length} keyed products</Tag>
        </div>
      </div>

      <RecentActivity events={recentActions} />

      <RunDetailPanel
        detail={runDetail}
        loading={runDetailLoading}
        onClose={() => setRunDetail(null)}
      />

      {products.length === 0 ? (
        <EmptyState icon="⬢" text="HiveCore does not have any product status to show yet." />
      ) : (
        <div style={{ display: "grid", gap: 12, gridTemplateColumns: "repeat(auto-fit, minmax(340px, 1fr))" }}>
          {products.map((product) => (
            <ProductCard
              key={product.slug}
              product={product}
              onDispatch={dispatchAction}
              onOpenRun={openRunDetail}
            />
          ))}
        </div>
      )}
    </div>
  );
}
