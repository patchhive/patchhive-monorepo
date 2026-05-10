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

const FLEET_FILTERS = [
  { id: "all", label: "All" },
  { id: "attention", label: "Needs attention" },
  { id: "running", label: "Running" },
  { id: "launchable", label: "Launchable" },
  { id: "gated", label: "Gated" },
  { id: "first-stack", label: "First stack" },
];

const COMMAND_RAIL_TOP = 66;
const COMMAND_RAIL_BOTTOM_CLEARANCE = 72;
const COMMAND_RAIL_HEIGHT = `calc(100vh - ${COMMAND_RAIL_TOP + COMMAND_RAIL_BOTTOM_CLEARANCE}px)`;

const commandConsoleStyle = {
  display: "grid",
  gap: 12,
  gridTemplateColumns: "260px minmax(0, 1fr) clamp(340px, 30vw, 420px)",
  alignItems: "start",
};

const commandRailStyle = {
  position: "sticky",
  top: COMMAND_RAIL_TOP,
  display: "grid",
  gap: 10,
  maxHeight: COMMAND_RAIL_HEIGHT,
  overflowY: "auto",
  paddingRight: 2,
};

const commandTopGridStyle = {
  display: "grid",
  gap: 12,
  gridTemplateColumns: "minmax(0, 1.4fr) minmax(320px, 0.9fr)",
  alignItems: "stretch",
};

const commandMainColumnStyle = {
  display: "grid",
  gap: 12,
  minWidth: 0,
};

const commandEvidenceGridStyle = {
  display: "grid",
  gap: 12,
  gridTemplateColumns: "minmax(280px, 0.9fr) minmax(0, 1.1fr)",
  alignItems: "stretch",
};

const commandEvidenceStackStyle = {
  display: "grid",
  gap: 12,
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
  ...commandSurfaceStyle("var(--blue)"),
  display: "flex",
  flexDirection: "column",
  gap: 10,
  height: COMMAND_RAIL_HEIGHT,
  maxHeight: COMMAND_RAIL_HEIGHT,
  boxSizing: "border-box",
  minHeight: 0,
  overflow: "hidden",
};

const commandKickerStyle = {
  fontSize: 10,
  letterSpacing: 0,
  textTransform: "uppercase",
  color: "var(--text-dim)",
};

const commandTitleStyle = {
  fontSize: 18,
  fontWeight: 900,
  letterSpacing: 0,
};

const commandBodyStyle = {
  fontSize: 11,
  color: "var(--text-dim)",
  lineHeight: 1.55,
};

const commandPanelScrollStyle = {
  maxHeight: "min(520px, 62vh)",
  overflow: "auto",
};

