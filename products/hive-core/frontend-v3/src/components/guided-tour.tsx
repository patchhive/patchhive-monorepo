import { useEffect, useState } from "react";
import { ArrowRight, X } from "lucide-react";
import { useHiveCommand } from "./hive-command";

const STEPS = [
  { id: "deck", title: "Deck", body: "Your top-level suite health. KPI strip, hero, honeycomb mesh." },
  { id: "registry", title: "Registry", body: "Every product with live status, latency, uptime, and drift." },
  { id: "runs", title: "Live Runs", body: "The durable run index refreshes here every ten seconds. Click a row for the drawer." },
  { id: "incidents", title: "Incidents & Runbooks", body: "Open incidents surface here; click Runbook to execute a playbook." },
  { id: "ask", title: "Ask the Hive", body: "Natural-language query grounded on the live registry." },
  { id: "audit", title: "Change Log", body: "Durable suite events and dispatches stay separate from this tab's temporary session actions." },
];

export function GuidedTour() {
  const { tourOpen, setTourOpen } = useHiveCommand();
  const [step, setStep] = useState(0);
  const [rect, setRect] = useState<DOMRect | null>(null);

  useEffect(() => {
    if (!tourOpen) return;
    setStep(0);
  }, [tourOpen]);

  useEffect(() => {
    if (!tourOpen) return;
    const el = document.getElementById(STEPS[step].id);
    if (!el) return;
    el.scrollIntoView({ behavior: "smooth", block: "center" });
    const update = () => setRect(el.getBoundingClientRect());
    update();
    const t = window.setTimeout(update, 600);
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    return () => {
      window.clearTimeout(t);
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
    };
  }, [step, tourOpen]);

  if (!tourOpen) return null;

  const finish = () => {
    setTourOpen(false);
    try {
      localStorage.setItem("hivecore.tour.seen", "1");
    } catch {
      /* ignore */
    }
  };
  const next = () => (step === STEPS.length - 1 ? finish() : setStep(step + 1));

  const cur = STEPS[step];
  const top = rect ? Math.min(window.innerHeight - 220, rect.bottom + 12) : 100;
  const left = rect ? Math.min(window.innerWidth - 340, Math.max(16, rect.left)) : 16;

  return (
    <>
      <div className="pointer-events-none fixed inset-0 z-[60]">
        {rect && (
          <div
            className="absolute rounded-lg border-2 border-[var(--honey)] shadow-[0_0_0_9999px_rgba(0,0,0,0.65)] transition-all"
            style={{
              top: rect.top - 6,
              left: rect.left - 6,
              width: rect.width + 12,
              height: rect.height + 12,
            }}
          />
        )}
      </div>
      <div
        className="fixed z-[70] w-80 rounded-xl border border-[var(--honey)]/50 bg-background/95 p-4 shadow-2xl backdrop-blur"
        style={{ top, left }}
      >
        <div className="flex items-center justify-between">
          <div className="font-display text-[10px] uppercase tracking-[0.25em] text-[var(--honey)]">
            Step {step + 1}/{STEPS.length}
          </div>
          <button onClick={finish} className="text-muted-foreground hover:text-foreground">
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
        <div className="mt-1 font-display text-base font-bold">{cur.title}</div>
        <p className="mt-1 text-xs text-muted-foreground">{cur.body}</p>
        <div className="mt-3 flex items-center justify-between">
          <button
            onClick={finish}
            className="font-display text-[10px] uppercase tracking-wider text-muted-foreground hover:text-foreground"
          >
            skip tour
          </button>
          <button
            onClick={next}
            className="inline-flex items-center gap-1 rounded-md bg-[var(--honey)] px-3 py-1.5 font-display text-[10px] font-bold uppercase tracking-wider text-primary-foreground transition hover:brightness-110"
          >
            {step === STEPS.length - 1 ? "Finish" : "Next"} <ArrowRight className="h-3 w-3" />
          </button>
        </div>
      </div>
    </>
  );
}
