import { ScoreBadge, ConfidenceBar, Tag, STATUS_LABELS, ROLE_META, timeAgo } from "../index.js";

const STATUS_COLOR = {
  queued:   "#484868",
  running:  "#c8922a",
  fixed:    "#2a8a4a",
  skipped:  "#484868",
  rejected: "#7b2d8b",
  error:    "#c41e3a",
};

const STATUS_ICON = {
  queued:   "◌",
  running:  "⚔",
  fixed:    "✓",
  skipped:  "⊘",
  rejected: "⬢",
  error:    "✗",
};

export default function IssueRow({ issue, onViewDiff }) {
  const status = issue.status || "queued";
  const color = STATUS_COLOR[status] || "#484868";
  const icon  = STATUS_ICON[status]  || "◌";
  const conf  = issue.confidence;

  return (
    <div style={{
      background:"#0d0d18", border:`1px solid ${status==="running" ? "#c41e3a33" : "#1c1c30"}`,
      borderRadius:5, padding:"10px 14px", transition:"border-color 0.2s",
    }}>
      <div style={{ display:"flex", alignItems:"center", gap:8, marginBottom:4 }}>
        <span style={{ color, fontSize:12, fontWeight:700, flexShrink:0 }}>{icon}</span>
        <span style={{ fontSize:11, color:"#d4d4e8", flex:1, fontWeight:600 }}>
          #{issue.number} — {issue.title}
        </span>
        {issue.fixability_score != null && <ScoreBadge score={issue.fixability_score} />}
      </div>

      <div style={{ fontSize:10, color:"#484868", marginLeft:20, marginBottom: conf != null ? 6 : 0 }}>
        {issue.repo}
        {issue.fixability_reason && <span> · {issue.fixability_reason}</span>}
        {status !== "queued" && (
          <span style={{ color, marginLeft:8 }}>
            {STATUS_LABELS[status] || status}
            {issue.reason && status === "skipped" && ` (${issue.reason})`}
          </span>
        )}
      </div>

      {conf != null && (
        <div style={{ marginLeft:20, marginBottom:4 }}>
          <div style={{ fontSize:9, color:"#484868", marginBottom:3 }}>Reaper confidence</div>
          <ConfidenceBar value={conf} size="sm" />
        </div>
      )}

      {status === "rejected" && issue.feedback && (
        <div style={{ marginLeft:20, fontSize:10, color:"#7b2d8b", marginTop:4 }}>
          ⬢ Smith: {issue.feedback}
        </div>
      )}

      {status === "fixed" && issue.pr_url && (
        <div style={{ display:"flex", alignItems:"center", gap:8, marginLeft:20, marginTop:4 }}>
          <a href={issue.pr_url} target="_blank" rel="noreferrer"
            style={{ fontSize:10, color:"#2a8a4a", textDecoration:"none" }}>
            PR #{issue.pr_number} ↗
          </a>
          {issue.diff && (
            <button onClick={() => onViewDiff && onViewDiff(issue.diff)} style={{
              background:"transparent", border:"1px solid #1c1c30", borderRadius:3,
              cursor:"pointer", fontSize:9, color:"#484868", padding:"1px 6px", fontFamily:"inherit",
            }}>view diff</button>
          )}
        </div>
      )}
    </div>
  );
}