function CommandCenterStyles() {
  return (
    <style>{`
      @keyframes ph-core-breathe {
        0%, 100% { transform: scale(0.96); opacity: 0.54; }
        50% { transform: scale(1.08); opacity: 0.94; }
      }

      @keyframes ph-signal-flow {
        0% { transform: translateX(-105%); opacity: 0; }
        16% { opacity: 1; }
        84% { opacity: 1; }
        100% { transform: translateX(315%); opacity: 0; }
      }

      @keyframes ph-node-pulse {
        0%, 100% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--node-tone) 28%, transparent); }
        50% { box-shadow: 0 0 0 5px transparent; }
      }

      .ph-core-visual {
        position: absolute;
        right: 12px;
        top: 12px;
        width: 74px;
        height: 74px;
        pointer-events: none;
        opacity: 0.9;
      }

      .ph-core-visual::before,
      .ph-core-visual::after {
        content: "";
        position: absolute;
        inset: 8px;
        border: 1px solid color-mix(in srgb, var(--blue) 44%, transparent);
        border-radius: 8px;
        transform: rotate(45deg);
        animation: ph-core-breathe 4.8s ease-in-out infinite;
      }

      .ph-core-visual::after {
        inset: 22px;
        border-color: color-mix(in srgb, var(--accent) 58%, transparent);
        animation-delay: -1.9s;
      }

      .ph-core-mark {
        position: absolute;
        inset: 25px;
        display: grid;
        place-items: center;
        border-radius: 6px;
        color: var(--text);
        background: color-mix(in srgb, var(--blue) 36%, var(--bg-panel));
        box-shadow: 0 0 24px color-mix(in srgb, var(--blue) 32%, transparent);
        z-index: 1;
      }

      .ph-command-console {
        isolation: isolate;
      }

      .ph-topology-board {
        position: relative;
        display: grid;
        gap: 12px;
        padding: 12px;
        border-radius: 8px;
        border: 1px solid color-mix(in srgb, var(--accent) 22%, var(--border));
        background:
          linear-gradient(180deg, color-mix(in srgb, var(--bg-panel) 96%, #051318) 0%, color-mix(in srgb, var(--bg) 70%, #030608) 100%),
          repeating-linear-gradient(90deg, rgba(255,255,255,0.026) 0 1px, transparent 1px 42px);
        overflow: hidden;
      }

      .ph-topology-board::before {
        content: "";
        position: absolute;
        inset: 0;
        background:
          linear-gradient(90deg, transparent, color-mix(in srgb, var(--accent) 10%, transparent), transparent),
          linear-gradient(180deg, color-mix(in srgb, var(--blue) 9%, transparent), transparent 44%);
        pointer-events: none;
      }

      .ph-topology-header,
      .ph-lane-stack,
      .ph-filter-row {
        position: relative;
        z-index: 1;
      }

      .ph-topology-header {
        display: grid;
        grid-template-columns: 170px minmax(0, 1fr) auto;
        gap: 12px;
        align-items: center;
      }

      .ph-core-chip {
        display: flex;
        align-items: center;
        gap: 8px;
        min-height: 36px;
        padding: 8px 10px;
        border-radius: 8px;
        border: 1px solid color-mix(in srgb, var(--blue) 36%, var(--border));
        background: color-mix(in srgb, var(--blue) 10%, var(--bg));
        color: var(--blue);
        font-size: 11px;
        font-weight: 900;
      }

      .ph-live-bus {
        position: relative;
        height: 4px;
        border-radius: 999px;
        background: linear-gradient(90deg, var(--blue), var(--accent), var(--green), var(--gold));
        box-shadow: 0 0 20px color-mix(in srgb, var(--accent) 24%, transparent);
        overflow: hidden;
      }

      .ph-live-bus::after {
        content: "";
        position: absolute;
        inset: 0;
        width: 34%;
        background: linear-gradient(90deg, transparent, rgba(255,255,255,0.82), transparent);
        animation: ph-signal-flow 3.6s linear infinite;
      }

      .ph-filter-row {
        display: flex;
        gap: 6px;
        flex-wrap: wrap;
      }

      .ph-lane-stack {
        display: grid;
        gap: 8px;
      }

      .ph-lane-row {
        --lane-tone: var(--accent);
        position: relative;
        display: grid;
        grid-template-columns: 118px minmax(0, 1fr);
        gap: 10px;
        align-items: stretch;
      }

      .ph-lane-label {
        display: grid;
        align-content: center;
        gap: 5px;
        min-height: 72px;
        padding: 9px 10px;
        border-radius: 8px;
        border: 1px solid color-mix(in srgb, var(--lane-tone) 24%, var(--border));
        background: color-mix(in srgb, var(--lane-tone) 7%, var(--bg));
      }

      .ph-lane-title {
        color: var(--lane-tone);
        font-size: 10px;
        font-weight: 950;
        text-transform: uppercase;
      }

      .ph-lane-count {
        color: var(--text-dim);
        font-size: 10px;
      }

      .ph-lane-track {
        position: relative;
        min-height: 72px;
        padding: 9px 10px;
        border-radius: 8px;
        border: 1px solid color-mix(in srgb, var(--lane-tone) 16%, var(--border));
        background: color-mix(in srgb, var(--bg) 56%, transparent);
        overflow: hidden;
      }

      .ph-signal-rail {
        position: absolute;
        left: 11px;
        right: 11px;
        top: 50%;
        height: 1px;
        background: linear-gradient(90deg, transparent, color-mix(in srgb, var(--lane-tone) 38%, transparent), transparent);
        opacity: 0.72;
      }

      .ph-signal-rail::after {
        content: "";
        position: absolute;
        inset-block: -1px;
        left: 0;
        width: 26%;
        border-radius: 999px;
        background: linear-gradient(90deg, transparent, var(--lane-tone), transparent);
        animation: ph-signal-flow 4.9s linear infinite;
      }

      .ph-node-grid {
        position: relative;
        z-index: 1;
        display: grid;
        gap: 8px;
        grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
      }

      .ph-product-node {
        --node-tone: var(--accent);
        display: grid;
        gap: 6px;
        min-height: 76px;
        padding: 9px 10px;
        text-align: left;
        border-radius: 8px;
        border: 1px solid color-mix(in srgb, var(--node-tone) 18%, var(--border));
        background:
          linear-gradient(180deg, color-mix(in srgb, var(--node-tone) 8%, var(--bg-panel)) 0%, color-mix(in srgb, var(--bg) 70%, transparent) 100%);
        color: var(--text);
        cursor: pointer;
        font-family: inherit;
        transition: transform 160ms ease, border-color 160ms ease, background 160ms ease;
      }

      .ph-product-node:hover {
        transform: translateY(-2px);
        border-color: color-mix(in srgb, var(--node-tone) 46%, var(--border));
      }

      .ph-product-node.is-selected {
        border-color: var(--node-tone);
        background:
          linear-gradient(180deg, color-mix(in srgb, var(--node-tone) 16%, var(--bg-panel)) 0%, color-mix(in srgb, var(--bg) 58%, transparent) 100%);
        animation: ph-node-pulse 2.8s ease-in-out infinite;
      }

      .ph-product-node.is-attention {
        border-color: color-mix(in srgb, var(--gold) 42%, var(--border));
      }

      .ph-cortex-shell {
        position: relative;
        display: grid;
        gap: 12px;
        min-height: clamp(520px, 44vh, 580px);
        padding: 14px;
        border-radius: 8px;
        border: 1px solid color-mix(in srgb, var(--accent) 26%, var(--border));
        background:
          radial-gradient(circle at 50% 46%, color-mix(in srgb, var(--blue) 10%, transparent) 0 17%, transparent 18%),
          linear-gradient(180deg, color-mix(in srgb, var(--bg-panel) 90%, #020509) 0%, color-mix(in srgb, var(--bg) 72%, #010406) 100%);
        overflow: hidden;
      }

      .ph-cortex-shell::before {
        content: "";
        position: absolute;
        inset: 0;
        background:
          linear-gradient(90deg, rgba(255,255,255,0.022) 1px, transparent 1px),
          linear-gradient(0deg, rgba(255,255,255,0.018) 1px, transparent 1px);
        background-size: 38px 38px;
        mask-image: radial-gradient(circle at 50% 46%, black 0%, transparent 74%);
        pointer-events: none;
      }

      .ph-cortex-shell::after {
        content: "";
        position: absolute;
        left: 8%;
        right: 8%;
        top: 49%;
        height: 1px;
        background: linear-gradient(90deg, transparent, color-mix(in srgb, var(--accent) 44%, transparent), transparent);
        box-shadow:
          0 -130px 0 color-mix(in srgb, var(--blue) 13%, transparent),
          0 130px 0 color-mix(in srgb, var(--green) 12%, transparent);
        pointer-events: none;
      }

      .ph-cortex-topbar {
        position: relative;
        z-index: 2;
        display: flex;
        justify-content: space-between;
        gap: 12px;
        flex-wrap: wrap;
        align-items: flex-start;
      }

      .ph-cortex-title {
        display: grid;
        gap: 4px;
      }

      .ph-cortex-stage {
        position: relative;
        z-index: 1;
        min-height: clamp(390px, 34vh, 430px);
      }

      .ph-orbit-ring {
        position: absolute;
        left: 50%;
        top: 50%;
        border: 1px solid color-mix(in srgb, var(--accent) 18%, transparent);
        border-radius: 50%;
        transform: translate(-50%, -50%);
        pointer-events: none;
      }

      .ph-orbit-ring.one {
        width: 42%;
        height: 42%;
        border-color: color-mix(in srgb, var(--blue) 18%, transparent);
      }

      .ph-orbit-ring.two {
        width: 68%;
        height: 68%;
      }

      .ph-orbit-ring.three {
        width: 88%;
        height: 82%;
        border-color: color-mix(in srgb, var(--green) 14%, transparent);
      }

      .ph-cortex-core {
        position: absolute;
        left: 50%;
        top: 50%;
        z-index: 3;
        display: grid;
        place-items: center;
        gap: 9px;
        width: 250px;
        min-height: 198px;
        padding: 18px;
        text-align: center;
        border-radius: 8px;
        border: 1px solid color-mix(in srgb, var(--accent) 44%, var(--border));
        background:
          linear-gradient(180deg, color-mix(in srgb, var(--bg-panel) 78%, #08161c) 0%, color-mix(in srgb, var(--bg) 72%, #020509) 100%);
        box-shadow:
          0 0 0 1px rgba(255,255,255,0.03) inset,
          0 0 48px color-mix(in srgb, var(--accent) 14%, transparent);
        transform: translate(-50%, -50%);
      }

      .ph-cortex-core::before,
      .ph-cortex-core::after {
        content: "";
        position: absolute;
        inset: -14px;
        border: 1px solid color-mix(in srgb, var(--blue) 24%, transparent);
        border-radius: 12px;
        animation: ph-core-breathe 5.4s ease-in-out infinite;
        pointer-events: none;
      }

      .ph-cortex-core::after {
        inset: -27px;
        border-color: color-mix(in srgb, var(--accent) 18%, transparent);
        animation-delay: -2s;
      }

      .ph-cortex-glyph {
        display: grid;
        place-items: center;
        width: 48px;
        height: 48px;
        border-radius: 8px;
        color: var(--text);
        background: color-mix(in srgb, var(--accent) 22%, var(--bg));
        border: 1px solid color-mix(in srgb, var(--accent) 42%, var(--border));
        font-size: 24px;
      }

      .ph-cortex-mission {
        color: var(--text);
        font-size: 22px;
        line-height: 1.05;
        font-weight: 950;
      }

      .ph-cortex-subcopy {
        max-width: 210px;
        color: var(--text-dim);
        font-size: 11px;
        line-height: 1.45;
      }

      .ph-cortex-counters {
        display: grid;
        grid-template-columns: repeat(3, minmax(0, 1fr));
        gap: 6px;
        width: 100%;
      }

      .ph-cortex-counter {
        display: grid;
        gap: 2px;
        padding: 7px 6px;
        border-radius: 7px;
        border: 1px solid color-mix(in srgb, var(--accent) 16%, var(--border));
        background: color-mix(in srgb, var(--bg) 60%, transparent);
      }

      .ph-cortex-counter strong {
        font-size: 15px;
        line-height: 1;
      }

      .ph-cortex-counter span {
        color: var(--text-dim);
        font-size: 9px;
        text-transform: uppercase;
      }

      .ph-orbit-node {
        --node-tone: var(--accent);
        position: absolute;
        z-index: 4;
        display: grid;
        gap: 6px;
        width: 148px;
        min-height: 82px;
        padding: 9px 10px;
        text-align: left;
        border-radius: 8px;
        border: 1px solid color-mix(in srgb, var(--node-tone) 24%, var(--border));
        background:
          linear-gradient(180deg, color-mix(in srgb, var(--node-tone) 10%, var(--bg-panel)) 0%, color-mix(in srgb, var(--bg) 72%, #020408) 100%);
        color: var(--text);
        cursor: pointer;
        font-family: inherit;
        transform: translate(-50%, -50%);
        transition: opacity 160ms ease, transform 160ms ease, border-color 160ms ease;
      }

      .ph-orbit-node:hover {
        border-color: color-mix(in srgb, var(--node-tone) 52%, var(--border));
        transform: translate(-50%, -52%);
      }

      .ph-orbit-node.is-selected {
        border-color: var(--node-tone);
        box-shadow:
          0 0 0 1px color-mix(in srgb, var(--node-tone) 18%, transparent) inset,
          0 0 26px color-mix(in srgb, var(--node-tone) 20%, transparent);
        animation: ph-node-pulse 2.6s ease-in-out infinite;
      }

      .ph-orbit-node.is-muted {
        opacity: 0.28;
      }

      .ph-orbit-node.is-attention {
        border-color: color-mix(in srgb, var(--gold) 56%, var(--border));
      }

      .ph-orbit-link {
        position: absolute;
        left: 50%;
        top: 50%;
        z-index: 2;
        height: 1px;
        width: var(--link-length);
        transform-origin: 0 50%;
        transform: rotate(var(--link-angle));
        background: linear-gradient(90deg, color-mix(in srgb, var(--node-tone) 52%, transparent), transparent);
        opacity: var(--link-opacity);
        pointer-events: none;
      }

      .ph-lane-strip {
        position: relative;
        z-index: 2;
        display: grid;
        gap: 7px;
        grid-template-columns: repeat(auto-fit, minmax(112px, 1fr));
      }

      .ph-lane-chip {
        --lane-tone: var(--accent);
        display: flex;
        justify-content: space-between;
        gap: 6px;
        align-items: center;
        padding: 8px 9px;
        border-radius: 7px;
        border: 1px solid color-mix(in srgb, var(--lane-tone) 20%, var(--border));
        background: color-mix(in srgb, var(--lane-tone) 6%, var(--bg));
        color: var(--text-dim);
        font-size: 10px;
      }

      .ph-lane-chip strong {
        color: var(--lane-tone);
        font-size: 10px;
      }

      .ph-node-top {
        display: flex;
        justify-content: space-between;
        gap: 8px;
        align-items: center;
      }

      .ph-node-title {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 12px;
        font-weight: 950;
      }

      .ph-node-led {
        width: 9px;
        height: 9px;
        border-radius: 50%;
        background: var(--node-tone);
        box-shadow: 0 0 15px color-mix(in srgb, var(--node-tone) 70%, transparent);
        flex: 0 0 auto;
      }

      .ph-node-tags {
        display: flex;
        gap: 5px;
        flex-wrap: wrap;
      }

      .ph-node-meta {
        display: flex;
        justify-content: space-between;
        gap: 8px;
        color: var(--text-dim);
        font-size: 10px;
      }

      @media (prefers-reduced-motion: reduce) {
        .ph-core-visual::before,
        .ph-core-visual::after,
        .ph-live-bus::after,
        .ph-signal-rail::after,
        .ph-product-node.is-selected,
        .ph-orbit-node.is-selected,
        .ph-cortex-core::before,
        .ph-cortex-core::after {
          animation: none;
        }
      }
    `}</style>
  );
}

