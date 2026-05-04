import { useEffect, useMemo, useState } from "react";
import { Btn, EmptyState, S, Tag } from "@patchhivehq/ui";

const statGridStyle = {
  display: "grid",
  gap: 12,
  gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))",
};

const statCardStyle = {
  ...S.panel,
  padding: 14,
  display: "grid",
  gap: 6,
};

const stepCardStyle = {
  ...S.panel,
  display: "grid",
  gap: 10,
};

function statusMeta(status) {
  switch (status) {
    case "complete":
      return { label: "complete", color: "var(--green)" };
    case "needs_action":
      return { label: "needs action", color: "var(--accent)" };
    case "optional":
      return { label: "optional", color: "var(--text-dim)" };
    default:
      return { label: "recommended", color: "var(--gold)" };
  }
}

function levelColor(level) {
  if (level === "error") return "var(--accent)";
  if (level === "warn") return "var(--gold)";
  if (level === "ok") return "var(--green)";
  return "var(--text-dim)";
}

function summarizeChecks(checks) {
  return checks.reduce(
    (counts, check) => {
      const level = String(check?.level || "info").toLowerCase();
      if (level === "error") counts.errors += 1;
      else if (level === "warn") counts.warns += 1;
      else if (level === "ok") counts.ok += 1;
      else counts.info += 1;
      return counts;
    },
    { errors: 0, warns: 0, ok: 0, info: 0 },
  );
}

async function fetchJson(fetch_, url, fallbackError) {
  const response = await fetch_(url);
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(data.error || `${fallbackError} (${response.status})`);
  }
  return data;
}

function resolveCustomStepStatus(step, context) {
  if (typeof step.status === "function") {
    return step.status(context);
  }
  return step.status || "recommended";
}

function healthRows(health) {
  if (!health || typeof health !== "object") return [];
  return [
    { label: "Status", value: health.status || "unknown" },
    { label: "Version", value: health.version || "unknown" },
    { label: "DB Path", value: health.db_path || "n/a" },
  ].filter((row) => row.value && row.value !== "n/a" ? true : row.label !== "DB Path");
}

