import { useCallback, useEffect, useState } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Loader2,
  Play,
  Power,
  RotateCw,
  Rocket,
  ScrollText,
  ShieldCheck,
  Square,
  XCircle,
} from "lucide-react";
import { toast } from "sonner";
import {
  fetchBootstrap,
  fetchProductLogs,
  pairFirstStack,
  runSmoke,
  runProductLifecycle,
  saveProductEnv,
  SMOKE_TIERS,
  startAllFleet,
  startFirstStack,
  startReadyFleet,
  stopFirstStack,
  STATUS_TONE,
  validateGitHubToken,
  type BootstrapState,
  type CredentialRequirement,
  type SetupProduct,
  type ProductLogs,
} from "@/lib/bootstrap";

/**
 * Bringing the suite up from nothing.
 *
 * HiveCore has had the backend for this for a while — launcher status, per-product
 * credential requirements, pairing, fleet launch, smoke tiers — and the v3 deck never
 * surfaced any of it, so first-run setup meant editing files by hand.
 *
 * The panel is ordered by dependency, and each step reports what is actually known
 * rather than assuming the step before it succeeded. That matters most at the top: the
 * launcher owns the `.env` files, so when it is unreachable HiveCore cannot say what
 * any product requires. An empty credential list then means "could not ask", not
 * "nothing needed", and this renders the difference.
 *
 * The browser never writes a file or touches Docker. Secrets typed here go to HiveCore,
 * which forwards approved writes to the launcher; they are never stored in the browser
 * and never read back.
 */
