import { useState } from "react";
import { CircleSlash, ShieldCheck } from "lucide-react";

import {
  POLICY_PRECEDENCE,
  REPOSITORY_POLICIES,
  type PolicyListKind,
} from "@/lib/suite-state";
import { Chip, EmptyDeck, Panel, Section, type ChipTone } from "./deck-ui";

const kindTone: Record<PolicyListKind, ChipTone> = {
  opt_out: "crit",
  denylist: "warn",
  allowlist: "ok",
  trusted: "honey",
};

const kindLabel: Record<PolicyListKind, string> = {
  opt_out: "opt-out",
  denylist: "denylist",
  allowlist: "allowlist",
  trusted: "trusted",
};

/**
 * Repository policy as structured rows. Today the allowlist and denylist are
 * free-text fields in suite_settings, re-parsed on every check — the suite's
 * autonomous-safety boundary is currently a textarea (blocker B4).
 */
export function PolicyPanel() {
  const [probe, setProbe] = useState("");

  return (
    <Section
      id="policy"
      title="Repository policy"
      kicker="Opt-out, denylist, allowlist, and trust — evaluated in a fixed order that later grants cannot override."
    >
      <div className="grid gap-4 lg:grid-cols-[1fr_320px]">
        <div>
          {REPOSITORY_POLICIES.length === 0 ? (
            <EmptyDeck
              title="No policy entries"
              detail="Nothing is excluded, allowed, or trusted. With no allowlist configured, autonomous discovery is bounded only by product-level scope — worth fixing before raising autonomy."
              source="GET /repository-policies"
            />
          ) : (
            <ul className="space-y-1">
              {REPOSITORY_POLICIES.map((entry) => (
                <li
                  key={`${entry.kind}-${entry.repository}`}
                  className="flex items-center gap-2 rounded border border-border/60 px-2 py-1.5 text-xs"
                >
                  <Chip tone={kindTone[entry.kind]}>{kindLabel[entry.kind]}</Chip>
                  <span className="flex-1 truncate font-mono text-[11px] text-foreground">
                    {entry.repository}
                  </span>
                  {entry.kind === "opt_out" &&
                    (entry.verified ? (
                      <Chip tone="ok">
                        <ShieldCheck className="h-2.5 w-2.5" /> verified
                      </Chip>
                    ) : (
                      <Chip tone="warn">operator-entered</Chip>
                    ))}
                </li>
              ))}
            </ul>
          )}

          <Panel className="mt-4">
            <label
              htmlFor="policy-probe"
              className="font-display text-[10px] uppercase tracking-wider text-muted-foreground"
            >
              Test a repository
            </label>
            <div className="mt-2 flex gap-2">
              <input
                id="policy-probe"
                value={probe}
                onChange={(event) => setProbe(event.target.value)}
                placeholder="owner/repo"
                className="flex-1 rounded border border-border bg-background/60 px-2 py-1.5 font-mono text-xs text-foreground outline-none focus:border-[var(--honey)]/50"
              />
              <button
                disabled
                title="Wire POST /repository-policy/check to enable"
                className="rounded border border-border px-3 py-1.5 font-display text-[10px] uppercase tracking-wider text-muted-foreground"
              >
                Evaluate
              </button>
            </div>
            <p className="mt-2 text-[11px] text-muted-foreground">
              Returns the full precedence chain, not a verdict — every step and why it passed or
              blocked.
            </p>
          </Panel>
        </div>

        <Panel>
          <div className="flex items-center gap-2 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
            <CircleSlash className="h-3 w-3" /> Evaluation order
          </div>
          <ol className="mt-2 space-y-1.5">
            {POLICY_PRECEDENCE.map((step, index) => (
              <li key={step} className="flex gap-2 text-[11px] text-foreground">
                <span className="font-mono text-muted-foreground">{index + 1}.</span>
                {step}
              </li>
            ))}
          </ol>
          <p className="mt-3 text-[11px] text-muted-foreground">
            Earlier denials cannot be overridden by later grants. Trust and remaining budget never
            override an opt-out.
          </p>
        </Panel>
      </div>
    </Section>
  );
}
