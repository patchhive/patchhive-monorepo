import { useCallback, useEffect, useState } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Loader2,
  Play,
  Rocket,
  ShieldCheck,
  XCircle,
} from "lucide-react";
import { toast } from "sonner";
import {
  fetchBootstrap,
  pairFirstStack,
  runSmoke,
  saveProductEnv,
  SMOKE_TIERS,
  STATUS_TONE,
  validateGitHubToken,
  type BootstrapState,
  type CredentialRequirement,
  type SetupProduct,
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

  async function pair(): Promise<void> {
    setBusy("pair");
    const result = await pairFirstStack();
    setBusy("");
    if (result.data) {
      setState(result.data);
      toast.success("Pairing complete");
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
          needsCredentials.map((product) => (
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
        status={unpaired.length > 0 ? "warn" : "ok"}
        detail={
          state.suite_bootstrap_configured
            ? unpaired.length > 0
              ? `${unpaired.length} product(s) reachable but unpaired.`
              : "Every reachable product has a scoped service token."
            : "PATCHHIVE_SUITE_BOOTSTRAP_SECRET is not set, so HiveCore cannot mint tokens."
        }
      >
        {unpaired.length > 0 && state.suite_bootstrap_configured && (
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

      {/* 4. Evidence. A ready screen that cites a smoke run rather than asserting. */}
      <Step
        index={4}
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
    toast.success(`${product.runtime.title} credentials saved`);
    onSaved();
  }

  async function check(requirement: CredentialRequirement): Promise<void> {
    const token = values[requirement.key]?.trim();
    if (!token) return;
    setChecking(requirement.key);
    const result = await validateGitHubToken(token, requirement.profile);
    setChecking("");
    if (result.data?.valid) {
      toast.success(`Token valid${result.data.login ? ` · ${result.data.login}` : ""}`, {
        description: result.data.scopes?.join(", "),
      });
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
        <span className="font-mono text-[10px] text-[var(--warn)]">
          {missing.length} missing
        </span>
      </button>

      {open && (
        <div className="space-y-2 border-t border-border/60 px-2 py-2">
          {missing.map((requirement) => (
            <div key={requirement.key}>
              <label className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">
                {requirement.label}
                {requirement.placeholder && (
                  <span className="ml-1 text-[var(--warn)]">· placeholder present</span>
                )}
              </label>
              {requirement.description && (
                <p className="text-[10px] text-muted-foreground">{requirement.description}</p>
              )}
              <div className="mt-1 flex gap-1.5">
                <input
                  type={requirement.redact ? "password" : "text"}
                  value={values[requirement.key] ?? ""}
                  onChange={(event) =>
                    setValues((current) => ({
                      ...current,
                      [requirement.key]: event.target.value,
                    }))
                  }
                  autoComplete="off"
                  spellCheck={false}
                  placeholder={requirement.key}
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
              </div>
            </div>
          ))}

          <button
            onClick={save}
            disabled={saving}
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