export function BootstrapWizard({ syncVersion = 0 }: { syncVersion?: number }) {
  const [state, setState] = useState<BootstrapState | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState("");
  const [logs, setLogs] = useState<ProductLogs | null>(null);

  const load = useCallback((signal?: AbortSignal) => {
    fetchBootstrap(signal)
      .then((next) => {
        setState(next);
        setError("");
      })
      .catch((cause) => setError(cause instanceof Error ? cause.message : String(cause)));
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    load(controller.signal);
    return () => controller.abort();
  }, [load, syncVersion]);

  const fleetJob =
    state?.latest_fleet_launch.state === "observed"
      ? state.latest_fleet_launch.value
      : null;
  const fleetActive =
    fleetJob?.lifecycle.state === "queued" || fleetJob?.lifecycle.state === "running";
  const previousFleetJobs =
    state?.fleet_launch_history.state === "observed"
      ? state.fleet_launch_history.value.filter((job) => job.id !== fleetJob?.id).slice(0, 4)
      : [];

  useEffect(() => {
    if (!fleetActive) return;
    const timer = window.setInterval(() => load(), 2000);
    return () => window.clearInterval(timer);
  }, [fleetActive, load]);

  async function pair(): Promise<void> {
    setBusy("pair");
    const result = await pairFirstStack();
    setBusy("");
    if (result.data) {
      setState(result.data);
      toast.success("Pairing pass recorded", {
        description: "The refreshed setup state shows which products accepted service tokens.",
      });
    } else {
      toast.error("Pairing failed", { description: result.message });
    }
  }

  async function smoke(tier: string): Promise<void> {
    setBusy(tier);
    const result = await runSmoke(tier);
    setBusy("");
    if (result.data) {
      setState(result.data);
      const run = result.data.latest_smoke;
      toast.success(`Smoke: ${run?.status ?? "done"}`, { description: run?.summary });
    } else {
      toast.error("Smoke run failed", { description: result.message });
    }
  }

  async function launchFleet(mode: "ready" | "all"): Promise<void> {
    setBusy(`fleet-${mode}`);
    const result = mode === "ready" ? await startReadyFleet() : await startAllFleet();
    setBusy("");
    if (result.data) {
      setState(result.data);
      toast.success(mode === "ready" ? "Ready fleet queued" : "Full fleet plan recorded");
    } else {
      toast.error("Fleet launch failed", { description: result.message });
    }
  }

  async function firstStack(action: "start" | "stop"): Promise<void> {
    setBusy(`first-stack-${action}`);
    const result = action === "start" ? await startFirstStack() : await stopFirstStack();
    setBusy("");
    if (result.data) {
      setState(result.data);
      toast.success(`First stack ${action === "start" ? "started" : "stopped"}`);
    } else {
      toast.error(`First-stack ${action} failed`, { description: result.message });
    }
  }

  async function productAction(slug: string, action: "start" | "stop" | "restart"): Promise<void> {
    setBusy(`${slug}-${action}`);
    const result = await runProductLifecycle(slug, action);
    setBusy("");
    if (result.data) {
      setState(result.data);
      toast.success(`${slug} ${action} requested`);
    } else {
      toast.error(`${slug} ${action} failed`, { description: result.message });
    }
  }

  async function showLogs(slug: string): Promise<void> {
    setBusy(`${slug}-logs`);
    try {
      setLogs(await fetchProductLogs(slug));
    } catch (cause) {
      toast.error(`Could not read ${slug} logs`, {
        description: cause instanceof Error ? cause.message : String(cause),
      });
    } finally {
      setBusy("");
    }
  }

  if (error) {
    return (
      <Section>
        <p className="text-[11px] text-[var(--crit)]">{error}</p>
      </Section>
    );
  }
  if (!state) {
    return (
      <Section>
        <p className="font-display text-[11px] uppercase tracking-wider text-muted-foreground">
          Reading setup state…
        </p>
      </Section>
    );
  }

  const unpaired = state.products.filter((product) => product.pairing_ready);
  const bootstrapReady = state.suite_bootstrap_authority.state === "ready";
  const needsCredentials = state.products.filter((product) =>
    product.credentials.some((item) => item.required && !item.configured),
  );

  return (
    <Section>
      <div className="mb-5 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <Rocket className="h-4 w-4 text-[var(--honey)]" /> Suite Bootstrap
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Bring the suite up from nothing. HiveCore plans and validates; the launcher
            performs host-level work. The browser writes no files and controls no containers.
          </p>
        </div>
      </div>

      {/* 1. The launcher. Everything below depends on it, so its state is reported
             once here rather than as a series of downstream mysteries. */}
      <Step
        index={1}
        title="Launcher"
        status={state.launcher.available ? "ok" : "fail"}
        detail={
          state.launcher.available
            ? `Reachable. Docker ${state.launcher.docker_available ? "available" : "unavailable"}, compose ${
                state.launcher.docker_compose_available ? "available" : "unavailable"
              }.`
            : state.launcher.message
        }
      >
        {!state.launcher.available && (
          <p className="mt-2 rounded border border-[var(--warn)]/40 bg-[var(--warn)]/5 p-2 text-[11px] text-muted-foreground">
            Without the launcher, HiveCore cannot read or write product{" "}
            <code className="font-mono">.env</code> files, cannot start containers, and
            cannot tell you what any product still needs. Start it with{" "}
            <code className="font-mono text-foreground">
              cargo run --manifest-path services/patchhive-launcher/Cargo.toml
            </code>{" "}
            and reload. Steps below are shown as unknown, not as complete.
          </p>
        )}
      </Step>

      {/* 2. Credentials. Only answerable when the launcher answered. */}
      <Step
        index={2}
        title="Credentials"
        status={
          !state.requirements_known
            ? "unknown"
            : needsCredentials.length > 0
              ? "warn"
              : "ok"
        }
        detail={
          !state.requirements_known
            ? state.requirements_error ||
              "HiveCore could not ask the launcher what each product requires."
            : needsCredentials.length === 0
              ? "Every product reports its required credentials as configured."
              : `${needsCredentials.length} product(s) missing required credentials.`
        }
      >
        {state.requirements_known &&
          state.products.filter((product) => product.credentials.length > 0).map((product) => (
            <CredentialForm
              key={product.runtime.slug}
              product={product}
              onSaved={() => load()}
            />
          ))}
      </Step>

      {/* 3. Pairing: minting scoped service tokens for products that are up. */}
      <Step
        index={3}
        title="Service tokens"
        status={bootstrapAuthorityStatus(state.suite_bootstrap_authority, unpaired.length)}
        detail={
          bootstrapReady
            ? unpaired.length > 0
              ? `${unpaired.length} product(s) reachable but unpaired. ${bootstrapAuthorityDetail(state.suite_bootstrap_authority)}`
              : `Every reachable product has a scoped service token. ${bootstrapAuthorityDetail(state.suite_bootstrap_authority)}`
            : bootstrapAuthorityDetail(state.suite_bootstrap_authority)
        }
      >
        {unpaired.length > 0 && bootstrapReady && (
          <div className="mt-2">
            <button
              onClick={pair}
              disabled={busy === "pair"}
              className="glow-honey inline-flex items-center gap-2 rounded bg-[var(--honey)] px-3 py-1.5 font-display text-[10px] font-bold uppercase tracking-wider text-primary-foreground transition hover:brightness-110 disabled:opacity-40"
            >
              {busy === "pair" ? (
                <Loader2 className="h-3 w-3 animate-spin" />
              ) : (
                <ShieldCheck className="h-3 w-3" />
              )}
              Pair {unpaired.length} product{unpaired.length === 1 ? "" : "s"}
            </button>
            <p className="mt-1 font-mono text-[10px] text-muted-foreground">
              {unpaired.map((product) => product.runtime.slug).join(", ")}
            </p>
          </div>
        )}
      </Step>

      {/* 4. Fleet launch. The durable job survives refresh and backend restart; the
             lifecycle, not a browser spinner, is the authority. */}
      <Step
        index={4}
        title="Fleet launch"
        status={fleetStatus(state.latest_fleet_launch)}
        detail={
          fleetJob
            ? `${fleetJob.mode.replaceAll("_", " ")} · ${fleetJob.summary}`
            : state.latest_fleet_launch.state === "observed"
              ? "Fleet-launch evidence is unavailable."
              : state.latest_fleet_launch.reason
        }
      >
        <div className="mt-2 flex flex-wrap gap-1.5">
          <button
            onClick={() => firstStack("start")}
            disabled={Boolean(busy) || fleetActive || !state.launcher.available}
            className="inline-flex items-center gap-1 rounded border border-[var(--ok)]/40 px-2 py-1 font-display text-[9px] uppercase tracking-wider text-[var(--ok)] transition hover:bg-[var(--ok)]/10 disabled:opacity-40"
          >
            {busy === "first-stack-start" ? <Loader2 className="h-2.5 w-2.5 animate-spin" /> : <Power className="h-2.5 w-2.5" />}
            Start first stack
          </button>
          <button
            onClick={() => firstStack("stop")}
            disabled={Boolean(busy) || fleetActive || !state.launcher.available}
            className="inline-flex items-center gap-1 rounded border border-[var(--warn)]/40 px-2 py-1 font-display text-[9px] uppercase tracking-wider text-[var(--warn)] transition hover:bg-[var(--warn)]/10 disabled:opacity-40"
          >
            {busy === "first-stack-stop" ? <Loader2 className="h-2.5 w-2.5 animate-spin" /> : <Square className="h-2.5 w-2.5" />}
            Stop first stack
          </button>
          <button
            onClick={() => launchFleet("ready")}
            disabled={Boolean(busy) || fleetActive || !state.launcher.available}
            className="inline-flex items-center gap-1 rounded border border-[var(--honey)]/40 px-2 py-1 font-display text-[9px] uppercase tracking-wider text-[var(--honey)] transition hover:bg-[var(--honey)]/10 disabled:opacity-40"
          >
            {busy === "fleet-ready" || fleetActive ? (
              <Loader2 className="h-2.5 w-2.5 animate-spin" />
            ) : (
              <Rocket className="h-2.5 w-2.5" />
            )}
            Start ready
          </button>
          <button
            onClick={() => launchFleet("all")}
            disabled={Boolean(busy) || fleetActive || !state.launcher.available}
            className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-display text-[9px] uppercase tracking-wider text-muted-foreground transition hover:border-[var(--warn)]/50 hover:text-[var(--warn)] disabled:opacity-40"
          >
            {busy === "fleet-all" ? (
              <Loader2 className="h-2.5 w-2.5 animate-spin" />
            ) : (
              <Play className="h-2.5 w-2.5" />
            )}
            Start all
          </button>
        </div>
        <details className="mt-2 rounded border border-border/70 bg-background/40 px-2 py-1.5">
          <summary className="cursor-pointer font-display text-[9px] uppercase tracking-wider text-muted-foreground">
            Product lifecycle · {state.products.filter((product) => product.launcher).length} launcher-managed
          </summary>
          <div className="mt-2 space-y-1.5">
            {state.products.map((product) => {
              const launcher = product.launcher;
              const native = product.runtime.slug === "hive-core";
              return (
                <div key={product.runtime.slug} className="rounded border border-border/60 px-2 py-1.5">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className={`h-1.5 w-1.5 rounded-full ${launcher?.compose_running ? "bg-[var(--ok)]" : "bg-muted-foreground"}`} />
                    <strong className="font-display text-[10px]">{product.runtime.title}</strong>
                    <span className="rounded border border-border px-1.5 py-0.5 font-display text-[8px] uppercase text-muted-foreground">
                      {launcher ? launcher.status : native ? "in process" : "not observed"}
                    </span>
                    {launcher && (
                      <span className="flex-1 truncate font-mono text-[9px] text-muted-foreground">
                        API {launcher.api_port} · UI {launcher.frontend_port} · {launcher.preflight_status || "preflight unknown"}
                      </span>
                    )}
                    {!native && launcher && (
                      <div className="ml-auto flex gap-1">
                        <LifecycleButton label="start" icon={Power} busy={busy === `${product.runtime.slug}-start`} disabled={Boolean(busy) || !launcher.start_ready || launcher.compose_running} onClick={() => productAction(product.runtime.slug, "start")} />
                        <LifecycleButton label="restart" icon={RotateCw} busy={busy === `${product.runtime.slug}-restart`} disabled={Boolean(busy) || !launcher.compose_running} onClick={() => productAction(product.runtime.slug, "restart")} />
                        <LifecycleButton label="stop" icon={Square} busy={busy === `${product.runtime.slug}-stop`} disabled={Boolean(busy) || !launcher.compose_running} onClick={() => productAction(product.runtime.slug, "stop")} />
                        <LifecycleButton label="logs" icon={ScrollText} busy={busy === `${product.runtime.slug}-logs`} disabled={Boolean(busy)} onClick={() => showLogs(product.runtime.slug)} />
                      </div>
                    )}
                  </div>
                  {launcher && launcher.start_blockers.length > 0 && (
                    <p className="mt-1 text-[9px] text-[var(--warn)]">{launcher.start_blockers.join(" · ")}</p>
                  )}
                  {launcher && (
                    <details className="mt-1 text-[9px] text-muted-foreground">
                      <summary className="cursor-pointer">Paths and image evidence</summary>
                      <div className="mt-1 grid gap-1 font-mono sm:grid-cols-2">
                        <span title={launcher.compose_file} className="truncate">compose: {launcher.compose_exists ? launcher.compose_file : "missing"}</span>
                        <span title={launcher.env_file} className="truncate">env: {launcher.env_exists ? launcher.env_file : "missing"}</span>
                        <span className="truncate">image: {launcher.image_status || "not observed"} {launcher.image_tag}</span>
                        <span>ports: API {launcher.api_port_open ? "open" : "closed"} · UI {launcher.frontend_port_open ? "open" : "closed"}</span>
                      </div>
                    </details>
                  )}
                </div>
              );
            })}
          </div>
        </details>
        {logs && (
          <details open className="mt-2 rounded border border-border/70 bg-background/60 p-2">
            <summary className="cursor-pointer font-display text-[9px] uppercase tracking-wider text-muted-foreground">
              {logs.title} logs · last 160 lines
            </summary>
            <div className="mt-2 flex justify-end"><button onClick={() => setLogs(null)} className="font-display text-[9px] uppercase text-muted-foreground hover:text-foreground">close</button></div>
            <pre className="mt-1 max-h-72 overflow-auto whitespace-pre-wrap rounded bg-background p-2 font-mono text-[9px] text-muted-foreground">{logs.logs || "No log output returned."}</pre>
          </details>
        )}
        {fleetJob && fleetJob.steps.length > 0 && (
          <ul className="mt-2 space-y-1">
            {fleetJob.steps.map((step) => (
              <li key={step.slug} className="flex flex-wrap items-center gap-2 text-[11px]">
                <span
                  className={`rounded border px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider ${
                    STATUS_TONE[step.lifecycle.state] ?? "border-border text-muted-foreground"
                  }`}
                >
                  {step.lifecycle.state.replaceAll("_", " ")}
                </span>
                <span className="font-mono text-[10px] text-foreground">{step.title}</span>
                <span className="flex-1 truncate text-muted-foreground">{step.message}</span>
              </li>
            ))}
          </ul>
        )}
        {previousFleetJobs.length > 0 && (
          <details className="mt-2 rounded border border-border/70 bg-background/40 px-2 py-1.5">
            <summary className="cursor-pointer font-display text-[9px] uppercase tracking-wider text-muted-foreground">
              Recent launches · {previousFleetJobs.length}
            </summary>
            <ul className="mt-1 space-y-1">
              {previousFleetJobs.map((job) => (
                <li key={job.id} className="flex flex-wrap items-center gap-2 text-[10px]">
                  <span
                    className={`rounded border px-1 py-0.5 font-display text-[8px] uppercase tracking-wider ${
                      STATUS_TONE[job.lifecycle.state] ?? "border-border text-muted-foreground"
                    }`}
                  >
                    {job.lifecycle.state.replaceAll("_", " ")}
                  </span>
                  <span className="font-mono text-muted-foreground">{job.id}</span>
                  <span className="flex-1 truncate text-muted-foreground">{job.summary}</span>
                </li>
              ))}
            </ul>
          </details>
        )}
      </Step>

      {/* 5. Evidence. A ready screen that cites a smoke run rather than asserting. */}
      <Step
        index={5}
        title="Readiness evidence"
        status={
          state.latest_smoke
            ? state.latest_smoke.status === "ok"
              ? "ok"
              : "warn"
            : "unknown"
        }
        detail={
          state.latest_smoke
            ? `${state.latest_smoke.tier || "smoke"} · ${state.latest_smoke.summary}`
            : "No smoke run recorded. Readiness is unproven, not failed."
        }
      >
        <div className="mt-2 flex flex-wrap gap-1.5">
          {SMOKE_TIERS.map((tier) => (
            <button
              key={tier.slug}
              onClick={() => smoke(tier.slug)}
              disabled={Boolean(busy)}
              className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-display text-[9px] uppercase tracking-wider text-muted-foreground transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)] disabled:opacity-40"
            >
              {busy === tier.slug ? (
                <Loader2 className="h-2.5 w-2.5 animate-spin" />
              ) : (
                <Play className="h-2.5 w-2.5" />
              )}
              {tier.label}
            </button>
          ))}
        </div>

        {state.latest_smoke && state.latest_smoke.steps.length > 0 && (
          <ul className="mt-2 space-y-1">
            {state.latest_smoke.steps.map((step, index) => (
              <li
                key={`${step.slug}-${step.check}-${index}`}
                className="flex flex-wrap items-center gap-2 text-[11px]"
              >
                <span
                  className={`rounded border px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider ${
                    STATUS_TONE[step.status] ?? "border-border text-muted-foreground"
                  }`}
                >
                  {step.status}
                </span>
                <span className="font-mono text-[10px] text-foreground">{step.title}</span>
                <span className="flex-1 truncate text-muted-foreground">{step.message}</span>
              </li>
            ))}
          </ul>
        )}
      </Step>
    </Section>
  );
}

