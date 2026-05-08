import { useEffect, useState } from "react";
import { Btn, EmptyState, Input, S, Tag } from "@patchhivehq/ui";

const FIRST_STACK_SLUGS = ["signal-hive", "trust-gate", "repo-reaper"];
const LANE_ORDER = [
  "Visibility",
  "Trust",
  "Memory",
  "Action",
  "Review",
  "Merge",
  "CI",
  "Dependencies",
  "Security",
  "Quality",
  "Control Plane",
];

const commandWorkspaceStyle = {
  display: "grid",
  gap: 14,
  gridTemplateColumns: "minmax(0, 1fr) clamp(320px, 32vw, 430px)",
  alignItems: "start",
};

const COMMAND_RAIL_TOP = 66;
const COMMAND_RAIL_BOTTOM_CLEARANCE = 72;
const COMMAND_RAIL_HEIGHT = `calc(100vh - ${COMMAND_RAIL_TOP + COMMAND_RAIL_BOTTOM_CLEARANCE}px)`;

const commandMainColumnStyle = {
  display: "grid",
  gap: 14,
  minWidth: 0,
};

const commandEvidenceGridStyle = {
  display: "grid",
  gap: 14,
  gridTemplateColumns: "minmax(280px, 0.9fr) minmax(0, 1.1fr)",
  alignItems: "stretch",
};

const commandEvidenceStackStyle = {
  display: "grid",
  gap: 14,
  minWidth: 0,
  alignContent: "start",
  height: "100%",
};

const focusRailStyle = {
  position: "sticky",
  top: COMMAND_RAIL_TOP,
  alignSelf: "start",
  minWidth: 0,
};

const focusPanelStyle = {
  ...S.panel,
  display: "flex",
  flexDirection: "column",
  gap: 10,
  height: COMMAND_RAIL_HEIGHT,
  maxHeight: COMMAND_RAIL_HEIGHT,
  boxSizing: "border-box",
  minHeight: 0,
  overflow: "hidden",
};

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

function imageTone(source, mode, status) {
  if (status === "fallback") return "var(--gold)";
  if (status === "pull") return "var(--green)";
  if (mode === "build") return "var(--gold)";
  if (source === "override") return "var(--blue)";
  if (source === "ghcr") return "var(--green)";
  return "var(--text-dim)";
}

