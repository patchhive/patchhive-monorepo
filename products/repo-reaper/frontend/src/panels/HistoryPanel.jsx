import { API } from "../config.js";
import { useState, useEffect } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { S, Btn, EmptyState, timeAgo } from "@patchhivehq/ui";

export default function HistoryPanel({ apiKey = "", onViewDiff }) {
  const [history, setHistory] = useState([]);
  const [expanded, setExpanded] = useState(null);
  const fetch_ = createApiFetcher(apiKey);

  useEffect(() => {
    fetch_(`${API}/history`).then(r => r.json()).then(d => setHistory(d.history || [])).catch(() => {});
  }, []);

  const refresh = () => fetch_(`${API}/history`).then(r => r.json()).then(d => setHistory(d.history || []));

  const statusColor = s => ({ done:"#2a8a4a", error:"#c41e3a", crashed:"#c41e3a", running:"#c8922a" })[s] || "#484868";

  return (
    <div>
      <div style={{ display:"flex", alignItems:"center", marginBottom:12, gap:8 }}>
        <span style={{ fontSize:13, fontWeight:700, color:"#d4d4e8" }}>◎ Hunt History</span>
        <div style={{ flex:1 }}/>
        <Btn onClick={refresh} color="#484868" style={{ fontSize:10 }}>↻ Refresh</Btn>
      </div>

      {history.length === 0
        ? <EmptyState icon="◌" text="No hunts recorded yet." />
        : history.map(run => (
            <div key={run.id} style={{ ...S.panel, marginBottom:8 }}>
              <div style={{ display:"flex", alignItems:"center", gap:10, cursor:"pointer" }}
                onClick={() => setExpanded(expanded === run.id ? null : run.id)}>
                <span style={{ fontSize:11, color: statusColor(run.status), fontWeight:700 }}>
                  {run.status === "done" ? "✓" : run.status === "running" ? "⚔" : "✗"}
                </span>
                <span style={{ fontSize:11, color:"#d4d4e8", fontWeight:600 }}>Run {run.id}</span>
                <span style={{ fontSize:10, color:"#484868" }}>{timeAgo(run.started_at)}</span>
                <div style={{ flex:1 }} />
                <span style={{ fontSize:10, color:"#2a8a4a" }}>{run.total_fixed} kills</span>
                <span style={{ fontSize:10, color:"#484868" }}>/ {run.total_attempted} targets</span>
                {run.total_cost_usd > 0 && <span style={{ fontSize:10, color:"#c8922a" }}>${run.total_cost_usd?.toFixed(4)}</span>}
              </div>

              {expanded === run.id && run.attempts && (
                <div style={{ marginTop:12, display:"flex", flexDirection:"column", gap:6 }}>
                  {run.attempts.map(a => {
                    const color = a.status === "fixed" ? "#2a8a4a" : a.status === "error" ? "#c41e3a" : "#484868";
                    return (
                      <div key={a.id} style={{ background:"#10101e", border:"1px solid #1c1c30", borderRadius:4, padding:"8px 12px" }}>
                        <div style={{ display:"flex", alignItems:"center", gap:8 }}>
                          <span style={{ color, fontSize:11 }}>{a.status === "fixed" ? "✓" : a.status === "error" ? "✗" : "⊘"}</span>
                          <span style={{ fontSize:11, color:"#d4d4e8", flex:1 }}>#{a.issue_number} {a.issue_title}</span>
                          {a.confidence > 0 && <span style={{ fontSize:9, color:"#c8922a" }}>{a.confidence}% conf</span>}
                          {a.cost_usd > 0 && <span style={{ fontSize:9, color:"#484868" }}>${a.cost_usd.toFixed(5)}</span>}
                        </div>
                        {a.pr_url && (
                          <div style={{ fontSize:10, color:"#2a8a4a", marginTop:4, marginLeft:20 }}>
                            <a href={a.pr_url} target="_blank" rel="noreferrer">PR #{a.pr_number} ↗</a>
                            {a.patch_diff && (
                              <button onClick={() => onViewDiff(a.patch_diff)} style={{
                                marginLeft:8, background:"transparent", border:"1px solid #1c1c30",
                                borderRadius:3, cursor:"pointer", fontSize:9, color:"#484868",
                                padding:"1px 5px", fontFamily:"inherit",
                              }}>diff</button>
                            )}
                          </div>
                        )}
                        {a.skip_reason && <div style={{ fontSize:10, color:"#484868", marginTop:2, marginLeft:20 }}>Reason: {a.skip_reason}</div>}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          ))
      }
    </div>
  );
}