export default function ProductSetupWizard({
  apiBase,
  fetch_,
  product,
  icon,
  description,
  steps = [],
  onOpenTab,
  checksTabId = "checks",
}) {
  const [health, setHealth] = useState(null);
  const [checks, setChecks] = useState([]);
  const [authStatus, setAuthStatus] = useState(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState("");

  async function refresh() {
    setLoading(true);
    setLoadError("");

    const [healthResult, checksResult, authResult] = await Promise.allSettled([
      fetchJson(fetch_, `${apiBase}/health`, "Could not load /health"),
      fetchJson(fetch_, `${apiBase}/startup/checks`, "Could not load /startup/checks"),
      fetchJson(fetch_, `${apiBase}/auth/status`, "Could not load /auth/status"),
    ]);

    const failures = [];

    if (healthResult.status === "fulfilled") {
      setHealth(healthResult.value);
    } else {
      setHealth(null);
      failures.push(healthResult.reason?.message || "Could not load /health");
    }

    if (checksResult.status === "fulfilled") {
      setChecks(Array.isArray(checksResult.value?.checks) ? checksResult.value.checks : []);
    } else {
      setChecks([]);
      failures.push(checksResult.reason?.message || "Could not load /startup/checks");
    }

    if (authResult.status === "fulfilled") {
      setAuthStatus(authResult.value);
    } else {
      setAuthStatus(null);
      failures.push(authResult.reason?.message || "Could not load /auth/status");
    }

    setLoadError(failures.join(" "));
    setLoading(false);
  }

  useEffect(() => {
    let cancelled = false;

    const run = async () => {
      await refresh();
    };

    if (!cancelled) {
      run();
    }

    return () => {
      cancelled = true;
    };
  }, [apiBase, fetch_]);

  const counts = useMemo(() => summarizeChecks(checks), [checks]);
  const overallStatus = loadError
    ? "needs_action"
    : counts.errors > 0
      ? "needs_action"
      : counts.warns > 0
        ? "recommended"
        : "complete";

  const overallMeta = statusMeta(overallStatus);
  const serviceStatus = authStatus?.service_auth_supported
    ? authStatus?.service_auth_expired
      ? "needs_action"
      : authStatus?.service_auth_configured
        ? authStatus?.service_auth_legacy
          ? "recommended"
          : "complete"
        : "optional"
    : "optional";

  const setupContext = useMemo(
    () => ({
      health,
      checks,
      counts,
      authStatus,
    }),
    [health, checks, counts, authStatus],
  );

  const wizardSteps = useMemo(() => {
    const base = [
      {
        title: "Verify operator access",
        detail: authStatus?.auth_configured
          ? "This browser session already has a working operator API key for this product."
          : "Generate the first local operator key or sign in with an existing key before continuing.",
        status: authStatus?.auth_configured ? "complete" : "needs_action",
      },
      {
        title: "Clear backend startup checks",
        detail:
          counts.errors > 0
            ? `${counts.errors} blocking startup check${counts.errors === 1 ? "" : "s"} still need attention before this product is truly ready.`
            : counts.warns > 0
              ? `${counts.warns} warning${counts.warns === 1 ? "" : "s"} remain. The product can usually run, but the setup deserves a quick pass.`
              : "The shared startup checks are clean right now.",
        status: counts.errors > 0 ? "needs_action" : counts.warns > 0 ? "recommended" : "complete",
        tab: checksTabId,
        actionLabel: "Open Checks",
      },
      {
        title: "Pair service auth for HiveCore later",
        detail: !authStatus?.service_auth_supported
          ? "This product does not advertise service-token auth."
          : authStatus?.service_auth_expired
            ? "The saved machine token is expired. Rotate it before HiveCore tries to dispatch through this product."
            : authStatus?.service_auth_configured
              ? authStatus?.service_auth_legacy
                ? "A legacy machine token exists. It can still read runs, but rotate it to a scoped token before relying on HiveCore dispatch."
                : "Scoped service auth is configured, so HiveCore can pair with this product cleanly."
              : "Machine auth is not configured yet. That is fine for standalone use, and HiveCore can provision it later.",
        status: serviceStatus,
      },
    ];

    return base.concat(
      steps.map((step) => ({
        ...step,
        status: resolveCustomStepStatus(step, setupContext),
      })),
    );
  }, [authStatus, checksTabId, counts.errors, counts.warns, serviceStatus, setupContext, steps]);

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div style={{ ...S.panel, display: "grid", gap: 12 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "flex-start" }}>
          <div style={{ display: "grid", gap: 6, maxWidth: 860 }}>
            <div style={{ fontSize: 20, fontWeight: 700, color: "var(--accent)" }}>
              {icon} {product} Setup Wizard
            </div>
            <div style={{ color: "var(--text-dim)", fontSize: 13, lineHeight: 1.6 }}>
              {description}
            </div>
          </div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            {onOpenTab && (
              <Btn onClick={() => onOpenTab(checksTabId)}>
                Open Checks
              </Btn>
            )}
            <Btn onClick={refresh} disabled={loading}>
              {loading ? "Refreshing..." : "Refresh"}
            </Btn>
          </div>
        </div>

        <div style={statGridStyle}>
          <div style={statCardStyle}>
            <div style={S.label}>Overall</div>
            <div style={{ fontSize: 18, fontWeight: 700, color: overallMeta.color }}>{overallMeta.label}</div>
            <div style={{ color: "var(--text-muted)", fontSize: 11 }}>
              {counts.errors > 0
                ? "Blocking checks still need work."
                : counts.warns > 0
                  ? "Safe to review, but not fully clean."
                  : "Shared setup signals look healthy."}
            </div>
          </div>
          <div style={statCardStyle}>
            <div style={S.label}>Operator Auth</div>
            <div style={{ fontSize: 18, fontWeight: 700, color: authStatus?.auth_configured ? "var(--green)" : "var(--accent)" }}>
              {authStatus?.auth_configured ? "configured" : "missing"}
            </div>
            <div style={{ color: "var(--text-muted)", fontSize: 11 }}>
              {authStatus?.bootstrap_required ? "First-time local bootstrap still required." : "Operator access is ready for this browser session."}
            </div>
          </div>
          <div style={statCardStyle}>
            <div style={S.label}>Service Auth</div>
            <div style={{ fontSize: 18, fontWeight: 700, color: statusMeta(serviceStatus).color }}>
              {statusMeta(serviceStatus).label}
            </div>
            <div style={{ color: "var(--text-muted)", fontSize: 11 }}>
              {authStatus?.service_auth_supported
                ? authStatus?.service_auth_configured
                  ? authStatus?.service_auth_legacy
                    ? "Legacy machine token only."
                    : "Scoped token ready for HiveCore."
                  : "Not configured yet."
                : "Not advertised by this product."}
            </div>
          </div>
          <div style={statCardStyle}>
            <div style={S.label}>Startup Checks</div>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <Tag color="var(--accent)">{counts.errors} error{counts.errors === 1 ? "" : "s"}</Tag>
              <Tag color="var(--gold)">{counts.warns} warning{counts.warns === 1 ? "" : "s"}</Tag>
              <Tag color="var(--green)">{counts.ok} ok</Tag>
            </div>
            <div style={{ color: "var(--text-muted)", fontSize: 11 }}>
              Live data from the shared backend startup contract.
            </div>
          </div>
        </div>

        {loadError && (
          <div style={{ border: "1px solid var(--accent)44", background: "var(--accent)10", color: "var(--accent)", borderRadius: 8, padding: "12px 14px", fontSize: 12 }}>
            {loadError}
          </div>
        )}
      </div>

      <div style={{ display: "grid", gap: 12 }}>
        {wizardSteps.map((step, index) => {
          const meta = statusMeta(step.status);
          return (
            <div key={`${step.title}-${index}`} style={stepCardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start", flexWrap: "wrap" }}>
                <div style={{ display: "grid", gap: 4, maxWidth: 820 }}>
                  <div style={{ fontSize: 11, color: "var(--text-muted)", letterSpacing: "0.08em", textTransform: "uppercase" }}>
                    Step {index + 1}
                  </div>
                  <div style={{ fontSize: 16, fontWeight: 700 }}>{step.title}</div>
                  <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{step.detail}</div>
                </div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
                  <Tag color={meta.color}>{meta.label}</Tag>
                  {step.tab && onOpenTab && (
                    <Btn onClick={() => onOpenTab(step.tab)}>
                      {step.actionLabel || "Open"}
                    </Btn>
                  )}
                </div>
              </div>
            </div>
          );
        })}
      </div>

      <div style={{ display: "grid", gap: 16, gridTemplateColumns: "repeat(auto-fit, minmax(320px, 1fr))" }}>
        <div style={{ ...S.panel, display: "grid", gap: 12 }}>
          <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
            <div>
              <div style={{ fontSize: 16, fontWeight: 700 }}>Live Startup Checks</div>
              <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
                The same backend checks the dedicated Checks tab reads.
              </div>
            </div>
            {onOpenTab && (
              <Btn onClick={() => onOpenTab(checksTabId)}>
                Open Full Checks
              </Btn>
            )}
          </div>

          {checks.length === 0 ? (
            <EmptyState icon="◌" text="No startup checks were returned." />
          ) : (
            checks.map((check, index) => (
              <div key={`${check.msg}-${index}`} style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 12, display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start" }}>
                <div style={{ fontSize: 12, lineHeight: 1.6 }}>{check.msg}</div>
                <Tag color={levelColor(String(check.level || "info").toLowerCase())}>
                  {String(check.level || "info").toLowerCase()}
                </Tag>
              </div>
            ))
          )}
        </div>

        <div style={{ ...S.panel, display: "grid", gap: 12 }}>
          <div>
            <div style={{ fontSize: 16, fontWeight: 700 }}>Health Snapshot</div>
            <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
              Quick backend facts from `/health` and `/auth/status`.
            </div>
          </div>

          <div style={{ display: "grid", gap: 10 }}>
            {healthRows(health).map((row) => (
              <div key={row.label} style={{ display: "grid", gap: 4 }}>
                <div style={S.label}>{row.label}</div>
                <div style={{ fontSize: row.label === "DB Path" ? 12 : 16, fontWeight: 700, color: row.label === "Status" && row.value !== "ok" ? "var(--accent)" : "var(--text)" }}>
                  {row.value}
                </div>
              </div>
            ))}
            {authStatus?.service_auth_token?.fingerprint && (
              <div style={{ display: "grid", gap: 4 }}>
                <div style={S.label}>Service Fingerprint</div>
                <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
                  {authStatus.service_auth_token.fingerprint}
                </div>
              </div>
            )}
            {Array.isArray(authStatus?.service_auth_scopes) && authStatus.service_auth_scopes.length > 0 && (
              <div style={{ display: "grid", gap: 6 }}>
                <div style={S.label}>Service Scopes</div>
                <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                  {authStatus.service_auth_scopes.map((scope) => (
                    <Tag key={scope} color="var(--green)">{scope}</Tag>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
