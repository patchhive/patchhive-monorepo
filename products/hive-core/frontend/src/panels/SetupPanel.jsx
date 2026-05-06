import { useEffect, useState } from "react";
import { Btn, EmptyState, Input, S, Tag } from "@patchhivehq/ui";

function statusColor(status) {
  if (status === "online") return "var(--green)";
  if (status === "degraded") return "var(--gold)";
  if (status === "disabled") return "var(--text-dim)";
  if (status === "unconfigured") return "var(--blue)";
  return "var(--accent)";
}

function authTone(item) {
  if (item.runtime.slug === "hive-core") return "var(--blue)";
  if (item.runtime.service_token_configured) return "var(--green)";
  if (item.runtime.legacy_api_key_configured) return "var(--gold)";
  return "var(--accent)";
}

function authLabel(item) {
  if (item.runtime.slug === "hive-core") return "native control plane";
  if (item.runtime.service_token_configured) return "service token saved";
  if (item.runtime.legacy_api_key_configured) return "legacy key saved";
  return "service token missing";
}

function launcherTone(status) {
  if (status === "running") return "var(--green)";
  if (status === "blocked") return "var(--accent)";
  return "var(--gold)";
}

function smokeTone(status) {
  if (status === "ready" || status === "pass") return "var(--green)";
  if (status === "attention" || status === "warn" || status === "skip") return "var(--gold)";
  if (status === "blocked" || status === "fail") return "var(--accent)";
  return "var(--blue)";
}

function credentialTone(status) {
  if (status === "ready") return "var(--green)";
  if (status === "optional") return "var(--blue)";
  if (status === "placeholder") return "var(--gold)";
  return "var(--accent)";
}

function inputTypeForCredential(requirement) {
  if (requirement.redact || requirement.kind === "github_token") return "password";
  if (requirement.kind === "email") return "email";
  return "text";
}

