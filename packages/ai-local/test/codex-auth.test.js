import assert from "node:assert/strict";
import test from "node:test";

import {
  classifyCodexAuthResult,
  codexClientOptions,
  legacyLoggedIn,
  probeCodexAuth,
} from "../adapters/codex/auth.js";

test("passes only Codex runtime context to the SDK process", () => {
  const options = codexClientOptions({
    HOME: "/safe/home",
    PATH: "/safe/bin",
    PATCHHIVE_AI_CODEX_CLI_PATH: "/safe/bin/codex",
    REPO_REAPER_GITHUB_TOKEN_RW: "must-not-leak",
    PATCHHIVE_AI_GATEWAY_API_KEY: "must-not-leak",
  });

  assert.deepEqual(options, {
    codexPathOverride: "/safe/bin/codex",
    env: {
      HOME: "/safe/home",
      PATH: "/safe/bin",
    },
  });
});

test("classifies ChatGPT subscription login without exposing command output", () => {
  const auth = classifyCodexAuthResult({ stdout: "Logged in using ChatGPT\n" });

  assert.deepEqual(auth, {
    status: "authenticated",
    mode: "chatgpt_subscription",
    managed_by: "codex",
  });
  assert.equal(legacyLoggedIn(auth), true);
});

test("keeps API-key and access-token login modes distinct", () => {
  assert.equal(
    classifyCodexAuthResult({ stdout: "Logged in using an API key" }).mode,
    "api_key",
  );
  assert.equal(
    classifyCodexAuthResult({ stdout: "Logged in using an access token" }).mode,
    "access_token",
  );
});

test("distinguishes missing login from an auth probe failure", () => {
  const missing = classifyCodexAuthResult({
    error: Object.assign(new Error("exit 1"), { code: 1 }),
    stderr: "Not logged in",
  });
  const failed = classifyCodexAuthResult({
    error: Object.assign(new Error("spawn codex ENOENT"), { code: "ENOENT" }),
  });

  assert.equal(missing.status, "not_authenticated");
  assert.equal(legacyLoggedIn(missing), false);
  assert.equal(failed.status, "failed");
  assert.equal(failed.reason, "cli_unavailable");
  assert.equal(legacyLoggedIn(failed), null);
});

test("reports a disabled probe as not observed", async () => {
  let called = false;
  const auth = await probeCodexAuth({
    env: { PATCHHIVE_AI_CODEX_AUTH_PROBE: "false" },
    run: () => {
      called = true;
    },
  });

  assert.equal(called, false);
  assert.deepEqual(auth, {
    status: "not_observed",
    mode: null,
    managed_by: "codex",
    reason: "probe_disabled",
  });
});
