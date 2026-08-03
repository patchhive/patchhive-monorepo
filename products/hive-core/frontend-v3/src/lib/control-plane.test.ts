import { beforeEach, describe, expect, it, vi } from "vitest";

import { apiFetch } from "./http";
import {
  fetchDiagnostics,
  saveRepositoryPolicies,
  saveSettings,
  type RepositoryPoliciesResponse,
  type SettingsResponse,
} from "./control-plane";
import {
  fetchProductLogs,
  runProductLifecycle,
  saveProductEnv,
  stopFirstStack,
  validateGitHubToken,
} from "./bootstrap";

vi.mock("./http", () => ({ apiFetch: vi.fn() }));

const mockedFetch = vi.mocked(apiFetch);

function response(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

beforeEach(() => mockedFetch.mockReset());

describe("control-plane writes", () => {
  it("sends every v1 suite and product override field through the v3 settings contract", async () => {
    const settings = {
      product: "hive-core",
      tagline: "control",
      suite_settings: {
        operator_label: "operator",
        mission: "mission",
        default_topics: "maintenance",
        default_languages: "rust",
        repo_allowlist: "owner/allowed",
        repo_denylist: "owner/denied",
        opt_out_notes: "respect opt-outs",
        preferred_launch_product: "signal-hive",
        notes: "notes",
        updated_at: "2026-08-03T00:00:00Z",
      },
      products: [
        {
          slug: "signal-hive",
          title: "SignalHive",
          icon: "",
          lane: "signal",
          role: "discover",
          repo: "products/signal-hive",
          default_frontend_url: "http://localhost:5174",
          default_api_url: "http://localhost:8010",
          override_frontend_url: "https://signal.example",
          override_api_url: "https://api.signal.example",
          auth_mode: "service_token",
          machine_auth_configured: true,
          service_token_configured: true,
          legacy_api_key_configured: false,
          service_token: "write-only-replacement",
          enabled: false,
          notes: "product notes",
          updated_at: "2026-08-03T00:00:00Z",
        },
      ],
    } satisfies SettingsResponse;
    mockedFetch.mockResolvedValue(response({ data: settings }));

    await saveSettings(settings);

    expect(mockedFetch).toHaveBeenCalledWith(
      "/api/products/hive-core/settings",
      expect.objectContaining({ method: "PUT" }),
    );
    const init = mockedFetch.mock.calls[0][1] as RequestInit;
    const payload = JSON.parse(String(init.body));
    expect(payload.suite_settings).toMatchObject({
      operator_label: "operator",
      preferred_launch_product: "signal-hive",
      repo_allowlist: "owner/allowed",
      repo_denylist: "owner/denied",
    });
    expect(payload.products[0]).toEqual({
      slug: "signal-hive",
      frontend_url: "https://signal.example",
      api_url: "https://api.signal.example",
      service_token: "write-only-replacement",
      enabled: false,
      notes: "product notes",
    });
  });

  it("never sends the server-owned public opt-out flag as operator authority", async () => {
    const policies = {
      policies: [
        {
          repository: "owner/quiet",
          trusted: false,
          operator_excluded: false,
          allowlisted: false,
          public_opt_out: true,
          source: "patchhive.dev",
          notes: "owner opted out",
          updated_at: "2026-08-03T00:00:00Z",
        },
      ],
      public_opt_out_sync: { state: "observed", value: {} },
    } satisfies RepositoryPoliciesResponse;
    mockedFetch.mockResolvedValue(response({ data: policies }));

    await saveRepositoryPolicies(policies);

    const init = mockedFetch.mock.calls[0][1] as RequestInit;
    const payload = JSON.parse(String(init.body));
    expect(payload.policies[0]).not.toHaveProperty("public_opt_out");
    expect(payload.policies[0]).toMatchObject({ repository: "owner/quiet" });
  });
});

describe("launcher and diagnostics wiring", () => {
  it("uses the real stop, lifecycle, and log routes", async () => {
    mockedFetch
      .mockResolvedValueOnce(response({ data: { stack_id: "first" } }))
      .mockResolvedValueOnce(response({ data: { stack_id: "first" } }))
      .mockResolvedValueOnce(response({ data: { slug: "signal-hive", title: "SignalHive", logs: "ready" } }));

    await stopFirstStack();
    await runProductLifecycle("signal-hive", "restart");
    await fetchProductLogs("signal-hive");

    expect(mockedFetch.mock.calls.map(([path]) => path)).toEqual([
      "/api/products/hive-core/setup/first-stack/stop",
      "/api/products/hive-core/setup/products/signal-hive/restart",
      "/api/products/hive-core/setup/products/signal-hive/logs?tail=160",
    ]);
  });

  it("sends the username expected by HiveCore's GitHub validation contract", async () => {
    mockedFetch.mockResolvedValue(response({ data: { ok: true, login: "patchhive", user_matches: true, message: "valid" } }));

    await validateGitHubToken("secret-token", "patchhive");

    const init = mockedFetch.mock.calls[0][1] as RequestInit;
    expect(JSON.parse(String(init.body))).toEqual({
      token: "secret-token",
      expected_user: "patchhive",
    });
  });

  it("recreates a product after writing credentials so the new environment is active", async () => {
    mockedFetch.mockResolvedValue(response({ data: { stack_id: "first" } }));

    await saveProductEnv("signal-hive", { BOT_GITHUB_TOKEN: "secret-token" });

    const init = mockedFetch.mock.calls[0][1] as RequestInit;
    expect(JSON.parse(String(init.body))).toEqual({
      values: { BOT_GITHUB_TOKEN: "secret-token" },
      restart: true,
    });
  });

  it("keeps suite, HiveCore, and startup-check evidence separate", async () => {
    mockedFetch
      .mockResolvedValueOnce(response({ service: "patchhive-backend", status: "ok" }))
      .mockResolvedValueOnce(response({ product: "HiveCore", status: "degraded", db_ok: true }))
      .mockResolvedValueOnce(response({ checks: [{ level: "warn", msg: "token missing" }] }));

    const diagnostics = await fetchDiagnostics();

    expect(diagnostics.suite.status).toBe("ok");
    expect(diagnostics.hiveCore.status).toBe("degraded");
    expect(diagnostics.startup).toEqual([{ level: "warn", msg: "token missing" }]);
  });
});