function LaunchGauge({ label, value, total, tone = "var(--accent)" }) {
  const pct = total > 0 ? Math.round((value / total) * 100) : 0;
  return (
    <div
      style={{
        display: "grid",
        gap: 7,
        padding: "11px 12px",
        border: "1px solid var(--border)",
        borderRadius: 12,
        background: "rgba(0,0,0,0.2)",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", gap: 8 }}>
        <span style={{ fontSize: 10, color: "var(--text-dim)", letterSpacing: "0.14em", textTransform: "uppercase" }}>{label}</span>
        <strong style={{ fontSize: 12, color: tone }}>
          {value}/{total}
        </strong>
      </div>
      <div style={{ height: 7, borderRadius: 999, background: "rgba(255,255,255,0.06)", overflow: "hidden" }}>
        <div style={{ width: `${pct}%`, height: "100%", borderRadius: 999, background: tone }} />
      </div>
    </div>
  );
}

function SmokeEvidence({ smoke }) {
  if (!smoke) return null;
  const grouped = smoke.steps.reduce((acc, step) => {
    acc[step.slug] = acc[step.slug] || [];
    acc[step.slug].push(step);
    return acc;
  }, {});

  return (
    <div style={{ ...S.panel, display: "grid", gap: 12, borderColor: `${smokeTone(smoke.status)}66` }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 10, flexWrap: "wrap", alignItems: "center" }}>
        <div>
          <div style={{ fontSize: 11, letterSpacing: "0.16em", textTransform: "uppercase", color: "var(--text-dim)" }}>
            Latest suite smoke
          </div>
          <div style={{ fontSize: 18, fontWeight: 900 }}>{smoke.summary}</div>
          <div style={{ fontSize: 11, color: "var(--text-dim)" }}>
            {smoke.finished_at} · {smoke.id}
          </div>
        </div>
        <Tag color={smokeTone(smoke.status)}>{smoke.status}</Tag>
      </div>

      <div style={{ display: "grid", gap: 10, gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))" }}>
        {Object.entries(grouped).map(([slug, steps]) => (
          <div
            key={slug}
            style={{
              display: "grid",
              gap: 7,
              padding: 12,
              border: "1px solid var(--border)",
              borderRadius: 12,
              background: "rgba(0,0,0,0.18)",
            }}
          >
            <div style={{ fontSize: 13, fontWeight: 800 }}>{steps[0]?.title || slug}</div>
            {steps.map((step) => (
              <div
                key={`${step.slug}-${step.check}`}
                style={{
                  display: "grid",
                  gap: 4,
                  padding: "8px 9px",
                  borderRadius: 9,
                  background: "color-mix(in srgb, var(--bg) 55%, transparent)",
                  border: `1px solid ${smokeTone(step.status)}44`,
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center" }}>
                  <span style={{ fontSize: 11, fontWeight: 800 }}>{step.check}</span>
                  <Tag color={smokeTone(step.status)}>{step.status}</Tag>
                </div>
                <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.45 }}>
                  {step.message}
                  {step.remote_status ? ` Remote HTTP ${step.remote_status}.` : ""}
                </div>
              </div>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

function ProductCard({
  item,
  busyAction,
  credentialDraft = {},
  credentialResult,
  onCredentialChange,
  onSaveCredentials,
  onValidateGithubToken,
  onProductAction,
  onLoadLogs,
}) {
  const authStatus = item.auth_status;
  const launcher = item.launcher;
  const credentials = item.credentials || [];
  const actionPrefix = `${item.runtime.slug}:`;
  const busy = Boolean(busyAction);
  const busyForProduct = busyAction.startsWith(actionPrefix);
  const hasCredentialDraft = Object.values(credentialDraft).some((value) => value?.trim?.());
  const githubTokenDraft = credentialDraft.BOT_GITHUB_TOKEN || "";
  return (
    <div
      style={{
        ...S.panel,
        display: "grid",
        gap: 10,
        borderColor: `${statusColor(item.runtime.status)}55`,
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", gap: 10, alignItems: "flex-start" }}>
        <div style={{ display: "grid", gap: 4 }}>
          <div style={{ fontSize: 15, fontWeight: 800 }}>
            {item.runtime.icon} {item.runtime.title}
          </div>
          <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.5 }}>{item.runtime.role}</div>
        </div>
        <Tag color={statusColor(item.runtime.status)}>{item.runtime.status}</Tag>
      </div>

      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        <Tag color="var(--accent)">{item.runtime.lane}</Tag>
        <Tag color={authTone(item)}>{authLabel(item)}</Tag>
        {authStatus?.suite_bootstrap_enabled && <Tag color="var(--blue)">suite bootstrap ready</Tag>}
        {item.pairing_ready && <Tag color="var(--green)">ready to pair</Tag>}
        {launcher && <Tag color={launcherTone(launcher.status)}>launcher {launcher.status}</Tag>}
      </div>

      <div style={{ display: "grid", gap: 6, fontSize: 11 }}>
        <div style={{ color: "var(--text-dim)" }}>
          API: <span style={{ color: "var(--text)" }}>{item.runtime.api_url}</span>
        </div>
        <div style={{ color: "var(--text-dim)" }}>
          Frontend: <span style={{ color: "var(--text)" }}>{item.runtime.frontend_url}</span>
        </div>
        {item.runtime.health.startup_errors > 0 && (
          <div style={{ color: "var(--accent)" }}>
            {item.runtime.health.startup_errors} startup error{item.runtime.health.startup_errors === 1 ? "" : "s"}
          </div>
        )}
        {item.runtime.health.startup_warns > 0 && (
          <div style={{ color: "var(--gold)" }}>
            {item.runtime.health.startup_warns} startup warn{item.runtime.health.startup_warns === 1 ? "" : "s"}
          </div>
        )}
        {item.auth_status_error && <div style={{ color: "var(--gold)" }}>{item.auth_status_error}</div>}
      </div>

      {launcher && (
        <div
          style={{
            display: "grid",
            gap: 7,
            padding: "10px",
            border: "1px solid var(--border)",
            borderRadius: 10,
            background: "color-mix(in srgb, var(--bg) 42%, transparent)",
            fontSize: 11,
          }}
        >
          <div style={{ display: "flex", gap: 7, flexWrap: "wrap" }}>
            <Tag color={launcher.compose_exists ? "var(--green)" : "var(--accent)"}>
              compose {launcher.compose_exists ? "found" : "missing"}
            </Tag>
            <Tag color={launcher.env_exists ? "var(--green)" : "var(--gold)"}>
              env {launcher.env_exists ? "ready" : "from example"}
            </Tag>
            <Tag color={launcher.suite_bootstrap_configured ? "var(--green)" : "var(--gold)"}>
              bootstrap {launcher.suite_bootstrap_configured ? "synced" : "missing"}
            </Tag>
            <Tag color={launcher.compose_running ? "var(--green)" : "var(--gold)"}>
              compose {launcher.compose_running ? "running" : "not running"}
            </Tag>
          </div>
          <div style={{ display: "grid", gap: 4, color: "var(--text-dim)" }}>
            <div>
              API port {launcher.api_port}:{" "}
              <span style={{ color: launcher.api_port_open ? "var(--green)" : "var(--gold)" }}>
                {launcher.api_port_open ? "open" : "closed"}
              </span>
            </div>
            <div>
              UI port {launcher.frontend_port}:{" "}
              <span style={{ color: launcher.frontend_port_open ? "var(--green)" : "var(--gold)" }}>
                {launcher.frontend_port_open ? "open" : "closed"}
              </span>
            </div>
          </div>
          {launcher.blockers?.length > 0 && (
            <div style={{ display: "grid", gap: 4, color: "var(--accent)" }}>
              {launcher.blockers.map((blocker) => (
                <div key={blocker}>{blocker}</div>
              ))}
            </div>
          )}
        </div>
      )}

      {credentials.length > 0 && (
        <div
          style={{
            display: "grid",
            gap: 9,
            padding: 10,
            border: "1px solid var(--border)",
            borderRadius: 10,
            background: "color-mix(in srgb, var(--bg) 50%, transparent)",
          }}
        >
          <div style={{ display: "flex", justifyContent: "space-between", gap: 10, flexWrap: "wrap", alignItems: "center" }}>
            <div>
              <div style={{ fontSize: 12, fontWeight: 900 }}>First-run credentials</div>
              <div style={{ fontSize: 10, color: "var(--text-dim)", lineHeight: 1.45 }}>
                Saved through HiveCore and written locally by patchhive-launcher.
              </div>
            </div>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {credentials.map((requirement) => (
                <Tag key={requirement.key} color={credentialTone(requirement.status)}>
                  {requirement.key} {requirement.status}
                </Tag>
              ))}
            </div>
          </div>

          <div style={{ display: "grid", gap: 8 }}>
            {credentials.map((requirement) => (
              <label key={requirement.key} style={{ ...S.field }}>
                <span style={S.label}>
                  {requirement.label} {requirement.required ? "" : "(optional)"}
                </span>
                <Input
                  type={inputTypeForCredential(requirement)}
                  value={credentialDraft[requirement.key] || ""}
                  onChange={(value) => onCredentialChange(item.runtime.slug, requirement.key, value)}
                  placeholder={requirement.configured ? "Configured - leave blank to keep current value" : requirement.key}
                />
                <span style={{ fontSize: 10, color: "var(--text-dim)", lineHeight: 1.45 }}>{requirement.description}</span>
              </label>
            ))}
          </div>

          {credentialResult && (
            <div
              style={{
                fontSize: 11,
                lineHeight: 1.5,
                color: credentialResult.ok && credentialResult.user_matches ? "var(--green)" : "var(--gold)",
              }}
            >
              {credentialResult.message}
            </div>
          )}

          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            {credentials.some((requirement) => requirement.kind === "github_token") && (
              <Btn
                onClick={() => onValidateGithubToken(item.runtime.slug)}
                disabled={busy || !githubTokenDraft.trim()}
                color="var(--blue)"
              >
                {busyAction === `${actionPrefix}validate-token` ? "Validating..." : "Validate GitHub token"}
              </Btn>
            )}
            <Btn
              onClick={() => onSaveCredentials(item.runtime.slug)}
              disabled={busy || !hasCredentialDraft}
              color="var(--green)"
            >
              {busyAction === `${actionPrefix}save-env` ? "Saving..." : "Save + recreate"}
            </Btn>
          </div>
        </div>
      )}

      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        {item.runtime.frontend_url && (
          <a href={item.runtime.frontend_url} target="_blank" rel="noreferrer" style={{ color: "var(--accent)", fontSize: 11 }}>
            Open app
          </a>
        )}
        {item.runtime.api_url && (
          <a href={item.runtime.api_url} target="_blank" rel="noreferrer" style={{ color: "var(--blue)", fontSize: 11 }}>
            Open API
          </a>
        )}
      </div>

      {item.runtime.slug !== "hive-core" && (
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <Btn onClick={() => onProductAction(item.runtime.slug, "start")} disabled={busy} color="var(--green)">
            {busyAction === `${actionPrefix}start` ? "Starting..." : "Start"}
          </Btn>
          <Btn onClick={() => onProductAction(item.runtime.slug, "restart")} disabled={busy} color="var(--blue)">
            {busyAction === `${actionPrefix}restart` ? "Restarting..." : "Restart"}
          </Btn>
          <Btn onClick={() => onProductAction(item.runtime.slug, "stop")} disabled={busy}>
            {busyAction === `${actionPrefix}stop` ? "Stopping..." : "Stop"}
          </Btn>
          <Btn onClick={() => onLoadLogs(item.runtime.slug)} disabled={busy}>
            {busyAction === `${actionPrefix}logs` ? "Loading logs..." : "Logs"}
          </Btn>
        </div>
      )}
    </div>
  );
}

export default function SetupPanel({ fetchEnvelope, setRunning, setError }) {
  const [setup, setSetup] = useState(null);
  const [busyAction, setBusyAction] = useState("");
  const [logs, setLogs] = useState(null);
  const [credentialDrafts, setCredentialDrafts] = useState({});
  const [credentialResults, setCredentialResults] = useState({});

  async function refresh() {
    setRunning(true);
    setError("");
    try {
      const data = await fetchEnvelope("/setup/first-stack");
      setSetup(data);
    } catch (err) {
      setSetup(null);
      setError(err.message || "HiveCore could not load the first-stack setup status.");
    } finally {
      setRunning(false);
    }
  }

  async function runAction(label, path) {
    setBusyAction(label);
    setRunning(true);
    setError("");
    try {
      const data = await fetchEnvelope(path, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({}),
      });
      setSetup(data);
    } catch (err) {
      setError(err.message || `HiveCore could not ${label.toLowerCase()}.`);
    } finally {
      setBusyAction("");
      setRunning(false);
    }
  }

  async function runProductAction(slug, action) {
    setBusyAction(`${slug}:${action}`);
    setRunning(true);
    setError("");
    try {
      const data = await fetchEnvelope(`/setup/products/${slug}/${action}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({}),
      });
      setSetup(data);
    } catch (err) {
      setError(err.message || `HiveCore could not ${action} ${slug}.`);
    } finally {
      setBusyAction("");
      setRunning(false);
    }
  }

  async function loadLogs(slug) {
    setBusyAction(`${slug}:logs`);
    setRunning(true);
    setError("");
    try {
      const data = await fetchEnvelope(`/setup/products/${slug}/logs?tail=160`);
      setLogs(data);
    } catch (err) {
      setError(err.message || `HiveCore could not load logs for ${slug}.`);
    } finally {
      setBusyAction("");
      setRunning(false);
    }
  }

  function setCredentialDraft(slug, key, value) {
    setCredentialDrafts((current) => ({
      ...current,
      [slug]: {
        ...(current[slug] || {}),
        [key]: value,
      },
    }));
    setCredentialResults((current) => ({
      ...current,
      [slug]: null,
    }));
  }

  function nonEmptyCredentialValues(slug) {
    return Object.fromEntries(
      Object.entries(credentialDrafts[slug] || {}).filter(([, value]) => value?.trim?.()),
    );
  }

  async function validateGithubToken(slug) {
    const values = credentialDrafts[slug] || {};
    if (!values.BOT_GITHUB_TOKEN?.trim()) return;
    setBusyAction(`${slug}:validate-token`);
    setRunning(true);
    setError("");
    try {
      const data = await fetchEnvelope("/setup/credentials/github/validate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          token: values.BOT_GITHUB_TOKEN,
          expected_user: values.BOT_GITHUB_USER || "",
        }),
      });
      setCredentialResults((current) => ({ ...current, [slug]: data }));
    } catch (err) {
      setError(err.message || "HiveCore could not validate the GitHub token.");
    } finally {
      setBusyAction("");
      setRunning(false);
    }
  }

  async function saveCredentials(slug) {
    const values = nonEmptyCredentialValues(slug);
    if (Object.keys(values).length === 0) return;
    setBusyAction(`${slug}:save-env`);
    setRunning(true);
    setError("");
    try {
      const data = await fetchEnvelope(`/setup/products/${slug}/env`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ values, restart: true }),
      });
      setSetup(data);
      setCredentialDrafts((current) => ({ ...current, [slug]: {} }));
      setCredentialResults((current) => ({
        ...current,
        [slug]: { ok: true, user_matches: true, message: "Credentials saved and product recreated." },
      }));
    } catch (err) {
      setError(err.message || `HiveCore could not save credentials for ${slug}.`);
    } finally {
      setBusyAction("");
      setRunning(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  if (!setup) {
    return (
      <div style={{ display: "grid", gap: 16 }}>
        <div style={{ ...S.panel, display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
          <div>
            <div style={{ fontSize: 18, fontWeight: 800 }}>Setup</div>
            <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
              Detect the first stack, start missing products, and pair HiveCore automatically.
            </div>
          </div>
          <Btn onClick={refresh}>Refresh</Btn>
        </div>
        <EmptyState icon="⬢" text="HiveCore has not loaded the first-stack setup status yet." />
      </div>
    );
  }

  const downstream = setup.products.filter((item) => item.runtime.slug !== "hive-core");
  const onlineCount = downstream.filter((item) => ["online", "degraded"].includes(item.runtime.status)).length;
  const pairedCount = downstream.filter((item) => item.runtime.service_token_configured).length;

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div
        style={{
          ...S.panel,
          position: "relative",
          overflow: "hidden",
          display: "grid",
          gap: 16,
          background:
            "linear-gradient(135deg, color-mix(in srgb, var(--accent) 22%, var(--bg-panel)) 0%, var(--bg-panel) 45%, color-mix(in srgb, var(--blue) 20%, var(--bg-panel)) 100%)",
        }}
      >
        <div
          aria-hidden="true"
          style={{
            position: "absolute",
            inset: 0,
            background:
              "linear-gradient(90deg, rgba(255,255,255,0.025) 1px, transparent 1px), linear-gradient(0deg, rgba(255,255,255,0.018) 1px, transparent 1px)",
            backgroundSize: "34px 34px",
            opacity: 0.8,
            pointerEvents: "none",
          }}
        />
        <div style={{ position: "relative", display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center" }}>
          <div>
            <div style={{ fontSize: 11, letterSpacing: "0.18em", color: "var(--accent)", textTransform: "uppercase" }}>
              Launch preset / first stack
            </div>
            <div style={{ fontSize: 30, fontWeight: 950, letterSpacing: "-0.05em" }}>Bring the first squad online.</div>
            <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
              HiveCore controls the full 11-product fleet; this preset starts the first operational trio and pairs them for orchestration.
            </div>
          </div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <Btn onClick={refresh} disabled={Boolean(busyAction)}>
              Refresh
            </Btn>
            <Btn
              onClick={() => runAction("Start first stack", "/setup/first-stack/start")}
              disabled={busyAction === "Start first stack"}
              color="var(--green)"
            >
              {busyAction === "Start first stack" ? "Starting..." : "Start first stack"}
            </Btn>
            <Btn
              onClick={() => runAction("Pair running products", "/setup/first-stack/pair")}
              disabled={busyAction === "Pair running products"}
              color="var(--blue)"
            >
              {busyAction === "Pair running products" ? "Pairing..." : "Pair running products"}
            </Btn>
            <Btn
              onClick={() => runAction("Run smoke check", "/setup/first-stack/smoke")}
              disabled={busyAction === "Run smoke check"}
              color="var(--accent)"
            >
              {busyAction === "Run smoke check" ? "Checking..." : "Run smoke check"}
            </Btn>
            <Btn
              onClick={() => runAction("Stop first stack", "/setup/first-stack/stop")}
              disabled={busyAction === "Stop first stack"}
            >
              {busyAction === "Stop first stack" ? "Stopping..." : "Stop first stack"}
            </Btn>
          </div>
        </div>

        <div style={{ position: "relative", display: "flex", gap: 8, flexWrap: "wrap" }}>
          <Tag color={setup.launcher.available ? "var(--green)" : "var(--accent)"}>
            launcher {setup.launcher.available ? "available" : "unavailable"}
          </Tag>
          <Tag color={setup.launcher.docker_compose_available ? "var(--green)" : "var(--gold)"}>
            docker compose {setup.launcher.docker_compose_available ? "ready" : "missing"}
          </Tag>
          <Tag color={setup.launcher.docker_available ? "var(--green)" : "var(--gold)"}>
            docker {setup.launcher.docker_available ? "reachable" : "offline"}
          </Tag>
          <Tag color={setup.suite_bootstrap_configured ? "var(--green)" : "var(--gold)"}>
            suite bootstrap {setup.suite_bootstrap_configured ? "configured" : "not configured"}
          </Tag>
          <Tag color="var(--blue)">{onlineCount}/3 downstream reachable</Tag>
          <Tag color={pairedCount === 3 ? "var(--green)" : "var(--gold)"}>{pairedCount}/3 paired</Tag>
          {setup.latest_smoke && <Tag color={smokeTone(setup.latest_smoke.status)}>smoke {setup.latest_smoke.status}</Tag>}
        </div>

        <div style={{ position: "relative", display: "grid", gap: 10, gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))" }}>
          <LaunchGauge label="Downstream reachable" value={onlineCount} total={3} tone={onlineCount === 3 ? "var(--green)" : "var(--gold)"} />
          <LaunchGauge label="Machine pairing" value={pairedCount} total={3} tone={pairedCount === 3 ? "var(--green)" : "var(--blue)"} />
          <LaunchGauge label="Fleet catalog" value={setup.products.length} total={11} tone="var(--accent)" />
        </div>

        <div style={{ position: "relative", fontSize: 12, color: "var(--text-dim)", lineHeight: 1.6 }}>
          {setup.launcher.message}
          {setup.launcher.repo_root ? ` Repo root: ${setup.launcher.repo_root}` : ""}
        </div>
      </div>

      <SmokeEvidence smoke={setup.latest_smoke} />

      {setup.actions?.length > 0 && (
        <div style={{ ...S.panel, display: "grid", gap: 8 }}>
          <div style={{ fontSize: 15, fontWeight: 800 }}>Latest Setup Actions</div>
          <div style={{ display: "grid", gap: 7 }}>
            {setup.actions.map((action) => (
              <div
                key={action}
                style={{
                  padding: "9px 10px",
                  border: "1px solid var(--border)",
                  borderRadius: 8,
                  fontSize: 11,
                  lineHeight: 1.5,
                }}
              >
                {action}
              </div>
            ))}
          </div>
        </div>
      )}

      {logs && (
        <div style={{ ...S.panel, display: "grid", gap: 10 }}>
          <div style={{ display: "flex", justifyContent: "space-between", gap: 10, alignItems: "center" }}>
            <div>
              <div style={{ fontSize: 15, fontWeight: 800 }}>{logs.title} Recent Logs</div>
              <div style={{ fontSize: 11, color: "var(--text-dim)" }}>Read through HiveCore from patchhive-launcher.</div>
            </div>
            <Btn onClick={() => setLogs(null)}>Close</Btn>
          </div>
          <pre
            style={{
              margin: 0,
              maxHeight: 360,
              overflow: "auto",
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
              fontSize: 11,
              lineHeight: 1.45,
              padding: 12,
              borderRadius: 10,
              border: "1px solid var(--border)",
              background: "var(--bg)",
            }}
          >
            {logs.logs || "No recent logs returned."}
          </pre>
        </div>
      )}

      <div style={{ display: "grid", gap: 12, gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))" }}>
        {setup.products.map((item) => (
          <ProductCard
            key={item.runtime.slug}
            item={item}
            busyAction={busyAction}
            credentialDraft={credentialDrafts[item.runtime.slug] || {}}
            credentialResult={credentialResults[item.runtime.slug]}
            onCredentialChange={setCredentialDraft}
            onSaveCredentials={saveCredentials}
            onValidateGithubToken={validateGithubToken}
            onProductAction={runProductAction}
            onLoadLogs={loadLogs}
          />
        ))}
      </div>
    </div>
  );
}
