import { useEffect, useState } from "react";
import { API } from "../config.js";
import { S, Input, Sel, Btn, Divider, EmptyState, ROLE_META, PROVIDERS } from "@patchhivehq/ui";
import { AgentCard } from "@patchhivehq/ui";

const PROVIDER_MODELS = {
  anthropic: ["claude-opus-4-6","claude-sonnet-4-6","claude-haiku-4-5"],
  openai:    [
    "gpt-5.4",
    "gpt-5.4-mini",
    "gpt-5.4-nano",
    "gpt-5.3-codex",
    "gpt-5.2-codex",
    "gpt-5.1",
    "gpt-5-mini",
    "gpt-5-nano",
    "gpt-5.1-codex",
    "gpt-5.1-codex-mini",
    "gpt-5.1-codex-max",
    "gpt-5-codex",
    "gpt-5",
    "gpt-4.1",
    "gpt-4.1-mini",
    "gpt-4.1-nano",
    "o3",
    "o4-mini",
    "o3-mini",
  ],
  gemini:    ["gemini-2.0-flash","gemini-2.5-pro","gemini-2.0-flash-lite"],
  groq:      ["llama-3.3-70b-versatile","llama-3.1-8b-instant","mixtral-8x7b-32768"],
  ollama:    ["llama3.2","codellama","deepseek-coder","qwen2.5-coder"],
};

const BLANK = { name:"", role:"reaper", provider:"anthropic", model:"claude-sonnet-4-6", api_key:"", bot_token:"", bot_user:"" };