function fleetStatus(
  evidence: BootstrapState["latest_fleet_launch"],
): "ok" | "warn" | "fail" | "unknown" {
  if (evidence.state !== "observed") {
    return evidence.state === "failed" ? "fail" : "unknown";
  }
  switch (evidence.value.lifecycle.state) {
    case "succeeded":
    case "no_op":
      return "ok";
    case "queued":
    case "running":
    case "needs_attention":
    case "blocked":
      return "warn";
    case "failed":
      return "fail";
    case "unknown":
      return "unknown";
  }
}

function bootstrapAuthorityStatus(
  authority: BootstrapState["suite_bootstrap_authority"],
  unpairedCount: number,
): "ok" | "warn" | "fail" | "unknown" {
  switch (authority.state) {
    case "ready":
      return unpairedCount > 0 ? "warn" : "ok";
    case "not_configured":
      return "warn";
    case "invalid":
      return "fail";
    case "unknown":
      return "unknown";
  }
}

function bootstrapAuthorityDetail(
  authority: BootstrapState["suite_bootstrap_authority"],
): string {
  switch (authority.state) {
    case "ready":
      return authority.source === "environment"
        ? "Bootstrap authority comes from the configured environment secret."
        : `Bootstrap authority is encrypted and durable${authority.established_at ? ` since ${authority.established_at}` : ""}.`;
    case "not_configured":
      return `Bootstrap authority is not configured: ${authority.reason}`;
    case "invalid":
      return `Bootstrap authority is invalid: ${authority.reason}`;
    case "unknown":
      return `Bootstrap authority is unknown: ${authority.reason}`;
  }
}

