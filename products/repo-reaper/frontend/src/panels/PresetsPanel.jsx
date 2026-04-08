import { API } from "../config.js";
import { useState, useEffect } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { S, Input, Btn, EmptyState, ROLE_META, PROVIDERS, timeAgo } from "@patchhivehq/ui";

export default function PresetsPanel({ apiKey = "", currentAgents, onLoadPreset }) {
  const [presets, setPresets] = useState([]);
  const [saveName, setSaveName] = useState("");
  const [saving, setSaving] = useState(false);
  const fetch_ = createApiFetcher(apiKey);

  const load = () => fetch_(`${API}/presets`).then(r => r.json()).then(d => setPresets(d.presets || []));
  useEffect(load, []);

  const save = async () => {
    if (!saveName.trim() || currentAgents.length === 0) return;
    setSaving(true);
    await fetch_(`${API}/presets`, {
      method:"POST", headers:{ "Content-Type":"application/json" },
      body: JSON.stringify({ name: saveName, agents: currentAgents }),
    });
    setSaveName(""); load();
    setSaving(false);
  };

  const del = async name => {
    await fetch_(`${API}/presets/${encodeURIComponent(name)}`, { method:"DELETE" });
    load();
  };

  return (
    <div>
      <div style={{ fontSize:13, fontWeight:700, color:"#d4d4e8", marginBottom:16 }}>⬢ Team Presets</div>

      {/* Save current team */}
      <div style={{ ...S.panel, marginBottom:16 }}>
        <div style={{ fontSize:11, fontWeight:700, color:"#d4d4e8", marginBottom:10 }}>Save Current Team</div>
        <div style={{ display:"flex", gap:8 }}>
          <Input value={saveName} onChange={setSaveName} placeholder="Preset name…" style={{ flex:1 }} />
          <Btn onClick={save} disabled={saving || !saveName.trim() || currentAgents.length === 0} color="#c41e3a">
            {saving ? "Saving…" : "Save"}
          </Btn>
        </div>
        {currentAgents.length === 0 && <div style={{ fontSize:10, color:"#484868", marginTop:6 }}>Add agents to the team first.</div>}
      </div>

      {presets.length === 0
        ? <EmptyState icon="⬢" text="No presets saved. Save your current team configuration above." />
        : <div style={{ display:"flex", flexDirection:"column", gap:10 }}>
            {presets.map(p => (
              <div key={p.name} style={{ ...S.panel }}>
                <div style={{ display:"flex", alignItems:"center", gap:8, marginBottom:10 }}>
                  <span style={{ fontSize:12, fontWeight:700, color:"#d4d4e8", flex:1 }}>{p.name}</span>
                  <span style={{ fontSize:9, color:"#484868" }}>{timeAgo(p.created_at)}</span>
                  <Btn onClick={() => onLoadPreset(p.agents)} color="#c41e3a" style={{ fontSize:10 }}>Load</Btn>
                  <Btn onClick={() => del(p.name)} color="#484868" style={{ fontSize:10 }}>Delete</Btn>
                </div>
                <div style={{ display:"flex", flexWrap:"wrap", gap:6 }}>
                  {(p.agents||[]).map((a, i) => {
                    const role = ROLE_META[a.role] || { icon:"◎", color:"#888", label: a.role };
                    const prov = PROVIDERS[a.provider] || { color:"#888" };
                    return (
                      <div key={i} style={{ fontSize:10, color: role.color, border:`1px solid ${role.color}33`, borderRadius:3, padding:"2px 7px" }}>
                        {role.icon} {a.name} <span style={{ color:"#484868" }}>({role.label})</span>
                      </div>
                    );
                  })}
                </div>
              </div>
            ))}
          </div>
      }
    </div>
  );
}
