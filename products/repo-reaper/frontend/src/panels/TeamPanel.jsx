import { useMemo, useState } from "react";
import { API } from "../config.js";
import { AIModelSelector, AI_PROVIDERS, DEFAULT_PROVIDER_MODELS, defaultBaseUrlForProvider, defaultModelForProvider } from "@patchhivehq/ai-models";
import { S, Input, Sel, Btn, EmptyState, ROLE_META } from "@patchhivehq/ui";
import { AgentCard } from "@patchhivehq/ui";

const BLANK = {
  name:"",
  role:"scout",
  provider:"anthropic",
  model:defaultModelForProvider("anthropic"),
  api_key:"",
  base_url:"",
  bot_token:"",
  bot_user:"",
};

export default function TeamPanel({ agents, logs, running, cooldowns, onAdd, onRemove, apiKey = "", existingConfig = {} }) {
  const [form, setForm] = useState(BLANK);
  const [showForm, setShowForm] = useState(false);
  const set = k => v => setForm(f => ({ ...f, [k]: v }));
  const setProvider = provider => setForm(current => {
    const priorDefault = defaultBaseUrlForProvider(current.provider);
    const keepCustomBase = current.base_url && current.base_url !== priorDefault;
    return {
      ...current,
      provider,
      base_url: defaultBaseUrlForProvider(provider) || (keepCustomBase ? current.base_url : ""),
    };
  });

  const hasCooldown = Object.keys(cooldowns||{}).length > 0;
  const agentList = Object.values(agents);
  const fallbackModels = useMemo(
    () => ({ ...DEFAULT_PROVIDER_MODELS, ...(existingConfig?.providers || {}) }),
    [existingConfig?.providers],
  );

  const resetBlankAgent = () => {
    const provider = existingConfig?.PATCHHIVE_AI_URL ? "openai" : "anthropic";
    setForm({
      ...BLANK,
      provider,
      model: defaultModelForProvider(provider, fallbackModels),
    });
  };

  const add = () => {
    if (!form.name || !form.role || !form.provider || !form.model) return;
    onAdd(form);
    resetBlankAgent();
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
          <Btn onClick={() => {
            if (!showForm) resetBlankAgent();
            setShowForm(s => !s);
          }} color="#c41e3a" style={{ fontSize:10 }}>
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
            <AIModelSelector
              apiBase={API}
              authToken={apiKey}
              provider={form.provider}
              model={form.model}
              providerKey={form.api_key}
              baseUrl={form.base_url}
              fallbackModels={fallbackModels}
              localGatewayConfigured={!!existingConfig?.PATCHHIVE_AI_URL}
              globalKeyConfigured={!!existingConfig?.PROVIDER_API_KEY_SET}
              onProviderChange={setProvider}
              onModelChange={set("model")}
            />
            {(form.provider === "custom" || form.provider === "openrouter") && (
              <div style={{ gridColumn:"1/-1", ...S.field }}>
                <label style={S.label}>{form.provider === "openrouter" ? "OpenRouter Base URL" : "Custom Base URL"}</label>
                <Input
                  value={form.base_url}
                  onChange={set("base_url")}
                  placeholder={defaultBaseUrlForProvider(form.provider) || "http://localhost:8787/v1 or https://api.example.com/v1"}
                />
                <div style={{ fontSize:10, color:"#484868", marginTop:4 }}>
                  Must speak the OpenAI-compatible chat and models APIs.
                </div>
              </div>
            )}
            {!AI_PROVIDERS[form.provider]?.noKey && (
              <div style={{ gridColumn:"1/-1", ...S.field }}>
                <label style={S.label}>API Key (leave blank to use global)</label>
                <Input
                  value={form.api_key}
                  onChange={set("api_key")}
                  placeholder={form.provider === "openai" && existingConfig?.PATCHHIVE_AI_URL ? "optional when using PatchHive Local AI" : (AI_PROVIDERS[form.provider]?.keyHint || "sk-...")}
                  type="password"
                />
                <div style={{ fontSize:10, color:"#484868", marginTop:4 }}>
                  Enter a provider-specific key, then use Refresh live on the model selector to pull the model list before recruiting.
                </div>
              </div>
            )}
            <div style={S.field}>
              <label style={S.label}>Bot GitHub User (override)</label>
              <Input value={form.bot_user} onChange={set("bot_user")} placeholder="optional" />
            </div>
            <div style={S.field}>
              <label style={S.label}>Bot GitHub Token (override)</label>
              <Input value={form.bot_token} onChange={set("bot_token")} placeholder="github_pat_…" type="password" />
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
