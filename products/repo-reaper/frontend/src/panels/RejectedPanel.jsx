import { API } from "../config.js";
import { useState, useEffect } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { S, Btn, EmptyState, ConfidenceBar, timeAgo } from "@patchhivehq/ui";

export default function RejectedPanel({ apiKey = "", onViewDiff }) {
  const [rejected, setRejected] = useState([]);
  const [expanded, setExpanded] = useState(null);
  const fetch_ = createApiFetcher(apiKey);

  const load = () => fetch_(`${API}/rejected`).then(r => r.json()).then(d => setRejected(d.rejected || []));
  useEffect(load, []);

  return (
    <div>
      <div style={{ display:"flex", alignItems:"center", marginBottom:12, gap:8 }}>
        <span style={{ fontSize:13, fontWeight:700, color:"#d4d4e8" }}>⬢ Rejected Patches</span>
        <span style={{ fontSize:10, color:"#484868" }}>Patches the Smith refused to let through</span>
        <div style={{ flex:1 }}/>
        <Btn onClick={load} color="#484868" style={{ fontSize:10 }}>↻</Btn>
      </div>

      {rejected.length === 0
        ? <EmptyState icon="⬢" text="No rejected patches. The Smith has approved everything so far." />
        : <div style={{ display:"flex", flexDirection:"column", gap:8 }}>
            {rejected.map(r => (
              <div key={r.id} style={{ ...S.panel }}>
                <div style={{ display:"flex", alignItems:"center", gap:8, cursor:"pointer" }}
                  onClick={() => setExpanded(expanded === r.id ? null : r.id)}>
                  <span style={{ fontSize:11, color:"#7b2d8b" }}>⬢</span>
                  <span style={{ fontSize:11, color:"#d4d4e8", fontWeight:600, flex:1 }}>#{r.issue_number} {r.issue_title}</span>
                  <span style={{ fontSize:9, color:"#484868" }}>{r.repo}</span>
                  <span style={{ fontSize:9, color:"#484868" }}>{timeAgo(r.created_at)}</span>
                </div>

                <div style={{ marginTop:8, marginLeft:20 }}>
                  <div style={{ fontSize:10, color:"#7b2d8b", marginBottom:6 }}>Smith confidence: {r.confidence}%</div>
                  <ConfidenceBar value={r.confidence} size="sm" />
                  {r.smith_feedback && (
                    <div style={{ fontSize:10, color:"#484868", marginTop:8, padding:"8px", background:"#10101e", borderRadius:4, borderLeft:"2px solid #7b2d8b" }}>
                      ⬢ {r.smith_feedback}
                    </div>
                  )}
                  {expanded === r.id && r.patch_diff && (
                    <button onClick={() => onViewDiff && onViewDiff(r.patch_diff)} style={{
                      marginTop:8, background:"transparent", border:"1px solid #1c1c30",
                      borderRadius:3, cursor:"pointer", fontSize:10, color:"#484868",
                      padding:"3px 8px", fontFamily:"inherit",
                    }}>View rejected diff</button>
                  )}
                </div>
              </div>
            ))}
          </div>
      }
    </div>
  );
}