export default function TeamPanel({ agents, logs, running, cooldowns, onAdd, onRemove, apiKey = "", existingConfig = {} }) {
  const [form, setForm] = useState(BLANK);
  const [showForm, setShowForm] = useState(false);
  const [providerModels, setProviderModels] = useState(PROVIDER_MODELS);
  const [modelStatus, setModelStatus] = useState({});
  const [loadingModels, setLoadingModels] = useState({});
  const set = k => v => setForm(f => ({ ...f, [k]: v }));

  const models = providerModels[form.provider] || PROVIDER_MODELS[form.provider] || [];
  const hasCooldown = Object.keys(cooldowns||{}).length > 0;
  const agentList = Object.values(agents);

  useEffect(() => {
    const provider = form.provider;
    let cancelled = false;
    setLoadingModels(s => ({ ...s, [provider]: true }));

    fetch(`${API}/models/${provider}`, {
      headers: apiKey ? { "X-API-Key": apiKey } : {},
    })
      .then(r => r.json())
      .then(data => {
        if (cancelled) return;
        const nextModels = Array.isArray(data.models) && data.models.length
          ? data.models
          : (PROVIDER_MODELS[provider] || []);
        setProviderModels(prev => ({ ...prev, [provider]: nextModels }));
        setModelStatus(prev => ({
          ...prev,
          [provider]: {
            source: data.source || "static",
            error: data.error || "",
          },
        }));
        setForm(current => {
          if (current.provider !== provider) return current;
          if (nextModels.includes(current.model)) return current;
          return { ...current, model: nextModels[0] || "" };
        });
      })
      .catch(error => {
        if (cancelled) return;
        setModelStatus(prev => ({
          ...prev,
          [provider]: {
            source: "static_fallback",
            error: `Could not load models: ${error}`,
          },
        }));
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingModels(s => ({ ...s, [provider]: false }));
        }
      });

    return () => {
      cancelled = true;
    };
  }, [form.provider, apiKey, existingConfig?.PATCHHIVE_AI_URL]);

  const add = () => {
    if (!form.name || !form.role || !form.provider || !form.model) return;
    onAdd(form);
    setForm(BLANK);
    setShowForm(false);
  };

  const recentLogs = (logs||[]).slice(-60);

  return (
    <div style={{ display:"grid", gridTemplateColumns:"1fr 1fr", gap:16 }}>

      {/* Left: agents */}
      <div>
        <div style={{ display:"flex", alignItems:"center", gap:8, marginBottom:12 }}>
          <span style={{ fontSize:13, fontWeight:700, color:"#d4d4e8" }}>⚔ Hunt Team</span>
          <span style={{ fontSize:10, color:"#484868" }}>{agentList.length} agents</span>
          {hasCooldown && <span style={{ fontSize:9, color:"#7b2d8b", border:"1px solid #7b2d8b44", borderRadius:3, padding:"1px 5px" }}>⏸ cooling</span>}
          <div style={{ flex:1 }}/>
          <Btn onClick={() => setShowForm(s => !s)} color="#c41e3a" style={{ fontSize:10 }}>
            {showForm ? "Cancel" : "+ Add Agent"}
          </Btn>
        </div>

        {showForm && (
          <div style={{ ...S.panel, marginBottom:12, display:"grid", gridTemplateColumns:"1fr 1fr", gap:10 }}>
            <div style={{ gridColumn:"1/-1", ...S.field }}>
              <label style={S.label}>Name</label>
              <Input value={form.name} onChange={set("name")} placeholder="e.g. Grim-1" />
            </div>
            <div style={S.field}>
              <label style={S.label}>Role</label>
              <Sel value={form.role} onChange={v => { set("role")(v); }} opts={Object.entries(ROLE_META).map(([v,m]) => ({ v, l: `${m.icon} ${m.label}` }))} />
            </div>
            <div style={S.field}>
              <label style={S.label}>Provider</label>
              <Sel value={form.provider} onChange={v => { set("provider")(v); set("model")((providerModels[v] || PROVIDER_MODELS[v] || [])[0] || ""); }} opts={Object.entries(PROVIDERS).map(([v,p]) => ({ v, l:`${p.icon} ${p.label}` }))} />
            </div>
            <div style={{ gridColumn:"1/-1", ...S.field }}>
              <label style={S.label}>Model</label>
              <Sel value={form.model} onChange={set("model")} opts={models} />
              <div style={{ fontSize:10, color:modelStatus[form.provider]?.error ? "#c8922a" : "#484868", marginTop:4 }}>
                {loadingModels[form.provider]
                  ? "Loading models…"
                  : form.provider === "openai" && modelStatus[form.provider]?.source === "patchhive-ai-local"
                    ? "Live models from PatchHive Local AI."
                    : modelStatus[form.provider]?.error || "Using built-in provider model list."}
              </div>
            </div>
            {!PROVIDERS[form.provider]?.noKey && (
              <div style={{ gridColumn:"1/-1", ...S.field }}>
                <label style={S.label}>API Key (leave blank to use global)</label>
                <Input
                  value={form.api_key}
                  onChange={set("api_key")}
                  placeholder={form.provider === "openai" && existingConfig?.PATCHHIVE_AI_URL ? "optional when using PatchHive Local AI" : (PROVIDERS[form.provider]?.keyHint || "sk-…")}
                  type="password"
                />
              </div>
            )}
            <div style={S.field}>
              <label style={S.label}>Bot GitHub User (override)</label>
              <Input value={form.bot_user} onChange={set("bot_user")} placeholder="optional" />
            </div>
            <div style={S.field}>
              <label style={S.label}>Bot GitHub Token (override)</label>
              <Input value={form.bot_token} onChange={set("bot_token")} placeholder="ghp_…" type="password" />
            </div>
            <div style={{ gridColumn:"1/-1" }}>
              <Btn onClick={add} color="#c41e3a" style={{ width:"100%" }}>Recruit Agent</Btn>
            </div>
          </div>
        )}

        {agentList.length === 0
          ? <EmptyState icon="⚔" text="No agents recruited. Add at least one Reaper to begin." />
          : <div style={{ display:"flex", flexDirection:"column", gap:8 }}>
              {agentList.map(a => <AgentCard key={a.id} agent={a} onRemove={onRemove} />)}
            </div>
        }

        {/* Role legend */}
        <div style={{ marginTop:16, ...S.panel }}>
          <div style={{ fontSize:10, color:"#484868", marginBottom:8, fontWeight:700, letterSpacing:"0.08em" }}>ROLE GUIDE</div>
          <div style={{ display:"flex", flexDirection:"column", gap:5 }}>
            {Object.entries(ROLE_META).map(([id, m]) => (
              <div key={id} style={{ display:"flex", alignItems:"center", gap:8 }}>
                <span style={{ color:m.color, fontSize:12 }}>{m.icon}</span>
                <span style={{ fontSize:10, color:m.color, fontWeight:700, minWidth:80 }}>{m.label}</span>
                <span style={{ fontSize:10, color:"#484868" }}>{m.desc}</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Right: live log */}
      <div>
        <div style={{ fontSize:13, fontWeight:700, color:"#d4d4e8", marginBottom:12 }}>◎ Live Feed</div>
        <div style={{ background:"#0d0d18", border:"1px solid #1c1c30", borderRadius:6, padding:"10px 12px", height:480, overflowY:"auto", display:"flex", flexDirection:"column", gap:3 }}>
          {recentLogs.length === 0
            ? <span style={{ fontSize:10, color:"#282840" }}>Waiting for the hunt to begin…</span>
            : recentLogs.map((l, i) => {
                const color = l.type==="error" ? "#c41e3a" : l.type==="success" ? "#2a8a4a" : l.type==="warn" ? "#c8922a" : "#484868";
                const roleM = ROLE_META[l.role];
                return (
                  <div key={i} style={{ display:"flex", gap:6, fontSize:10 }}>
                    <span style={{ color:"#282840", flexShrink:0 }}>{l.ts}</span>
                    {roleM && <span style={{ color:roleM.color, flexShrink:0 }}>{roleM.icon}</span>}
                    <span style={{ color:"#484868", flexShrink:0 }}>{l.agent}</span>
                    <span style={{ color }}>{l.msg}</span>
                  </div>
                );
              })
          }
        </div>
      </div>
    </div>
  );
}
