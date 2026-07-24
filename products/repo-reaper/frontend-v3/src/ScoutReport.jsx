import { ShieldCheck } from "lucide-react";
import { V3_TEXT } from "@patchhivehq/ui-v3";
import { Chip, normalizeCollection } from "./shared.jsx";

function reportTone(value) {
  const normalized = String(value || "").toLowerCase();
  if (normalized.includes("safe") || normalized === "attempt" || normalized === "high") return "ok";
  if (normalized.includes("skip") || normalized === "low") return "hot";
  if (normalized.includes("review") || normalized === "medium") return "warn";
  return "neutral";
}

export function scoutReportMarkdown(report) {
  if (!report) return "";
  const candidates = normalizeCollection(report.candidates);
  const top = report.top_candidate || {};
  const lines = [
    "## Scout assessment",
    "",
    `- **Recommendation:** ${report.recommendation || "No recommendation"}`,
    `- **Success band:** ${report.success_band || "Unknown"}`,
    `- **Risk:** ${report.risk || "No risk statement supplied"}`,
    "",
    report.summary || "No assessment summary was supplied.",
  ];
  if (top.title) {
    lines.push(
      "",
      "### Top candidate",
      "",
      `**${top.title}**`,
      "",
      `${top.repo || "Repository unavailable"} · ${top.score ?? "Unscored"}/100`,
      "",
      top.why || "No rationale supplied.",
    );
  }
  if (candidates.length) {
    lines.push("", "### Candidate shortlist", "");
    candidates.forEach((candidate, index) => {
      lines.push(
        `${index + 1}. **${candidate.title || "Untitled candidate"}** — ${candidate.call || "review"}, ${candidate.score ?? "unscored"}/100`,
        `   - ${candidate.repo || "Repository unavailable"}`,
        `   - ${candidate.reason || "No rationale supplied."}`,
      );
    });
  }
  return lines.join("\n");
}

export default function ScoutReport({ report }) {
  if (!report) return null;
  const candidates = normalizeCollection(report.candidates);
  const top = report.top_candidate || {};

  return <div className="surface-inset mt-4 rounded-xl p-4 sm:p-5">
    <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
      <div>
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}><ShieldCheck size={12}/> Scout assessment</div>
        <h3 className={`mt-2 font-display text-[22px] font-semibold ${V3_TEXT.strong}`}>{report.summary || "Scout analysis complete."}</h3>
      </div>
      <div className="flex shrink-0 flex-wrap gap-2">
        <Chip tone={reportTone(report.recommendation)}>{report.recommendation || "no recommendation"}</Chip>
        <Chip tone={reportTone(report.success_band)}>{report.success_band || "unknown"} success</Chip>
      </div>
    </div>

    <div className="mt-4 grid gap-3 lg:grid-cols-[1.25fr_0.75fr]">
      <div className="surface rounded-xl p-4">
        <div className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>Top candidate</div>
        <div className={`mt-2 font-display text-[17px] font-semibold ${V3_TEXT.strong}`}>{top.title || "No top candidate selected"}</div>
        <div className={`mt-1 text-[10px] ${V3_TEXT.mute}`}>{top.repo || "Repository unavailable"}{top.score != null ? ` · ${top.score}/100` : ""}</div>
        <p className={`mt-3 text-[12px] leading-relaxed ${V3_TEXT.body}`}>{top.why || "The Scout did not provide a top-candidate rationale."}</p>
      </div>
      <div className="surface rounded-xl p-4">
        <div className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>Risk assessment</div>
        <p className={`mt-2 text-[12px] leading-relaxed ${V3_TEXT.body}`}>{report.risk || "The Scout did not provide a risk statement."}</p>
      </div>
    </div>

    {candidates.length ? <div className="mt-4">
      <div className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>Candidate shortlist</div>
      <div className="mt-2 grid gap-2">
        {candidates.map((candidate, index) => <article className="surface rounded-xl p-3" key={`${candidate.repo || "repo"}-${candidate.title || index}`}>
          <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <div className={`font-display text-[14px] font-semibold ${V3_TEXT.strong}`}>{candidate.title || "Untitled candidate"}</div>
              <div className={`mt-1 text-[10px] ${V3_TEXT.mute}`}>{candidate.repo || "Repository unavailable"}</div>
              <p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.body}`}>{candidate.reason || "No rationale supplied."}</p>
            </div>
            <div className="flex shrink-0 gap-2"><Chip tone={reportTone(candidate.call)}>{candidate.call || "review"}</Chip><Chip>{candidate.score ?? "unscored"}{candidate.score != null ? "/100" : ""}</Chip></div>
          </div>
        </article>)}
      </div>
    </div> : null}

    <details className="mt-4">
      <summary className={`cursor-pointer text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>Raw Scout response</summary>
      <pre className={`surface mt-3 max-h-96 overflow-auto whitespace-pre-wrap rounded-xl p-4 text-[10px] leading-relaxed ${V3_TEXT.body}`}>{JSON.stringify(report, null, 2)}</pre>
    </details>
  </div>;
}