function commandSurfaceStyle(tone = "var(--border)", extra = {}) {
  return {
    ...S.panel,
    position: "relative",
    overflow: "hidden",
    borderRadius: 8,
    border: `1px solid color-mix(in srgb, ${tone} 24%, var(--border))`,
    background:
      "linear-gradient(180deg, color-mix(in srgb, var(--bg-panel) 94%, #0a1215) 0%, color-mix(in srgb, var(--bg-panel) 99%, #03070a) 100%)",
    boxShadow: "inset 0 1px 0 rgba(255,255,255,0.03)",
    ...extra,
  };
}

function commandInsetStyle(tone = "var(--border)", extra = {}) {
  return {
    display: "grid",
    gap: 8,
    padding: 10,
    borderRadius: 6,
    border: `1px solid color-mix(in srgb, ${tone} 18%, var(--border))`,
    background: "color-mix(in srgb, var(--bg) 58%, transparent)",
    boxSizing: "border-box",
    ...extra,
  };
}

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

function laneTone(lane) {
  if (lane === "Control Plane") return "var(--blue)";
  if (lane === "Action") return "var(--accent)";
  if (lane === "Trust" || lane === "Quality") return "var(--green)";
  if (lane === "Review" || lane === "Merge" || lane === "Security") return "var(--gold)";
  if (lane === "Memory" || lane === "Dependencies" || lane === "CI") return "var(--blue)";
  return "var(--accent)";
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

function productNeedsAttention(product) {
  const runtime = product?.runtime;
  const launcher = product?.setupItem?.launcher;
  if (!runtime) return false;
  if (runtime.status === "offline" || runtime.status === "degraded") return true;
  if (runtime.slug !== "hive-core" && !runtime.service_token_configured) return true;
  if ((launcher?.blockers || []).length > 0 || (launcher?.start_blockers || []).length > 0) return true;
  return Boolean(launcher && !productLooksRunning(product) && !launcher.start_ready);
}

function productMatchesFleetFilter(product, filter) {
  const launcher = product?.setupItem?.launcher;
  if (filter === "attention") return productNeedsAttention(product);
  if (filter === "running") return productLooksRunning(product);
  if (filter === "launchable") return Boolean(launcher?.start_ready && !productLooksRunning(product));
  if (filter === "gated") return Boolean(launcher && !productLooksRunning(product) && !launcher.start_ready);
  if (filter === "first-stack") return FIRST_STACK_SLUGS.includes(product?.runtime?.slug);
  return true;
}

function fleetFilterCount(products, filter) {
  return (products || []).filter((product) => productMatchesFleetFilter(product, filter)).length;
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
    <div style={commandInsetStyle(tone, { gap: 9, minHeight: 94 })}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "baseline" }}>
        <span style={commandKickerStyle}>{label}</span>
        <strong style={{ fontSize: 11, color: "var(--text-dim)" }}>{total} total</strong>
      </div>
      <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
        <span style={{ fontSize: 26, lineHeight: 1, fontWeight: 950, letterSpacing: 0, color: tone }}>{value}</span>
        <span style={{ fontSize: 11, color: "var(--text-dim)" }}>of {total}</span>
      </div>
      <div style={{ height: 8, borderRadius: 999, background: "rgba(255,255,255,0.06)", overflow: "hidden" }}>
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
        gap: 7,
        textAlign: "left",
        padding: "12px 13px",
        borderRadius: 8,
        border: `1px solid color-mix(in srgb, ${color} 32%, var(--border))`,
        background: active
          ? `linear-gradient(180deg, color-mix(in srgb, ${color} 16%, var(--bg-panel)) 0%, color-mix(in srgb, var(--bg) 48%, transparent) 100%)`
          : "linear-gradient(180deg, color-mix(in srgb, var(--bg) 18%, var(--bg-panel)) 0%, color-mix(in srgb, var(--bg) 58%, transparent) 100%)",
        color: "var(--text)",
        cursor: disabled ? "not-allowed" : "pointer",
        opacity: disabled ? 0.62 : 1,
        boxShadow: active ? `0 0 0 1px color-mix(in srgb, ${color} 22%, transparent) inset` : "none",
      }}
    >
      <span style={{ ...commandKickerStyle, color }}>Command</span>
      <span style={{ fontSize: 13, fontWeight: 900, color }}>{title}</span>
      <span style={{ fontSize: 10, color: "var(--text-dim)", lineHeight: 1.5 }}>{detail}</span>
    </button>
  );
}

