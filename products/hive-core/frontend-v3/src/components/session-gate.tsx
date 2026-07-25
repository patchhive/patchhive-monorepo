import { useEffect, useState, type FormEvent, type ReactNode } from "react";
import { Hexagon, KeyRound, Loader2, ShieldAlert } from "lucide-react";

import { API, API_KEY_STORAGE } from "@/config";

/**
 * Operator session for the deck.
 *
 * The suite API is fail-closed: with no key configured the backend refuses every
 * protected route with 503 rather than running open, so this gate has three states
 * and not two — needs bootstrap, needs login, authenticated.
 *
 * The key lives in localStorage and travels as X-API-Key. There is no cookie and no
 * server-side session, so nothing between the browser and the Rust backend holds a
 * credential.
 */
type Phase = "checking" | "bootstrap" | "login" | "ready" | "unreachable";

interface AuthStatus {
  auth_enabled: boolean;
  bootstrap_required: boolean;
}

export function readApiKey(): string {
  if (typeof window === "undefined") return "";
  return window.localStorage.getItem(API_KEY_STORAGE) ?? "";
}

export function SessionGate({ children }: { children: ReactNode }) {
  const [phase, setPhase] = useState<Phase>("checking");
  const [error, setError] = useState("");
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [generated, setGenerated] = useState("");

  useEffect(() => {
    let cancelled = false;

    async function check() {
      try {
        const response = await fetch(`${API}/api/auth/status`);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const status = (await response.json()) as AuthStatus;
        if (cancelled) return;

        if (!status.auth_enabled) {
          setPhase("bootstrap");
          return;
        }

        const stored = readApiKey();
        if (!stored) {
          setPhase("login");
          return;
        }

        // A stored key can be stale — the hash may have been rotated in .env.
        const probe = await fetch(`${API}/api/auth/login`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ api_key: stored }),
        });
        if (cancelled) return;
        setPhase(probe.ok ? "ready" : "login");
        if (!probe.ok) setError("Stored key was rejected. It may have been rotated.");
      } catch {
        if (!cancelled) setPhase("unreachable");
      }
    }

    check();
    return () => {
      cancelled = true;
    };
  }, []);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!key.trim() || busy) return;
    setBusy(true);
    setError("");
    try {
      const response = await fetch(`${API}/api/auth/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ api_key: key.trim() }),
      });
      if (response.ok) {
        window.localStorage.setItem(API_KEY_STORAGE, key.trim());
        setPhase("ready");
      } else {
        setError("That key was rejected.");
      }
    } catch {
      setError("Could not reach the control plane.");
    } finally {
      setBusy(false);
    }
  }

  async function bootstrap() {
    setBusy(true);
    setError("");
    try {
      const response = await fetch(`${API}/api/auth/generate-key`, { method: "POST" });
      const payload = (await response.json()) as { api_key?: string; message?: string };
      if (response.ok && payload.api_key) {
        window.localStorage.setItem(API_KEY_STORAGE, payload.api_key);
        setGenerated(payload.api_key);
      } else {
        setError(payload.message ?? `Bootstrap failed (HTTP ${response.status}).`);
      }
    } catch {
      setError("Could not reach the control plane.");
    } finally {
      setBusy(false);
    }
  }

  if (phase === "ready") return <>{children}</>;

  return (
    <main className="hex-grid relative flex min-h-screen items-center justify-center px-6">
      <div className="pointer-events-none absolute inset-0 scanline opacity-40" />
      <div className="relative w-full max-w-md rounded-2xl border border-border bg-card/60 p-8 backdrop-blur">
        <div className="mb-6 flex items-center gap-3">
          <div className="relative grid h-10 w-10 place-items-center">
            <Hexagon
              className="absolute inset-0 h-full w-full text-[var(--honey)]"
              strokeWidth={1.4}
            />
            <Hexagon className="h-5 w-5 fill-[var(--honey)] text-[var(--honey)]" />
          </div>
          <div className="leading-tight">
            <div className="font-display text-lg font-bold tracking-tight">
              <span className="text-gradient-honey">HIVE</span>
              <span className="text-foreground">CORE</span>
            </div>
            <div className="font-display text-[10px] uppercase tracking-[0.25em] text-muted-foreground">
              operator session
            </div>
          </div>
        </div>

        {phase === "checking" && (
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <Loader2 className="h-3.5 w-3.5 animate-spin" /> Checking the control plane…
          </div>
        )}

        {phase === "unreachable" && (
          <div className="rounded-lg border border-[var(--crit)]/40 bg-[var(--crit)]/[0.06] p-4">
            <div className="flex items-center gap-2 font-display text-[10px] font-bold uppercase tracking-wider text-[var(--crit)]">
              <ShieldAlert className="h-3 w-3" /> Control plane unreachable
            </div>
            <p className="mt-1 text-xs text-muted-foreground">
              Nothing is listening at <code className="font-mono">{API}</code>. Start
              patchhive-backend and reload.
            </p>
          </div>
        )}

        {phase === "bootstrap" &&
          (generated ? (
            <div>
              <p className="text-xs text-muted-foreground">
                Key generated and stored in this browser. Save it somewhere — it will not be
                shown again.
              </p>
              <code className="mt-3 block break-all rounded border border-[var(--honey)]/40 bg-background/60 p-3 font-mono text-[11px] text-[var(--honey)]">
                {generated}
              </code>
              <button
                onClick={() => setPhase("ready")}
                className="glow-honey mt-4 w-full rounded-md bg-[var(--honey)] px-4 py-2.5 font-display text-xs font-bold uppercase tracking-wider text-primary-foreground transition hover:brightness-110"
              >
                Enter the deck
              </button>
            </div>
          ) : (
            <div>
              <p className="text-xs text-muted-foreground">
                No suite key is configured, so the control plane is refusing every protected
                route. Generate the first one — this only works from localhost, and only once.
              </p>
              <button
                onClick={bootstrap}
                disabled={busy}
                className="glow-honey mt-4 inline-flex w-full items-center justify-center gap-2 rounded-md bg-[var(--honey)] px-4 py-2.5 font-display text-xs font-bold uppercase tracking-wider text-primary-foreground transition hover:brightness-110 disabled:opacity-50"
              >
                {busy ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <KeyRound className="h-3.5 w-3.5" />}
                Generate suite key
              </button>
            </div>
          ))}

        {phase === "login" && (
          <form onSubmit={submit}>
            <label
              htmlFor="suite-key"
              className="font-display text-[10px] uppercase tracking-wider text-muted-foreground"
            >
              Suite API key
            </label>
            <input
              id="suite-key"
              type="password"
              value={key}
              onChange={(event) => setKey(event.target.value)}
              placeholder="ph-suite-…"
              autoFocus
              className="mt-2 w-full rounded border border-border bg-background/60 px-3 py-2 font-mono text-sm text-foreground outline-none focus:border-[var(--honey)]/60"
            />
            <button
              type="submit"
              disabled={busy || !key.trim()}
              className="glow-honey mt-4 inline-flex w-full items-center justify-center gap-2 rounded-md bg-[var(--honey)] px-4 py-2.5 font-display text-xs font-bold uppercase tracking-wider text-primary-foreground transition hover:brightness-110 disabled:opacity-50"
            >
              {busy ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <KeyRound className="h-3.5 w-3.5" />}
              Unlock
            </button>
          </form>
        )}

        {error && <p className="mt-3 text-xs text-[var(--crit)]">{error}</p>}

        <p className="mt-6 text-[10px] leading-relaxed text-muted-foreground">
          The key is stored in this browser and sent as <code className="font-mono">X-API-Key</code>{" "}
          to {API} only. No cookie, no server-side session.
        </p>
      </div>
    </main>
  );
}