function Section({ children }: { children: React.ReactNode }) {
  return (
    <section
      id="bootstrap"
      className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur"
    >
      {children}
    </section>
  );
}

const stepIcon = {
  ok: CheckCircle2,
  warn: AlertTriangle,
  fail: XCircle,
  unknown: ChevronRight,
} as const;

function Step({
  index,
  title,
  status,
  detail,
  children,
}: {
  index: number;
  title: string;
  status: keyof typeof stepIcon;
  detail: string;
  children?: React.ReactNode;
}) {
  const Icon = stepIcon[status];
  const tone = STATUS_TONE[status] ?? "text-muted-foreground border-border";
  return (
    <div className="mt-3 rounded-lg border border-border bg-background/40 p-3">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-display text-[10px] text-[var(--honey)]">{index}</span>
        <Icon className={`h-3.5 w-3.5 ${tone.split(" ")[0]}`} />
        <span className="font-display text-xs font-bold">{title}</span>
        <span
          className={`rounded border px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider ${tone}`}
        >
          {status}
        </span>
      </div>
      <p className="mt-1 text-[11px] text-muted-foreground">{detail}</p>
      {children}
    </div>
  );
}

function LifecycleButton({
  label,
  icon: Icon,
  busy,
  disabled,
  onClick,
}: {
  label: string;
  icon: typeof Power;
  busy: boolean;
  disabled: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      disabled={disabled}
      className="rounded border border-border p-1 text-muted-foreground transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)] disabled:opacity-30"
    >
      {busy ? <Loader2 className="h-2.5 w-2.5 animate-spin" /> : <Icon className="h-2.5 w-2.5" />}
    </button>
  );
}