function CommandRail({ setup, mission, onlineCount, pairedCount, fleetCount, fleetPlan, busyAction, onRefresh, onAction }) {
  const busy = Boolean(busyAction) || fleetLaunchActive(setup.latest_fleet_launch);
  const smoke = setup.latest_smoke;
  return (
    <div style={commandRailStyle}>
      <div style={commandSurfaceStyle(mission.tone, { display: "grid", gap: 12, paddingRight: 88, minHeight: 154 })}>
        <div className="ph-core-visual" aria-hidden="true">
          <div className="ph-core-mark">⬢</div>
        </div>
        <div style={{ display: "grid", gap: 7 }}>
          <div style={commandKickerStyle}>HiveCore command</div>
          <div style={{ fontSize: 24, lineHeight: 1.05, fontWeight: 950, letterSpacing: 0 }}>{mission.label}</div>
          <div style={commandBodyStyle}>{mission.headline}</div>
          {mission.detail && <div style={{ ...commandBodyStyle, color: mission.tone }}>{mission.detail}</div>}
        </div>
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
          <Tag color={setup.launcher.available ? "var(--green)" : "var(--accent)"}>
            launcher {setup.launcher.available ? "online" : "offline"}
          </Tag>
          <Tag color={setup.suite_bootstrap_configured ? "var(--blue)" : "var(--gold)"}>
            bootstrap {setup.suite_bootstrap_configured ? "ready" : "missing"}
          </Tag>
          {smoke && <Tag color={smokeTone(smoke.status)}>smoke {smoke.status}</Tag>}
        </div>
      </div>

      <div style={commandSurfaceStyle("var(--accent)", { display: "grid", gap: 10 })}>
        <div style={commandKickerStyle}>Suite counters</div>
        <div style={{ display: "grid", gap: 8 }}>
          <LaunchGauge label="Reachable" value={onlineCount} total={FIRST_STACK_SLUGS.length} tone={onlineCount === FIRST_STACK_SLUGS.length ? "var(--green)" : "var(--gold)"} />
          <LaunchGauge label="Paired" value={pairedCount} total={FIRST_STACK_SLUGS.length} tone={pairedCount === FIRST_STACK_SLUGS.length ? "var(--green)" : "var(--blue)"} />
          <LaunchGauge label="Fleet" value={fleetCount} total={11} tone="var(--accent)" />
        </div>
        <div style={commandBodyStyle}>{setup.launcher.repo_root ? `Root: ${setup.launcher.repo_root}` : setup.launcher.message}</div>
      </div>

      <div style={commandSurfaceStyle("var(--blue)", { display: "grid", gap: 8 })}>
        <div style={commandKickerStyle}>Commands</div>
        <CommandButton
          title="Refresh telemetry"
          detail="Reload health, launcher, pairing, and fleet data."
          color="var(--text-dim)"
          disabled={busy}
          onClick={onRefresh}
        />
        <CommandButton
          title="Start first stack"
          detail="Bring up SignalHive, TrustGate, and RepoReaper."
          color="var(--green)"
          active={onlineCount < FIRST_STACK_SLUGS.length}
          disabled={busy || busyAction === "Start first stack"}
          onClick={() => onAction("Start first stack", "/setup/first-stack/start")}
        />
        <CommandButton
          title="Start ready fleet"
          detail={`${fleetPlan.launchable} products can launch now.`}
          color="var(--green)"
          active={fleetPlan.launchable > 0}
          disabled={busy || busyAction === "Start ready fleet" || fleetPlan.launchable === 0}
          onClick={() => onAction("Start ready fleet", "/setup/fleet/start-ready")}
        />
        <CommandButton
          title="Start all 11"
          detail={fleetPlan.gated ? `${fleetPlan.gated} products are still gated.` : "Launch every stopped managed product."}
          color="var(--blue)"
          active={fleetPlan.canStartAll}
          disabled={busy || busyAction === "Start all 11" || !fleetPlan.canStartAll}
          onClick={() => onAction("Start all 11", "/setup/fleet/start-all")}
        />
        <CommandButton
          title="Pair running products"
          detail="Provision scoped service tokens for HiveCore machine control."
          color="var(--blue)"
          active={pairedCount < FIRST_STACK_SLUGS.length}
          disabled={busy || busyAction === "Pair running products"}
          onClick={() => onAction("Pair running products", "/setup/first-stack/pair")}
        />
        <CommandButton
          title="Smoke first stack"
          detail="Run auth, health, capability, and safe-action checks."
          color="var(--accent)"
          active={pairedCount === FIRST_STACK_SLUGS.length && smoke?.status !== "ready"}
          disabled={busy || busyAction === "First-stack smoke"}
          onClick={() => onAction("First-stack smoke", "/setup/smoke/first-stack")}
        />
        <CommandButton
          title="Read-only fleet smoke"
          detail="Verify every non-write product without dispatching actions."
          color="var(--blue)"
          disabled={busy || busyAction === "Read-only smoke"}
          onClick={() => onAction("Read-only smoke", "/setup/smoke/read-only-fleet")}
        />
        <CommandButton
          title="Dry-run smoke"
          detail="Exercise RepoReaper without opening PRs."
          color="var(--gold)"
          disabled={busy || busyAction === "Dry-run smoke"}
          onClick={() => onAction("Dry-run smoke", "/setup/smoke/write-dry-run")}
        />
        <CommandButton
          title="Stop first stack"
          detail="Stop the starter trio without removing data."
          color="var(--gold)"
          disabled={busy || busyAction === "Stop first stack"}
          onClick={() => onAction("Stop first stack", "/setup/first-stack/stop")}
        />
      </div>
    </div>
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
    <div style={commandSurfaceStyle(plan.gated ? "var(--gold)" : "var(--accent)", { display: "grid", gap: 14, ...commandPanelScrollStyle })}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 10, flexWrap: "wrap", alignItems: "center" }}>
        <div>
          <div style={commandKickerStyle}>Launch matrix</div>
          <div style={commandTitleStyle}>Fleet launch posture</div>
          <div style={commandBodyStyle}>
            HiveCore stages launcher-managed products by preflight readiness. Start-ready products can move now; the rest stay visible with exact blockers.
          </div>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center", justifyContent: "flex-end" }}>
          <Tag color="var(--green)">{plan.ready} ready</Tag>
          <Tag color="var(--blue)">{plan.launchable} launchable</Tag>
          <Tag color="var(--accent)">{plan.running} running</Tag>
          <Tag color={plan.gated ? "var(--gold)" : "var(--green)"}>{plan.gated} gated</Tag>
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

      <div style={{ display: "grid", gap: 10, gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))" }}>
        <div style={commandInsetStyle("var(--green)")}>
          <div style={commandKickerStyle}>Start-ready</div>
          <div style={{ fontSize: 24, lineHeight: 1, fontWeight: 950, color: "var(--green)" }}>{plan.ready}</div>
          <div style={commandBodyStyle}>Products that already satisfy compose, env, image, and credential preflight.</div>
        </div>
        <div style={commandInsetStyle("var(--accent)")}>
          <div style={commandKickerStyle}>Running now</div>
          <div style={{ fontSize: 24, lineHeight: 1, fontWeight: 950, color: "var(--accent)" }}>{plan.running}</div>
          <div style={commandBodyStyle}>Launcher-managed products that HiveCore currently sees as online or compose-running.</div>
        </div>
        <div style={commandInsetStyle(plan.gated ? "var(--gold)" : "var(--blue)")}>
          <div style={commandKickerStyle}>Blocked starts</div>
          <div style={{ fontSize: 24, lineHeight: 1, fontWeight: 950, color: plan.gated ? "var(--gold)" : "var(--blue)" }}>{plan.gated}</div>
          <div style={commandBodyStyle}>Stopped products that still need credentials, images, or local compose fixes before launch.</div>
        </div>
      </div>

      {launchJob && (
        <div style={commandInsetStyle(fleetLaunchTone(launchJob.status), { gap: 6 })}>
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

      <div style={{ display: "grid", gap: 10, gridTemplateColumns: "repeat(auto-fit, minmax(230px, 1fr))" }}>
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
                gap: 8,
                padding: 12,
                borderRadius: 8,
                border: `1px solid color-mix(in srgb, ${native ? "var(--blue)" : preflightTone(launcher?.preflight_status)} 30%, var(--border))`,
                background: "linear-gradient(180deg, color-mix(in srgb, var(--bg) 20%, var(--bg-panel)) 0%, color-mix(in srgb, var(--bg) 58%, transparent) 100%)",
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center" }}>
                <span style={{ fontSize: 12, fontWeight: 900, letterSpacing: 0 }}>
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
                    env {native ? "active" : launcher?.env_exists ? "ready" : "template"}
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

function SmokeEvidence({ smoke }) {
  if (!smoke) {
    return (
      <div style={commandSurfaceStyle("var(--text-dim)", { display: "grid", gap: 12, ...commandPanelScrollStyle })}>
        <div>
          <div style={commandKickerStyle}>Latest suite smoke</div>
          <div style={commandTitleStyle}>No smoke evidence yet</div>
          <div style={commandBodyStyle}>Run a HiveCore smoke tier to capture reachability, auth, capability, and safe-action proof.</div>
        </div>
      </div>
    );
  }
  const grouped = smoke.steps.reduce((acc, step) => {
    acc[step.slug] = acc[step.slug] || [];
    acc[step.slug].push(step);
    return acc;
  }, {});

  return (
    <div style={commandSurfaceStyle(smokeTone(smoke.status), { display: "grid", gap: 12, ...commandPanelScrollStyle })}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 10, flexWrap: "wrap", alignItems: "center" }}>
        <div>
          <div style={commandKickerStyle}>Latest suite smoke</div>
          <div style={commandTitleStyle}>{smoke.summary}</div>
          <div style={commandBodyStyle}>
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
              border: `1px solid color-mix(in srgb, ${smokeTone(steps.some((step) => step.status === "fail") ? "fail" : steps.some((step) => step.status === "warn") ? "warn" : "pass")} 18%, var(--border))`,
              borderRadius: 8,
              background: "linear-gradient(180deg, color-mix(in srgb, var(--bg) 24%, var(--bg-panel)) 0%, color-mix(in srgb, var(--bg) 58%, transparent) 100%)",
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
                  background: "color-mix(in srgb, var(--bg) 60%, transparent)",
                  border: `1px solid color-mix(in srgb, ${smokeTone(step.status)} 26%, var(--border))`,
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
  if (!job) {
    return (
      <div style={commandSurfaceStyle("var(--text-dim)", { display: "grid", gap: 12, ...commandPanelScrollStyle })}>
        <div>
          <div style={commandKickerStyle}>Fleet launch job</div>
          <div style={commandTitleStyle}>No orchestration run yet</div>
          <div style={commandBodyStyle}>HiveCore will record per-product launch progress here the first time it runs a managed fleet launch.</div>
        </div>
      </div>
    );
  }
  return (
    <div style={commandSurfaceStyle(fleetLaunchTone(job.status), { display: "grid", gap: 12, ...commandPanelScrollStyle })}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 10, flexWrap: "wrap", alignItems: "center" }}>
        <div>
          <div style={commandKickerStyle}>Fleet launch job</div>
          <div style={commandTitleStyle}>{job.summary}</div>
          <div style={commandBodyStyle}>
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
              border: `1px solid color-mix(in srgb, ${fleetLaunchTone(step.status)} 26%, var(--border))`,
              borderRadius: 8,
              background: "linear-gradient(180deg, color-mix(in srgb, var(--bg) 24%, var(--bg-panel)) 0%, color-mix(in srgb, var(--bg) 58%, transparent) 100%)",
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
    <div style={commandSurfaceStyle(smoke ? smokeTone(smoke.status) : "var(--blue)", { display: "grid", gap: 12, ...commandPanelScrollStyle })}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 10, flexWrap: "wrap", alignItems: "center" }}>
        <div>
          <div style={commandKickerStyle}>Evidence timeline</div>
          <div style={commandTitleStyle}>Operational proof trail</div>
          <div style={commandBodyStyle}>The shortest path through HiveCore’s current launch, pairing, and smoke evidence.</div>
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
              borderRadius: 8,
              border: `1px solid color-mix(in srgb, ${item.tone} 24%, var(--border))`,
              background: "color-mix(in srgb, var(--bg) 56%, transparent)",
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

