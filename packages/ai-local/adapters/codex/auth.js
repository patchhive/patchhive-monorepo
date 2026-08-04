import { execFile } from "node:child_process";

const DEFAULT_AUTH_PROBE_TIMEOUT_MS = 5_000;
const MAX_AUTH_PROBE_OUTPUT_BYTES = 16_384;
const CODEX_ENV_KEYS = [
  "HOME",
  "PATH",
  "CODEX_HOME",
  "XDG_CONFIG_HOME",
  "XDG_CACHE_HOME",
  "XDG_RUNTIME_DIR",
  "DBUS_SESSION_BUS_ADDRESS",
  "USERPROFILE",
  "LOCALAPPDATA",
  "APPDATA",
  "SYSTEMROOT",
  "TMPDIR",
  "TEMP",
  "TMP",
  "LANG",
  "LC_ALL",
  "SSL_CERT_FILE",
  "SSL_CERT_DIR",
  "HTTP_PROXY",
  "HTTPS_PROXY",
  "NO_PROXY",
  "ALL_PROXY",
  "http_proxy",
  "https_proxy",
  "no_proxy",
  "all_proxy",
];

function envString(name, env = process.env) {
  const value = env[name];
  if (value == null) return null;
  const trimmed = String(value).trim();
  return trimmed ? trimmed : null;
}

function envBool(name, fallback, env = process.env) {
  const value = envString(name, env);
  if (value == null) return fallback;
  if (["1", "true", "yes", "on"].includes(value.toLowerCase())) return true;
  if (["0", "false", "no", "off"].includes(value.toLowerCase())) return false;
  return fallback;
}

function authMode(output) {
  const normalized = output.toLowerCase();
  if (normalized.includes("chatgpt")) return "chatgpt_subscription";
  if (normalized.includes("api key")) return "api_key";
  if (normalized.includes("access token")) return "access_token";
  return "unknown";
}

export function classifyCodexAuthResult({ error = null, stdout = "", stderr = "" } = {}) {
  const output = `${stdout}\n${stderr}`.trim();
  const normalized = output.toLowerCase();

  if (!error) {
    return {
      status: "authenticated",
      mode: authMode(output),
      managed_by: "codex",
    };
  }

  if (normalized.includes("not logged in") || normalized.includes("not authenticated")) {
    return {
      status: "not_authenticated",
      mode: null,
      managed_by: "codex",
      reason: "login_required",
    };
  }

  const reason = error?.code === "ENOENT"
    ? "cli_unavailable"
    : error?.killed || error?.code === "ETIMEDOUT"
      ? "probe_timeout"
      : "probe_failed";

  return {
    status: "failed",
    mode: null,
    managed_by: "codex",
    reason,
  };
}

export function codexBootstrapHint(env = process.env) {
  const command = envString("PATCHHIVE_AI_CODEX_CLI_PATH", env) || "codex";
  return `Run \`${command} login\` for browser sign-in or \`${command} login --device-auth\` for a headless host.`;
}

export function codexProcessEnv(env = process.env) {
  return Object.fromEntries(
    CODEX_ENV_KEYS
      .filter(name => env[name] != null && String(env[name]).length > 0)
      .map(name => [name, String(env[name])]),
  );
}

export function codexClientOptions(env = process.env) {
  const cliPath = envString("PATCHHIVE_AI_CODEX_CLI_PATH", env);
  return {
    env: codexProcessEnv(env),
    ...(cliPath ? { codexPathOverride: cliPath } : {}),
  };
}

export async function probeCodexAuth({ env = process.env, run = execFile } = {}) {
  if (!envBool("PATCHHIVE_AI_CODEX_AUTH_PROBE", true, env)) {
    return {
      status: "not_observed",
      mode: null,
      managed_by: "codex",
      reason: "probe_disabled",
    };
  }

  const command = envString("PATCHHIVE_AI_CODEX_CLI_PATH", env) || "codex";

  return new Promise(resolve => {
    run(
      command,
      ["login", "status"],
      {
        encoding: "utf8",
        env: codexProcessEnv(env),
        maxBuffer: MAX_AUTH_PROBE_OUTPUT_BYTES,
        timeout: DEFAULT_AUTH_PROBE_TIMEOUT_MS,
        windowsHide: true,
      },
      (error, stdout, stderr) => {
        resolve(classifyCodexAuthResult({ error, stdout, stderr }));
      },
    );
  });
}

export function legacyLoggedIn(auth) {
  if (auth.status === "authenticated") return true;
  if (auth.status === "not_authenticated") return false;
  return null;
}