function preflightTone(status) {
  if (status === "ready") return "var(--green)";
  if (status === "running") return "var(--blue)";
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

function fleetLaunchActive(job) {
  return ["queued", "running"].includes(job?.status || "");
}

function fleetLaunchTone(status) {
  if (status === "ready") return "var(--green)";
  if (status === "running" || status === "queued") return "var(--blue)";
  if (status === "attention" || status === "skipped") return "var(--gold)";
  return "var(--accent)";
}

function inputTypeForCredential(requirement) {
  if (requirement.redact || requirement.kind === "github_token") return "password";
  if (requirement.kind === "email") return "email";
  return "text";
}

function generateSecretValue(prefix = "ph-local") {
  const bytes = new Uint8Array(24);
  if (globalThis.crypto?.getRandomValues) {
    globalThis.crypto.getRandomValues(bytes);
  } else {
    for (let index = 0; index < bytes.length; index += 1) {
      bytes[index] = Math.floor(Math.random() * 256);
    }
  }
  return `${prefix}-${Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("")}`;
}

function generatedSecretPrefix(requirement) {
  if (requirement.key?.includes("WEBHOOK_SECRET")) return "ph-webhook";
  return "ph-local";
}

function mergedFleetProducts(setup, fleet) {
  const setupBySlug = new Map((setup?.products || []).map((item) => [item.runtime.slug, item]));
  const source = fleet?.length ? fleet : (setup?.products || []).map((item) => item.runtime);
  return source.map((runtime) => ({
    runtime,
    setupItem: setupBySlug.get(runtime.slug),
    firstStack: FIRST_STACK_SLUGS.includes(runtime.slug),
  }));
}

function productLooksRunning(product) {
  return ["online", "degraded"].includes(product?.runtime?.status) || Boolean(product?.setupItem?.launcher?.compose_running);
}

function selectedSetupItem(product) {
  if (!product) return null;
  return (
    product.setupItem || {
      runtime: product.runtime,
      auth_status: null,
      auth_status_error: "",
      pairing_ready: false,
      launcher: null,
      credentials: [],
    }
  );
}

function setupMission(setup, onlineCount, pairedCount) {
  const fleetLaunch = setup?.latest_fleet_launch;
  const smoke = setup?.latest_smoke;
  if (fleetLaunchActive(fleetLaunch)) {
    return {
      label: "Fleet Launching",
      tone: "var(--blue)",
      headline: "HiveCore is bringing more products online.",
      detail: fleetLaunch.summary,
    };
  }
  if (smoke?.status === "blocked") {
    return {
      label: "Blocked",
      tone: "var(--accent)",
      headline: "HiveCore found a blocker.",
      detail: smoke.summary,
    };
  }
  if (onlineCount < FIRST_STACK_SLUGS.length) {
    return {
      label: "Launch Needed",
      tone: "var(--gold)",
      headline: "Some first-stack products are not reachable.",
      detail: "Start the missing products, then HiveCore can pair and smoke-check the suite.",
    };
  }
  if (pairedCount < FIRST_STACK_SLUGS.length) {
    return {
      label: "Pairing Needed",
      tone: "var(--blue)",
      headline: "Products are running; machine pairing is next.",
      detail: "HiveCore needs scoped service tokens before it can dispatch suite actions safely.",
    };
  }
  if (smoke?.status === "ready") {
    return {
      label: "Suite Ready",
      tone: "var(--green)",
      headline: "First stack is under HiveCore control.",
      detail: smoke.summary,
    };
  }
  if (smoke?.status === "attention") {
    return {
      label: "Ready With Notes",
      tone: "var(--gold)",
      headline: "The stack is wired, with non-blocking notes.",
      detail: smoke.summary,
    };
  }
  return {
    label: "Smoke Needed",
    tone: "var(--accent)",
    headline: "The first stack is ready for evidence.",
    detail: "Run the HiveCore-controlled smoke check to prove health, auth, capabilities, and safe actions.",
  };
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

function CommandButton({ title, detail, color, disabled, active, onClick }) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      style={{
        display: "grid",
        gap: 5,
        textAlign: "left",
        padding: "12px 13px",
        borderRadius: 14,
        border: `1px solid ${color}55`,
        background: active
          ? `color-mix(in srgb, ${color} 18%, var(--bg-panel))`
          : "color-mix(in srgb, var(--bg) 58%, transparent)",
        color: "var(--text)",
        cursor: disabled ? "not-allowed" : "pointer",
        opacity: disabled ? 0.62 : 1,
      }}
    >
      <span style={{ fontSize: 12, fontWeight: 900, color }}>{title}</span>
      <span style={{ fontSize: 10, color: "var(--text-dim)", lineHeight: 1.45 }}>{detail}</span>
    </button>
  );
}

function fleetLaunchPlan(products) {
  const items = products || [];
  const native = items.filter((item) => item.runtime.slug === "hive-core").length;
  const controlled = items.filter((item) => item.runtime.slug !== "hive-core" && item.setupItem?.launcher);
  const ready = controlled.filter((item) => item.setupItem?.launcher?.start_ready).length;
  const running = controlled.filter(productLooksRunning).length;
  const launchable = controlled.filter((item) => !productLooksRunning(item) && item.setupItem?.launcher?.start_ready).length;
  const gated = controlled.filter((item) => !productLooksRunning(item) && !item.setupItem?.launcher?.start_ready).length;
  return {
    items,
    controlled,
    native,
    ready,
    running,
    launchable,
    gated,
    total: items.length || 11,
    canStartAll: controlled.length > 0 && launchable > 0 && gated === 0,
  };
}