function FleetLaneMap({ products, selectedSlug, filter, onFilterChange, onSelect, mission, onlineCount, pairedCount, fleetPlan }) {
  const filteredProducts = products.filter((product) => productMatchesFleetFilter(product, filter));
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
  const productCount = Math.max(products.length, 1);

  return (
    <div className="ph-cortex-shell">
      <div className="ph-cortex-topbar">
        <div className="ph-cortex-title">
          <div style={commandKickerStyle}>HiveCore cortex</div>
          <div style={{ ...commandTitleStyle, fontSize: 22 }}>Suite control room</div>
          <div style={commandBodyStyle}>This is the live nervous system: products orbit the HiveCore core, filtered nodes dim, and the selected node drives the inspector.</div>
        </div>
        <div style={{ display: "grid", gap: 8, justifyItems: "end" }}>
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap", justifyContent: "flex-end" }}>
            <Tag color="var(--green)">{products.filter(productLooksRunning).length} active</Tag>
            <Tag color="var(--gold)">{products.filter(productNeedsAttention).length} attention</Tag>
            <Tag color="var(--blue)">{filteredProducts.length} visible</Tag>
          </div>
          <div className="ph-filter-row" style={{ justifyContent: "flex-end" }}>
            {FLEET_FILTERS.map((item) => {
              const active = item.id === filter;
              const count = fleetFilterCount(products, item.id);
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => onFilterChange(item.id)}
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 6,
                    minHeight: 28,
                    padding: "5px 8px",
                    borderRadius: 7,
                    border: `1px solid ${active ? "var(--accent)" : "var(--border)"}`,
                    background: active ? "color-mix(in srgb, var(--accent) 16%, var(--bg-panel))" : "color-mix(in srgb, var(--bg) 56%, transparent)",
                    color: active ? "var(--accent)" : "var(--text-dim)",
                    cursor: "pointer",
                    fontFamily: "inherit",
                    fontSize: 10,
                    fontWeight: 850,
                  }}
                >
                  <span>{item.label}</span>
                  <span style={{ color: active ? "var(--text)" : "var(--text-dim)" }}>{count}</span>
                </button>
              );
            })}
          </div>
        </div>
      </div>

      <div className="ph-cortex-stage">
        <div className="ph-orbit-ring one" aria-hidden="true" />
        <div className="ph-orbit-ring two" aria-hidden="true" />
        <div className="ph-orbit-ring three" aria-hidden="true" />
        <div className="ph-cortex-core">
          <div className="ph-cortex-glyph">⬢</div>
          <div className="ph-cortex-mission">{mission.label}</div>
          <div className="ph-cortex-subcopy">{mission.headline}</div>
          <div className="ph-cortex-counters">
            <div className="ph-cortex-counter">
              <strong style={{ color: onlineCount === FIRST_STACK_SLUGS.length ? "var(--green)" : "var(--gold)" }}>{onlineCount}/{FIRST_STACK_SLUGS.length}</strong>
              <span>Reachable</span>
            </div>
            <div className="ph-cortex-counter">
              <strong style={{ color: pairedCount === FIRST_STACK_SLUGS.length ? "var(--green)" : "var(--blue)" }}>{pairedCount}/{FIRST_STACK_SLUGS.length}</strong>
              <span>Paired</span>
            </div>
            <div className="ph-cortex-counter">
              <strong style={{ color: fleetPlan.gated ? "var(--gold)" : "var(--accent)" }}>{fleetPlan.running}/{fleetPlan.controlled.length}</strong>
              <span>Running</span>
            </div>
          </div>
        </div>
        {products.map((product, index) => {
          const runtime = product.runtime;
          const angle = -90 + (index * 360) / productCount;
          const visible = productMatchesFleetFilter(product, filter);
          const nodeTone = statusColor(runtime.status);
          const linkLength = product.firstStack ? "27%" : "38%";
          return (
            <div key={`${runtime.slug}-link`} className="ph-orbit-link" style={{ "--link-angle": `${angle}deg`, "--link-length": linkLength, "--node-tone": nodeTone, "--link-opacity": visible ? 0.58 : 0.1 }} />
          );
        })}
        {products.map((product, index) => {
          const runtime = product.runtime;
          const angle = -90 + (index * 360) / productCount;
          const radians = (angle * Math.PI) / 180;
          const x = 50 + Math.cos(radians) * 41;
          const y = 50 + Math.sin(radians) * 38;
          const selected = runtime.slug === selectedSlug;
          const managed = Boolean(product.setupItem?.launcher);
          const visible = productMatchesFleetFilter(product, filter);
          const attention = productNeedsAttention(product);
          const nodeTone = statusColor(runtime.status);
          return (
            <button
              type="button"
              className={`ph-orbit-node${selected ? " is-selected" : ""}${visible ? "" : " is-muted"}${attention ? " is-attention" : ""}`}
              key={runtime.slug}
              onClick={() => onSelect(runtime.slug)}
              style={{ "--node-tone": nodeTone, left: `${x}%`, top: `${y}%` }}
            >
              <div className="ph-node-top">
                <span className="ph-node-title">
                  {runtime.icon} {runtime.title}
                </span>
                <span className="ph-node-led" aria-label={runtime.status} />
              </div>
              <div className="ph-node-tags">
                <Tag color={statusColor(runtime.status)}>{runtime.status}</Tag>
                {product.firstStack && <Tag color="var(--accent)">core</Tag>}
                {managed && <Tag color="var(--green)">launcher</Tag>}
              </div>
              <div className="ph-node-meta">
                <span>{runtime.lane}</span>
                <span>{runtime.service_token_configured ? "paired" : runtime.slug === "hive-core" ? "native" : "unpaired"}</span>
              </div>
            </button>
          );
        })}
      </div>

      <div className="ph-lane-strip">
        {lanes.map((lane) => {
          const tone = laneTone(lane);
          return (
            <div className="ph-lane-chip" key={lane} style={{ "--lane-tone": tone }}>
              <strong>{lane}</strong>
              <span>{grouped[lane].length}</span>
            </div>
          );
        })}
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
      style={commandSurfaceStyle(statusColor(item.runtime.status), { display: "grid", gap: 12 })}
    >
      <div style={{ display: "flex", justifyContent: "space-between", gap: 10, alignItems: "flex-start" }}>
        <div style={{ display: "grid", gap: 4 }}>
          <div style={commandKickerStyle}>{item.runtime.lane}</div>
          <div style={{ fontSize: 18, fontWeight: 900, letterSpacing: 0 }}>
            {item.runtime.icon} {item.runtime.title}
          </div>
          <div style={commandBodyStyle}>{item.runtime.role}</div>
        </div>
        <Tag color={statusColor(item.runtime.status)}>{item.runtime.status}</Tag>
      </div>

      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        <Tag color={authTone(item)}>{authLabel(item)}</Tag>
        {authStatus?.suite_bootstrap_enabled && <Tag color="var(--blue)">suite bootstrap ready</Tag>}
        {item.pairing_ready && <Tag color="var(--green)">ready to pair</Tag>}
        {launcher && <Tag color={launcherTone(launcher.status)}>launcher {launcher.status}</Tag>}
      </div>

      <div style={commandInsetStyle(statusColor(item.runtime.status), { fontSize: 11 })}>
        <div style={commandKickerStyle}>Runtime telemetry</div>
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
        <div style={commandInsetStyle(preflightTone(launcher.preflight_status), { fontSize: 11 })}>
          <div style={commandKickerStyle}>Launcher telemetry</div>
          <div style={{ display: "flex", gap: 7, flexWrap: "wrap" }}>
            <Tag color={launcher.compose_exists ? "var(--green)" : "var(--accent)"}>
              compose {launcher.compose_exists ? "found" : "missing"}
            </Tag>
            <Tag color={launcher.env_exists ? "var(--green)" : "var(--gold)"}>
              env {launcher.env_exists ? "ready" : "template"}
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
        <div style={commandInsetStyle("var(--blue)", { gap: 9 })}>
          <div style={{ display: "flex", justifyContent: "space-between", gap: 10, flexWrap: "wrap", alignItems: "center" }}>
            <div>
              <div style={commandKickerStyle}>First-run setup</div>
              <div style={{ ...commandBodyStyle, fontSize: 10 }}>Saved through HiveCore and written locally by patchhive-launcher.</div>
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
              <div style={{ ...commandBodyStyle, fontSize: 10 }}>
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

      <div style={commandInsetStyle("var(--text-dim)", { gap: 8 })}>
        <div style={commandKickerStyle}>Quick links</div>
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
      </div>

      {item.runtime.slug !== "hive-core" && item.launcher && (
        <div style={commandInsetStyle("var(--green)", { gap: 8 })}>
          <div style={commandKickerStyle}>Product controls</div>
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
        </div>
      )}
    </div>
  );
}

