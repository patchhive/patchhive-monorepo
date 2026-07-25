import { Target } from "lucide-react";

import { AUTONOMY_LEVELS, MANDATES, type AutonomyLevel } from "@/lib/suite-state";
import { PRODUCTS_BY_KEY } from "@/lib/hive-data";
import { Chip, EmptyDeck, Panel, Section, type ChipTone } from "./deck-ui";

const autonomyTone: Record<AutonomyLevel, ChipTone> = {
  observe: "neutral",
  propose: "ok",
  act_with_approval: "warn",
  act: "crit",
};

const autonomyBlurb: Record<AutonomyLevel, string> = {
  observe: "Watches only. Records nothing it would do.",
  propose: "Plans and records intended work. Dispatches nothing.",
  act_with_approval: "Dispatches read-only work; mutating work waits for a grant.",
  act: "Dispatches within policy and budget without asking.",
};

/**
 * The operator's unit of input. Runs are the output of a mandate, not the input —
 * this is what separates a suite that can be orchestrated from one that runs itself.
 */
export function MandatesPanel() {
  return (
    <Section
      id="mandates"
      title="Mandates"
      kicker="Standing intent. HiveCore keeps the suite satisfying these, inside their own budgets and politeness limits."
      actions={
        <Chip tone={MANDATES.some((m) => m.enabled) ? "honey" : "neutral"}>
          {MANDATES.filter((m) => m.enabled).length} active
        </Chip>
      }
    >
      {MANDATES.length === 0 ? (
        <>
          <EmptyDeck
            title="No mandates defined"
            detail="Without a mandate the suite only does what you click. A mandate names an objective, a bounded scope, an autonomy level, and its own PR and cost budgets."
            source="GET /mandates (architecture doc §3.6)"
          />
          <div className="mt-4 grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
            {AUTONOMY_LEVELS.map((level) => (
              <Panel key={level}>
                <Chip tone={autonomyTone[level]}>{level.replace(/_/g, " ")}</Chip>
                <p className="mt-2 text-[11px] text-muted-foreground">{autonomyBlurb[level]}</p>
              </Panel>
            ))}
          </div>
          <p className="mt-3 text-[11px] text-muted-foreground">
            A mandate cannot be raised past its proven smoke tier, and a tier regression demotes it
            automatically.
          </p>
        </>
      ) : (
        <ul className="space-y-3">
          {MANDATES.map((mandate) => (
            <li key={mandate.id}>
              <Panel>
                <div className="flex flex-wrap items-center gap-2">
                  <Target className="h-3.5 w-3.5 text-[var(--honey)]" />
                  <span className="font-display text-xs font-bold text-foreground">
                    {mandate.name}
                  </span>
                  <Chip tone={autonomyTone[mandate.autonomy]}>
                    {mandate.autonomy.replace(/_/g, " ")}
                  </Chip>
                  {!mandate.enabled && <Chip tone="neutral">paused</Chip>}
                </div>
                <p className="mt-1.5 text-xs text-muted-foreground">{mandate.objective}</p>
                <div className="mt-2 flex flex-wrap gap-1.5">
                  {mandate.scope.languages.map((language) => (
                    <Chip key={language} tone="neutral">
                      {language}
                    </Chip>
                  ))}
                  {mandate.scope.topics.map((topic) => (
                    <Chip key={topic} tone="neutral">
                      {topic}
                    </Chip>
                  ))}
                </div>
                <div className="mt-2 flex flex-wrap gap-3 font-mono text-[10px] text-muted-foreground">
                  <span>PRs ≤ {mandate.prBudget}</span>
                  <span>${mandate.costBudgetUsdPerDay.toFixed(2)}/day</span>
                  <span>{mandate.politeness.perOwnerOpenPrs} open PR per owner</span>
                  <span>cooldown {mandate.politeness.cooldownAfterClose}</span>
                </div>
                {mandate.gatedBy.length > 0 && (
                  <div className="mt-2 text-[11px] text-muted-foreground">
                    Stops if unavailable:{" "}
                    {mandate.gatedBy
                      .map((key) => PRODUCTS_BY_KEY[key]?.name ?? key)
                      .join(", ")}
                  </div>
                )}
              </Panel>
            </li>
          ))}
        </ul>
      )}
    </Section>
  );
}