function FleetStartPlan({ products, busyAction, launchJob, onAction }) {
  const plan = fleetLaunchPlan(products);
  const busy = Boolean(busyAction) || fleetLaunchActive(launchJob);
  return (
    <div style={{ ...S.panel, display: "grid", gap: 12, height: "100%", boxSizing: "border-box" }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 10, flexWrap: "wrap", alignItems: "center" }}>
        <div>
          <div style={{ fontSize: 15, fontWeight: 900 }}>Start All 11 Plan</div>
          <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.45 }}>
            HiveCore can now start only the stopped products that pass launcher preflight. Full fleet launch stays gated until every stopped target is ready.
          </div>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center", justifyContent: "flex-end" }}>
          <Tag color="var(--green)">{plan.ready} start-ready</Tag>
          <Tag color="var(--blue)">{plan.launchable} launchable now</Tag>
          <Tag color="var(--blue)">{plan.running} running</Tag>
          <Tag color={plan.gated ? "var(--gold)" : "var(--green)"}>{plan.gated} gated</Tag>
          <Tag color="var(--text-dim)">{plan.native} native</Tag>
          <Btn
            onClick={() => onAction("Start ready fleet", "/setup/fleet/start-ready")}
            disabled={busy || busyAction === "Start ready fleet" || plan.launchable === 0}
            color="var(--green)"
          >
            {fleetLaunchActive(launchJob)
              ? "Fleet running..."
              : busyAction === "Start ready fleet"
                ? "Starting..."
                : "Start ready fleet"}
          </Btn>
          <Btn
            onClick={() => onAction("Start all 11", "/setup/fleet/start-all")}
            disabled={busy || busyAction === "Start all 11" || !plan.canStartAll}
            color="var(--blue)"
          >
            {fleetLaunchActive(launchJob)
              ? "Fleet running..."
              : busyAction === "Start all 11"
                ? "Launching..."
                : "Start all 11"}
          </Btn>
        </div>
      </div>

      {launchJob && (
        <div
          style={{
            display: "grid",
            gap: 6,
            padding: "10px 11px",
            borderRadius: 12,
            border: `1px solid ${fleetLaunchTone(launchJob.status)}44`,
            background: "color-mix(in srgb, var(--bg) 56%, transparent)",
          }}
        >
          <div style={{ display: "flex", justifyContent: "space-between", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
            <div style={{ fontSize: 12, fontWeight: 900 }}>Latest fleet launch</div>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              <Tag color={fleetLaunchTone(launchJob.status)}>{launchJob.status}</Tag>
              {launchJob.mode && <Tag color="var(--blue)">{launchJob.mode}</Tag>}
            </div>
          </div>
          <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.5 }}>
            {launchJob.summary}
          </div>
        </div>
      )}

      <div style={{ display: "grid", gap: 8, gridTemplateColumns: "repeat(auto-fit, minmax(230px, 1fr))" }}>
        {plan.items.map((item) => {
          const launcher = item.setupItem?.launcher;
          const native = item.runtime.slug === "hive-core";
          const imageStatus = launcher?.image_status || launcher?.image_mode || "native";
          const blockers = launcher?.start_blockers || [];
          return (
            <div
              key={item.runtime.slug}
              style={{
                display: "grid",
                gap: 7,
                padding: 11,
                borderRadius: 12,
                border: `1px solid ${native ? "var(--blue)" : preflightTone(launcher?.preflight_status)}44`,
                background: "color-mix(in srgb, var(--bg) 52%, transparent)",
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center" }}>
                <span style={{ fontSize: 12, fontWeight: 900 }}>
                  {item.runtime.icon} {item.runtime.title}
                </span>
                <Tag color={native ? "var(--blue)" : preflightTone(launcher?.preflight_status)}>
                  {native ? "native" : launcher?.preflight_status || "unplanned"}
                </Tag>
              </div>
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                <Tag color={launcher?.compose_exists || native ? "var(--green)" : "var(--gold)"}>
                  compose {native ? "n/a" : launcher?.compose_exists ? "ready" : "missing"}
                </Tag>
                <Tag color={launcher?.env_exists || native ? "var(--green)" : "var(--gold)"}>
                  env {native ? "active" : launcher?.env_exists ? "ready" : "from example"}
                </Tag>
                <Tag color={imageTone(launcher?.image_source, launcher?.image_mode, imageStatus)}>
                  images {imageStatus}
                </Tag>
              </div>
              {launcher?.backend_image_ref && (
                <div style={{ fontSize: 10, color: "var(--text-dim)", lineHeight: 1.45, overflowWrap: "anywhere" }}>
                  {launcher.backend_image_ref}
                  <br />
                  {launcher.frontend_image_ref}
                </div>
              )}
              {blockers.length > 0 && (
                <div style={{ display: "grid", gap: 3, fontSize: 10, color: "var(--gold)", lineHeight: 1.4 }}>
                  {blockers.slice(0, 2).map((blocker) => (
                    <div key={blocker}>{blocker}</div>
                  ))}
                  {blockers.length > 2 && <div>{blockers.length - 2} more blocker(s)</div>}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function MissionControl({ setup, onlineCount, pairedCount, fleetCount, fleetPlan, busyAction, onRefresh, onAction }) {
  const mission = setupMission(setup, onlineCount, pairedCount);
  const busy = Boolean(busyAction) || fleetLaunchActive(setup.latest_fleet_launch);
  return (
    <div
      style={{
        ...S.panel,
        position: "relative",
        overflow: "hidden",
        display: "grid",
        gap: 18,
        borderColor: `${mission.tone}77`,
        background:
          "linear-gradient(135deg, color-mix(in srgb, var(--accent) 20%, var(--bg-panel)) 0%, var(--bg-panel) 44%, color-mix(in srgb, var(--blue) 19%, var(--bg-panel)) 100%)",
      }}
    >
      <div
        aria-hidden="true"
        style={{
          position: "absolute",
          inset: 0,
          background:
            "radial-gradient(circle at 82% 18%, color-mix(in srgb, var(--blue) 22%, transparent) 0, transparent 34%), linear-gradient(90deg, rgba(255,255,255,0.025) 1px, transparent 1px), linear-gradient(0deg, rgba(255,255,255,0.018) 1px, transparent 1px)",
          backgroundSize: "auto, 34px 34px, 34px 34px",
          opacity: 0.9,
          pointerEvents: "none",
        }}
      />
      <div style={{ position: "relative", display: "grid", gap: 14, gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))" }}>
        <div style={{ display: "grid", gap: 10 }}>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
            <Tag color={mission.tone}>{mission.label}</Tag>
            <Tag color={setup.launcher.available ? "var(--green)" : "var(--accent)"}>
              launcher {setup.launcher.available ? "available" : "offline"}
            </Tag>
            <Tag color={setup.suite_bootstrap_configured ? "var(--green)" : "var(--gold)"}>
              suite bootstrap {setup.suite_bootstrap_configured ? "configured" : "missing"}
            </Tag>
            {setup.launcher.image_mode && (
              <Tag color={imageTone("ghcr", setup.launcher.image_mode, setup.launcher.image_mode)}>
                images {setup.launcher.image_mode}
                {setup.launcher.image_tag ? `:${setup.launcher.image_tag}` : ""}
              </Tag>
            )}
          </div>
          <div style={{ fontSize: 34, lineHeight: 1.02, fontWeight: 950, letterSpacing: "-0.055em" }}>
            {mission.headline}
          </div>
          <div style={{ fontSize: 13, color: "var(--text-dim)", lineHeight: 1.6, maxWidth: 780 }}>
            {mission.detail}
          </div>
        </div>

        <div style={{ display: "grid", gap: 10 }}>
          <div style={{ display: "grid", gap: 8, gridTemplateColumns: "repeat(3, minmax(0, 1fr))" }}>
            <LaunchGauge label="Reachable" value={onlineCount} total={FIRST_STACK_SLUGS.length} tone={onlineCount === FIRST_STACK_SLUGS.length ? "var(--green)" : "var(--gold)"} />
            <LaunchGauge label="Paired" value={pairedCount} total={FIRST_STACK_SLUGS.length} tone={pairedCount === FIRST_STACK_SLUGS.length ? "var(--green)" : "var(--blue)"} />
            <LaunchGauge label="Fleet" value={fleetCount} total={11} tone="var(--accent)" />
          </div>
          <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.55 }}>
            {setup.launcher.message}
            {setup.launcher.repo_root ? ` Repo root: ${setup.launcher.repo_root}` : ""}
          </div>
        </div>
      </div>

      <div style={{ position: "relative", display: "grid", gap: 10 }}>
        <div style={{ fontSize: 11, letterSpacing: "0.16em", textTransform: "uppercase", color: "var(--text-dim)" }}>
          Next action rail
        </div>
        <div style={{ display: "grid", gap: 10, gridTemplateColumns: "repeat(auto-fit, minmax(170px, 1fr))" }}>
          <CommandButton
            title="Refresh telemetry"
            detail="Reload launcher, product health, pairing, and fleet data."
            color="var(--text-dim)"
            disabled={busy}
            onClick={onRefresh}
          />
          <CommandButton
            title="Start first stack"
            detail="Bring up missing SignalHive, TrustGate, and RepoReaper containers."
            color="var(--green)"
            active={onlineCount < FIRST_STACK_SLUGS.length}
            disabled={busy || busyAction === "Start first stack"}
            onClick={() => onAction("Start first stack", "/setup/first-stack/start")}
          />
          <CommandButton
            title="Start ready fleet"
            detail="Launch stopped managed products whose compose, env, credentials, and images pass preflight."
            color="var(--green)"
            active={fleetPlan.launchable > 0}
            disabled={busy || busyAction === "Start ready fleet" || fleetPlan.launchable === 0}
            onClick={() => onAction("Start ready fleet", "/setup/fleet/start-ready")}
          />
          <CommandButton
            title="Start all 11"
            detail="Launch every stopped managed product, but only when nothing remaining is gated."
            color="var(--blue)"
            active={fleetPlan.canStartAll}
            disabled={busy || busyAction === "Start all 11" || !fleetPlan.canStartAll}
            onClick={() => onAction("Start all 11", "/setup/fleet/start-all")}
          />
          <CommandButton
            title="Pair running products"
            detail="Provision scoped service tokens for HiveCore machine control."
            color="var(--blue)"
            active={onlineCount === FIRST_STACK_SLUGS.length && pairedCount < FIRST_STACK_SLUGS.length}
            disabled={busy || busyAction === "Pair running products"}
            onClick={() => onAction("Pair running products", "/setup/first-stack/pair")}
          />
          <CommandButton
            title="First-stack smoke"
            detail="Verify reachability, auth, capabilities, and safe product actions."
            color="var(--accent)"
            active={pairedCount === FIRST_STACK_SLUGS.length && setup.latest_smoke?.status !== "ready"}
            disabled={busy || busyAction === "First-stack smoke"}
            onClick={() => onAction("First-stack smoke", "/setup/smoke/first-stack")}
          />
          <CommandButton
            title="Read-only smoke"
            detail="Check visibility and triage products without dispatching product actions."
            color="var(--blue)"
            disabled={busy || busyAction === "Read-only smoke"}
            onClick={() => onAction("Read-only smoke", "/setup/smoke/read-only-fleet")}
          />
          <CommandButton
            title="Dry-run smoke"
            detail="Exercise RepoReaper only through dry-run mode; no PRs are opened."
            color="var(--gold)"
            disabled={busy || busyAction === "Dry-run smoke"}
            onClick={() => onAction("Dry-run smoke", "/setup/smoke/write-dry-run")}
          />
          <CommandButton
            title="Stop first stack"
            detail="Stop the first operational trio without removing local data."
            color="var(--gold)"
            disabled={busy || busyAction === "Stop first stack"}
            onClick={() => onAction("Stop first stack", "/setup/first-stack/stop")}
          />
        </div>
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
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
          {smoke.tier && <Tag color="var(--blue)">{smoke.tier}</Tag>}
          <Tag color={smokeTone(smoke.status)}>{smoke.status}</Tag>
        </div>
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

function FleetLaunchEvidence({ job }) {
  if (!job) return null;
  return (
    <div style={{ ...S.panel, display: "grid", gap: 12, borderColor: `${fleetLaunchTone(job.status)}66` }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 10, flexWrap: "wrap", alignItems: "center" }}>
        <div>
          <div style={{ fontSize: 11, letterSpacing: "0.16em", textTransform: "uppercase", color: "var(--text-dim)" }}>
            Fleet launch job
          </div>
          <div style={{ fontSize: 18, fontWeight: 900 }}>{job.summary}</div>
          <div style={{ fontSize: 11, color: "var(--text-dim)" }}>
            {job.started_at}
            {job.finished_at ? ` -> ${job.finished_at}` : " -> in progress"}
          </div>
        </div>
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
          {job.mode && <Tag color="var(--blue)">{job.mode}</Tag>}
          <Tag color={fleetLaunchTone(job.status)}>{job.status}</Tag>
        </div>
      </div>

      <div style={{ display: "grid", gap: 10, gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))" }}>
        {(job.steps || []).map((step) => (
          <div
            key={`${job.id}-${step.slug}`}
            style={{
              display: "grid",
              gap: 7,
              padding: 12,
              border: `1px solid ${fleetLaunchTone(step.status)}44`,
              borderRadius: 12,
              background: "rgba(0,0,0,0.18)",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center" }}>
              <div style={{ fontSize: 13, fontWeight: 800 }}>{step.title}</div>
              <Tag color={fleetLaunchTone(step.status)}>{step.status}</Tag>
            </div>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {step.phase && <Tag color="var(--text-dim)">{step.phase}</Tag>}
            </div>
            <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.45 }}>{step.message}</div>
          </div>
        ))}
      </div>
    </div>
  );
}

function EvidenceTimeline({ setup }) {
  const fleetLaunch = setup.latest_fleet_launch;
  const smoke = setup.latest_smoke;
  const actions = setup.actions || [];
  const importantSteps = (smoke?.steps || [])
    .filter((step) => step.status !== "pass" || ["preflight", "safe-action"].includes(step.check))
    .slice(0, 8);
  const timeline = [
    ...(fleetLaunch
      ? [
          {
            id: fleetLaunch.id,
            title: "Latest fleet launch",
            detail: fleetLaunch.summary,
            tone: fleetLaunchTone(fleetLaunch.status),
            status: fleetLaunch.status,
          },
        ]
      : []),
    ...(smoke
      ? [
          {
            id: smoke.id,
            title: "Latest smoke",
            detail: smoke.summary,
            tone: smokeTone(smoke.status),
            status: smoke.status,
          },
        ]
      : [
          {
            id: "smoke-missing",
            title: "Smoke not recorded",
            detail: "Run the HiveCore smoke check to create suite evidence.",
            tone: "var(--gold)",
            status: "needed",
          },
        ]),
    ...importantSteps.map((step) => ({
      id: `${step.slug}-${step.check}`,
      title: `${step.title} / ${step.check}`,
      detail: step.message,
      tone: smokeTone(step.status),
      status: step.status,
    })),
    ...actions.slice(0, 4).map((action, index) => ({
      id: `action-${index}-${action}`,
      title: "Setup action",
      detail: action,
      tone: "var(--blue)",
      status: "logged",
    })),
  ].slice(0, 10);

  return (
    <div style={{ ...S.panel, display: "grid", gap: 12 }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 10, flexWrap: "wrap", alignItems: "center" }}>
        <div>
          <div style={{ fontSize: 15, fontWeight: 900 }}>Evidence Timeline</div>
          <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.45 }}>
            The short proof trail behind HiveCore's current suite-ready call.
          </div>
        </div>
        {smoke && <Tag color={smokeTone(smoke.status)}>{smoke.status}</Tag>}
      </div>
      <div style={{ display: "grid", gap: 8 }}>
        {timeline.map((item) => (
          <div
            key={item.id}
            style={{
              display: "grid",
              gap: 5,
              padding: "10px 11px",
              borderRadius: 12,
              border: `1px solid ${item.tone}44`,
              background: "color-mix(in srgb, var(--bg) 50%, transparent)",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center" }}>
              <span style={{ fontSize: 12, fontWeight: 900 }}>{item.title}</span>
              <Tag color={item.tone}>{item.status}</Tag>
            </div>
            <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.45 }}>{item.detail}</div>
          </div>
        ))}
      </div>
    </div>
  );
}

function FleetLaneMap({ products, selectedSlug, onSelect }) {
  const grouped = products.reduce((acc, product) => {
    const lane = product.runtime.lane || "Other";
    acc[lane] = acc[lane] || [];
    acc[lane].push(product);
    return acc;
  }, {});
  const lanes = [
    ...LANE_ORDER.filter((lane) => grouped[lane]),
    ...Object.keys(grouped).filter((lane) => !LANE_ORDER.includes(lane)).sort(),
  ];

  return (
    <div style={{ ...S.panel, display: "grid", gap: 12 }}>
      <div>
        <div style={{ fontSize: 15, fontWeight: 900 }}>11-Product Fleet Map</div>
        <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.45 }}>
          Compact lanes first; details stay in the selected-product drawer.
        </div>
      </div>
      <div style={{ display: "grid", gap: 10, gridTemplateColumns: "repeat(auto-fit, minmax(210px, 1fr))" }}>
        {lanes.map((lane) => (
          <div
            key={lane}
            style={{
              display: "grid",
              gap: 8,
              padding: 11,
              border: "1px solid var(--border)",
              borderRadius: 14,
              background: "rgba(0,0,0,0.17)",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center" }}>
              <span style={{ fontSize: 11, letterSpacing: "0.14em", textTransform: "uppercase", color: "var(--text-dim)" }}>{lane}</span>
              <Tag color="var(--blue)">{grouped[lane].length}</Tag>
            </div>
            <div style={{ display: "grid", gap: 7 }}>
              {grouped[lane].map((product) => {
                const runtime = product.runtime;
                const selected = runtime.slug === selectedSlug;
                const managed = Boolean(product.setupItem?.launcher);
                return (
                  <button
                    type="button"
                    key={runtime.slug}
                    onClick={() => onSelect(runtime.slug)}
                    style={{
                      display: "grid",
                      gap: 4,
                      textAlign: "left",
                      padding: "9px 10px",
                      borderRadius: 11,
                      border: `1px solid ${selected ? statusColor(runtime.status) : "var(--border)"}`,
                      background: selected
                        ? `color-mix(in srgb, ${statusColor(runtime.status)} 16%, var(--bg-panel))`
                        : "color-mix(in srgb, var(--bg) 48%, transparent)",
                      color: "var(--text)",
                      cursor: "pointer",
                    }}
                  >
                    <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center" }}>
                      <span style={{ fontSize: 12, fontWeight: 900 }}>
                        {runtime.icon} {runtime.title}
                      </span>
                      <span
                        aria-label={runtime.status}
                        style={{
                          width: 9,
                          height: 9,
                          borderRadius: "50%",
                          background: statusColor(runtime.status),
                          boxShadow: `0 0 14px ${statusColor(runtime.status)}88`,
                          flex: "0 0 auto",
                        }}
                      />
                    </div>
                    <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                      <Tag color={statusColor(runtime.status)}>{runtime.status}</Tag>
                      {product.firstStack && <Tag color="var(--accent)">first stack</Tag>}
                      {managed && <Tag color="var(--green)">launcher</Tag>}
                      {product.setupItem?.launcher?.preflight_status && (
                        <Tag color={preflightTone(product.setupItem.launcher.preflight_status)}>
                          {product.setupItem.launcher.preflight_status}
                        </Tag>
                      )}
                    </div>
                  </button>
                );
              })}
            </div>
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
  onGenerateCredential,
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
  const canStart = Boolean(launcher?.start_ready);
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
            {launcher.image_mode && (
              <Tag color={imageTone(launcher.image_source, launcher.image_mode, launcher.image_status)}>
                images {launcher.image_status || launcher.image_mode}
              </Tag>
            )}
            {launcher.image_source && (
              <Tag color={imageTone(launcher.image_source, launcher.image_mode, launcher.image_status)}>{launcher.image_source}</Tag>
            )}
            <Tag color={launcher.start_ready ? "var(--green)" : "var(--gold)"}>
              start {launcher.start_ready ? "ready" : "gated"}
            </Tag>
            {launcher.image_tag && <Tag color="var(--blue)">tag {launcher.image_tag}</Tag>}
            {launcher.image_pull_policy && launcher.image_mode !== "build" && (
              <Tag color="var(--text-dim)">pull {launcher.image_pull_policy}</Tag>
            )}
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
            {(launcher.backend_image_ref || launcher.frontend_image_ref) && (
              <>
                <div>
                  Backend image:{" "}
                  <span style={{ color: "var(--text)", overflowWrap: "anywhere" }}>
                    {launcher.backend_image_ref || "not reported"}
                  </span>
                </div>
                <div>
                  Frontend image:{" "}
                  <span style={{ color: "var(--text)", overflowWrap: "anywhere" }}>
                    {launcher.frontend_image_ref || "not reported"}
                  </span>
                </div>
              </>
            )}
          </div>
          {launcher.blockers?.length > 0 && (
            <div style={{ display: "grid", gap: 4, color: "var(--accent)" }}>
              {launcher.blockers.map((blocker) => (
                <div key={blocker}>{blocker}</div>
              ))}
            </div>
          )}
          {launcher.start_blockers?.length > 0 && (
            <div style={{ display: "grid", gap: 4, color: "var(--gold)" }}>
              {launcher.start_blockers.map((blocker) => (
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
              <div style={{ fontSize: 12, fontWeight: 900 }}>First-run setup</div>
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
            {credentials.some((requirement) => requirement.kind === "github_token") && (
              <div style={{ fontSize: 10, color: "var(--text-dim)", lineHeight: 1.45 }}>
                Use a fine-grained GitHub token. The recommended minimum scopes for each product are listed under its token field.
              </div>
            )}
            {credentials.map((requirement) => (
              <label key={requirement.key} style={{ ...S.field }}>
                <span style={S.label}>
                  {requirement.label} {requirement.required ? "" : "(optional)"}
                </span>
                <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                  <Input
                    type={inputTypeForCredential(requirement)}
                    value={credentialDraft[requirement.key] || ""}
                    onChange={(value) => onCredentialChange(item.runtime.slug, requirement.key, value)}
                    placeholder={requirement.configured ? "Configured - leave blank to keep current value" : requirement.key}
                  />
                  {requirement.kind === "generated_secret" && (
                    <Btn
                      onClick={() => onGenerateCredential(item.runtime.slug, requirement)}
                      disabled={busy}
                      color="var(--blue)"
                    >
                      Generate
                    </Btn>
                  )}
                </div>
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

      {item.runtime.slug !== "hive-core" && item.launcher && (
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <Btn onClick={() => onProductAction(item.runtime.slug, "start")} disabled={busy || !canStart} color="var(--green)">
            {busyAction === `${actionPrefix}start` ? "Starting..." : "Start"}
          </Btn>
          <Btn onClick={() => onProductAction(item.runtime.slug, "restart")} disabled={busy || !canStart} color="var(--blue)">
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
  const [fleet, setFleet] = useState([]);
  const [selectedSlug, setSelectedSlug] = useState("hive-core");
  const [busyAction, setBusyAction] = useState("");
  const [logs, setLogs] = useState(null);
  const [credentialDrafts, setCredentialDrafts] = useState({});
  const [credentialResults, setCredentialResults] = useState({});

  async function loadFleet(fallbackSetup) {
    try {
      const data = await fetchEnvelope("/products");
      setFleet(Array.isArray(data) ? data : []);
    } catch {
      setFleet((fallbackSetup?.products || []).map((item) => item.runtime));
    }
  }

  async function refresh() {
    setRunning(true);
    setError("");
    try {
      const data = await fetchEnvelope("/setup/first-stack");
      setSetup(data);
      await loadFleet(data);
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
      await loadFleet(data);
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
      await loadFleet(data);
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

  function generateCredential(slug, requirement) {
    setCredentialDraft(slug, requirement.key, generateSecretValue(generatedSecretPrefix(requirement)));
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
      await loadFleet(data);
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

  useEffect(() => {
    if (!fleetLaunchActive(setup?.latest_fleet_launch)) return undefined;
    const timer = setInterval(() => {
      refresh();
    }, 3000);
    return () => clearInterval(timer);
  }, [setup?.latest_fleet_launch?.id, setup?.latest_fleet_launch?.status]);

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

  const fleetProducts = mergedFleetProducts(setup, fleet);
  const fleetPlan = fleetLaunchPlan(fleetProducts);
  const firstStackProducts = setup.products.filter((item) => FIRST_STACK_SLUGS.includes(item.runtime.slug));
  const onlineCount = firstStackProducts.filter((item) => ["online", "degraded"].includes(item.runtime.status)).length;
  const pairedCount = firstStackProducts.filter((item) => item.runtime.service_token_configured).length;
  const selectedProduct =
    fleetProducts.find((product) => product.runtime.slug === selectedSlug) ||
    fleetProducts.find((product) => product.runtime.slug === "hive-core") ||
    fleetProducts[0];
  const selectedItem = selectedSetupItem(selectedProduct);

  return (
    <div style={{ display: "grid", gap: 16, paddingBottom: 16 }}>
      <MissionControl
        setup={setup}
        onlineCount={onlineCount}
        pairedCount={pairedCount}
        fleetCount={fleetProducts.length}
        fleetPlan={fleetPlan}
        busyAction={busyAction}
        onRefresh={refresh}
        onAction={runAction}
      />

      <FleetStartPlan
        products={fleetProducts}
        busyAction={busyAction}
        launchJob={setup.latest_fleet_launch}
        onAction={runAction}
      />

      <div style={commandWorkspaceStyle}>
        <div style={commandMainColumnStyle}>
          <FleetLaneMap products={fleetProducts} selectedSlug={selectedProduct?.runtime.slug} onSelect={setSelectedSlug} />
          <div style={commandEvidenceGridStyle}>
            <EvidenceTimeline setup={setup} />
            <div style={commandEvidenceStackStyle}>
              <FleetLaunchEvidence job={setup.latest_fleet_launch} />
              <SmokeEvidence smoke={setup.latest_smoke} />
            </div>
          </div>
        </div>
        <div style={focusRailStyle}>
          <div style={focusPanelStyle}>
            <div>
              <div style={{ fontSize: 15, fontWeight: 900 }}>Focused Product</div>
              <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.45 }}>
                Select any product from the fleet map. HiveCore only shows launcher/env controls when that product is managed by this setup flow.
              </div>
            </div>
            <div
              style={{
                flex: "1 1 auto",
                minHeight: 0,
                overflowY: "auto",
                overflowX: "hidden",
                overscrollBehavior: "contain",
                paddingRight: 4,
                paddingBottom: 4,
              }}
            >
              {selectedItem ? (
                <ProductCard
                  item={selectedItem}
                  busyAction={busyAction}
                  credentialDraft={credentialDrafts[selectedItem.runtime.slug] || {}}
                  credentialResult={credentialResults[selectedItem.runtime.slug]}
                  onCredentialChange={setCredentialDraft}
                  onSaveCredentials={saveCredentials}
                  onGenerateCredential={generateCredential}
                  onValidateGithubToken={validateGithubToken}
                  onProductAction={runProductAction}
                  onLoadLogs={loadLogs}
                />
              ) : (
                <EmptyState icon="⬢" text="Select a product to inspect." />
              )}
            </div>
          </div>
        </div>
      </div>

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

    </div>
  );
}