export default function SetupPanel({ fetchEnvelope, setRunning, setError }) {
  const [setup, setSetup] = useState(null);
  const [fleet, setFleet] = useState([]);
  const [selectedSlug, setSelectedSlug] = useState("hive-core");
  const [fleetFilter, setFleetFilter] = useState("all");
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
  const mission = setupMission(setup, onlineCount, pairedCount);

  return (
    <>
    <CommandCenterStyles />
    <div style={{ display: "grid", gap: 12, paddingBottom: 22 }}>
      <div className="ph-command-console" style={commandConsoleStyle}>
        <CommandRail
          setup={setup}
          mission={mission}
          onlineCount={onlineCount}
          pairedCount={pairedCount}
          fleetCount={fleetProducts.length}
          fleetPlan={fleetPlan}
          busyAction={busyAction}
          onRefresh={refresh}
          onAction={runAction}
        />

        <div style={commandMainColumnStyle}>
          <FleetLaneMap
            products={fleetProducts}
            selectedSlug={selectedProduct?.runtime.slug}
            filter={fleetFilter}
            onFilterChange={setFleetFilter}
            onSelect={setSelectedSlug}
            mission={mission}
            onlineCount={onlineCount}
            pairedCount={pairedCount}
            fleetPlan={fleetPlan}
          />
          <div style={commandTopGridStyle}>
            <FleetStartPlan
              products={fleetProducts}
              busyAction={busyAction}
              launchJob={setup.latest_fleet_launch}
              onAction={runAction}
            />
            <FleetLaunchEvidence job={setup.latest_fleet_launch} />
          </div>
          <div style={commandEvidenceGridStyle}>
            <EvidenceTimeline setup={setup} />
            <div style={commandEvidenceStackStyle}>
              <SmokeEvidence smoke={setup.latest_smoke} />
            </div>
          </div>
        </div>

        <div style={focusRailStyle}>
          <div style={focusPanelStyle}>
            <div>
              <div style={commandKickerStyle}>Selected inspector</div>
              <div style={commandTitleStyle}>Focused product</div>
              <div style={commandBodyStyle}>Select any product from the fleet map. HiveCore only shows launcher and env controls when that product is managed by this setup flow.</div>
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
              borderRadius: 8,
              border: "1px solid var(--border)",
              background: "var(--bg)",
            }}
          >
            {logs.logs || "No recent logs returned."}
          </pre>
        </div>
      )}

    </div>
    </>
  );
}
