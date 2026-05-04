import { useEffect, useState } from "react";
import { Btn, EmptyState, S, Tag } from "@patchhivehq/ui";

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

function ProductCard({ item, busyAction, onProductAction, onLoadLogs }) {
  const authStatus = item.auth_status;
  const launcher = item.launcher;
  const actionPrefix = `${item.runtime.slug}:`;
  const busy = Boolean(busyAction);
  const busyForProduct = busyAction.startsWith(actionPrefix);
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
          display: "grid",
          gap: 12,
          background:
            "linear-gradient(135deg, color-mix(in srgb, var(--accent) 18%, var(--bg-panel)) 0%, var(--bg-panel) 52%, color-mix(in srgb, var(--blue) 16%, var(--bg-panel)) 100%)",
        }}
      >
        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center" }}>
          <div>
            <div style={{ fontSize: 22, fontWeight: 900, letterSpacing: "-0.03em" }}>First-Stack Setup</div>
            <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
              HiveCore should adapt to already-running products and only use the launcher for what is missing.
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
              onClick={() => runAction("Stop first stack", "/setup/first-stack/stop")}
              disabled={busyAction === "Stop first stack"}
            >
              {busyAction === "Stop first stack" ? "Stopping..." : "Stop first stack"}
            </Btn>
          </div>
        </div>

        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
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
        </div>

        <div style={{ fontSize: 12, color: "var(--text-dim)", lineHeight: 1.6 }}>
          {setup.launcher.message}
          {setup.launcher.repo_root ? ` Repo root: ${setup.launcher.repo_root}` : ""}
        </div>
      </div>

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
            onProductAction={runProductAction}
            onLoadLogs={loadLogs}
          />
        ))}
      </div>
    </div>
  );
}