/**
 * Credential entry for one product.
 *
 * Values are write-only. A saved secret is never returned by a read — the API reports
 * configured or not — so the form clears rather than showing what is stored. GitHub
 * tokens can be checked against GitHub before being written anywhere, which is the
 * point of validating: a token that fails here never reaches a file.
 */
function CredentialForm({
  product,
  onSaved,
}: {
  product: SetupProduct;
  onSaved: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [values, setValues] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);
  const [checking, setChecking] = useState("");

  const missing = product.credentials.filter((item) => item.required && !item.configured);
  const hasDraft = Object.values(values).some((value) => value.trim().length > 0);

  function generate(requirement: CredentialRequirement): void {
    const bytes = new Uint8Array(24);
    if (!globalThis.crypto?.getRandomValues) {
      toast.error("Secure secret generation is unavailable in this browser context.");
      return;
    }
    globalThis.crypto.getRandomValues(bytes);
    const prefix = requirement.key.includes("WEBHOOK_SECRET") ? "ph-webhook" : "ph-local";
    const secret = `${prefix}-${Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("")}`;
    setValues((current) => ({ ...current, [requirement.key]: secret }));
  }

  async function save(): Promise<void> {
    const filled = Object.fromEntries(
      Object.entries(values).filter(([, value]) => value.trim().length > 0),
    );
    if (Object.keys(filled).length === 0) return;
    setSaving(true);
    const result = await saveProductEnv(product.runtime.slug, filled);
    setSaving(false);
    if (result.message) {
      toast.error(`Could not save ${product.runtime.title} credentials`, {
        description: result.message,
      });
      return;
    }
    // Clear immediately: the browser has no business holding these once written.
    setValues({});
    toast.success(`${product.runtime.title} credentials saved and product recreated`);
    onSaved();
  }

  async function check(requirement: CredentialRequirement): Promise<void> {
    const token = values[requirement.key]?.trim();
    if (!token) return;
    setChecking(requirement.key);
    const result = await validateGitHubToken(token, values.BOT_GITHUB_USER ?? "");
    setChecking("");
    if (result.data?.ok && result.data.user_matches) {
      toast.success(`Token valid${result.data.login ? ` · ${result.data.login}` : ""}`, {
        description: result.data.message,
      });
    } else if (result.data?.ok) {
      toast.warning("Token user mismatch", { description: result.data.message });
    } else {
      toast.error("Token rejected", {
        description: result.data?.message ?? result.message ?? "GitHub did not accept it.",
      });
    }
  }

  return (
    <div className="mt-2 rounded border border-border/70 bg-background/60">
      <button
        onClick={() => setOpen((value) => !value)}
        className="flex w-full flex-wrap items-center gap-2 px-2 py-1.5 text-left"
      >
        {open ? (
          <ChevronDown className="h-3 w-3 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-3 w-3 text-muted-foreground" />
        )}
        <span className="font-display text-[11px] font-bold">{product.runtime.title}</span>
        <span className={`font-mono text-[10px] ${missing.length > 0 ? "text-[var(--warn)]" : "text-[var(--ok)]"}`}>
          {missing.length > 0 ? `${missing.length} missing` : `${product.credentials.length} configured/optional`}
        </span>
      </button>

      {open && (
        <div className="space-y-2 border-t border-border/60 px-2 py-2">
          {product.credentials.map((requirement) => (
            <div key={requirement.key}>
              <label className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">
                {requirement.label}
                {!requirement.required && <span className="ml-1">· optional</span>}
                {requirement.configured && <span className="ml-1 text-[var(--ok)]">· configured</span>}
                {requirement.placeholder && (
                  <span className="ml-1 text-[var(--warn)]">· placeholder present</span>
                )}
              </label>
              {requirement.description && (
                <p className="text-[10px] text-muted-foreground">{requirement.description}</p>
              )}
              <div className="mt-1 flex gap-1.5">
                <input
                  type={requirement.redact || requirement.kind === "github_token" ? "password" : requirement.kind === "email" ? "email" : "text"}
                  value={values[requirement.key] ?? ""}
                  onChange={(event) =>
                    setValues((current) => ({
                      ...current,
                      [requirement.key]: event.target.value,
                    }))
                  }
                  autoComplete="off"
                  spellCheck={false}
                  placeholder={requirement.configured ? "Configured — leave blank to keep" : requirement.key}
                  className="flex-1 rounded border border-border bg-background/60 px-2 py-1 font-mono text-[11px] text-foreground outline-none focus:border-[var(--honey)]/50"
                />
                {requirement.kind === "github_token" && (
                  <button
                    onClick={() => check(requirement)}
                    disabled={checking === requirement.key || !values[requirement.key]}
                    className="rounded border border-border px-2 py-1 font-display text-[9px] uppercase tracking-wider text-muted-foreground transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)] disabled:opacity-40"
                    title="Check against GitHub before writing it anywhere"
                  >
                    {checking === requirement.key ? "…" : "verify"}
                  </button>
                )}
                {requirement.kind === "generated_secret" && (
                  <button
                    onClick={() => generate(requirement)}
                    className="rounded border border-border px-2 py-1 font-display text-[9px] uppercase tracking-wider text-muted-foreground transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)]"
                  >
                    generate
                  </button>
                )}
              </div>
            </div>
          ))}

          <button
            onClick={save}
            disabled={saving || !hasDraft}
            className="inline-flex items-center gap-1.5 rounded border border-[var(--honey)]/50 bg-[var(--honey)]/10 px-2 py-1 font-display text-[9px] uppercase tracking-wider text-[var(--honey)] transition hover:brightness-125 disabled:opacity-40"
          >
            {saving && <Loader2 className="h-2.5 w-2.5 animate-spin" />}
            Save to launcher
          </button>
        </div>
      )}
    </div>
  );
}
