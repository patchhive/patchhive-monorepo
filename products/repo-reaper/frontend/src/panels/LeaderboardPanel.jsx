import { API } from "../config.js";
import { useState, useEffect } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { S, Btn, EmptyState, ConfidenceBar, ROLE_META, PROVIDERS } from "@patchhivehq/ui";

export default function LeaderboardPanel({ apiKey = "" }) {
  const [data, setData] = useState([]);
  const [lifetime, setLifetime] = useState(0);
  const fetch_ = createApiFetcher(apiKey);

  const load = () => {
    fetch_(`${API}/leaderboard`).then(r => r.json()).then(d => setData(d.leaderboard || []));
    fetch_(`${API}/stats/lifetime-cost`).then(r => r.json()).then(d => setLifetime(d.lifetime_cost_usd || 0));
  };
  useEffect(load, []);

  return (
    <div>
      <div style={{ display:"flex", alignItems:"center", marginBottom:12, gap:8 }}>
        <span style={{ fontSize:13, fontWeight:700, color:"#d4d4e8" }}>⚖ Agent Leaderboard</span>
        <div style={{ flex:1 }}/>
        {lifetime > 0 && <span style={{ fontSize:10, color:"#c8922a" }}>Lifetime: ${lifetime.toFixed(4)}</span>}
        <Btn onClick={load} color="#484868" style={{ fontSize:10 }}>↻</Btn>
      </div>

      {data.length === 0
        ? <EmptyState icon="⚖" text="No agent stats yet. Run a hunt first." />
        : <div style={{ ...S.panel }}>
            <div style={{ display:"grid", gridTemplateColumns:"1fr 80px 60px 60px 60px 60px 80px", gap:8, fontSize:9, color:"#484868", fontWeight:700, letterSpacing:"0.08em", padding:"0 0 8px", borderBottom:"1px solid #1c1c30", marginBottom:8 }}>
              <div>AGENT</div><div>ROLE</div><div style={{textAlign:"right"}}>KILLS</div><div style={{textAlign:"right"}}>ESCAPED</div><div style={{textAlign:"right"}}>ERRORS</div><div style={{textAlign:"right"}}>RATE</div><div style={{textAlign:"right"}}>COST</div>
            </div>
            {data.map((row, i) => {
              const role = ROLE_META[row.role] || { icon:"◎", color:"#888" };
              const prov = PROVIDERS[row.provider] || { color:"#888" };
              return (
                <div key={i} style={{ display:"grid", gridTemplateColumns:"1fr 80px 60px 60px 60px 60px 80px", gap:8, fontSize:11, padding:"6px 0", borderBottom:"1px solid #10101e", alignItems:"center" }}>
                  <div>
                    <div style={{ color:"#d4d4e8", fontWeight:600 }}>{row.agent_name}</div>
                    <div style={{ fontSize:9, color: prov.color }}>{row.provider} · {row.model}</div>
                  </div>
                  <div style={{ color: role.color, fontSize:10 }}>{role.icon} {row.role}</div>
                  <div style={{ textAlign:"right", color:"#2a8a4a" }}>{row.total_fixed}</div>
                  <div style={{ textAlign:"right", color:"#484868" }}>{row.total_skipped}</div>
                  <div style={{ textAlign:"right", color:"#c41e3a" }}>{row.total_errors}</div>
                  <div style={{ textAlign:"right", color: row.fix_rate >= 70 ? "#2a8a4a" : row.fix_rate >= 40 ? "#c8922a" : "#c41e3a" }}>
                    {row.fix_rate}%
                  </div>
                  <div style={{ textAlign:"right", color:"#484868", fontSize:10 }}>${(row.total_cost_usd||0).toFixed(4)}</div>
                </div>
              );
            })}
          </div>
      }
    </div>
  );
}
