import { API } from "../config.js";
import { useState, useEffect } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { S, Input, Btn, Divider } from "@patchhivehq/ui";

export default function ConfigPanel({ existingConfig, apiKey = "", onSaved }) {
  const [cfg, setCfg] = useState({
    BOT_GITHUB_TOKEN:"", BOT_GITHUB_USER:"", BOT_GITHUB_EMAIL:"",
    PROVIDER_API_KEY:"", PATCHHIVE_AI_URL:"", OLLAMA_BASE_URL:"", WEBHOOK_SECRET:"",
    COST_BUDGET_USD:"0", MIN_REVIEW_CONFIDENCE:"40",
  });
  const [saved, setSaved] = useState(false);
  const [aiStatus, setAiStatus] = useState(existingConfig?.AI_LOCAL_STATUS || { configured:false });
  const [loadingAiStatus, setLoadingAiStatus] = useState(false);
  const fetch_ = createApiFetcher(apiKey);

  useEffect(() => {
    if (existingConfig) setCfg(c => ({ ...c, ...existingConfig }));
  }, [existingConfig]);

  const loadAiStatus = async () => {
    setLoadingAiStatus(true);
    try {
      const resp = await fetch_(`${API}/ai-local/status`);
      const data = await resp.json();
      setAiStatus(data);
    } catch (error) {
      setAiStatus({ configured:true, ok:false, error:`Could not load local AI status: ${error}` });
    } finally {
      setLoadingAiStatus(false);
    }
  };

  useEffect(() => {
    if (existingConfig?.AI_LOCAL_STATUS) {
      setAiStatus(existingConfig.AI_LOCAL_STATUS);
      return;
    }
    loadAiStatus();
  }, [existingConfig?.AI_LOCAL_STATUS, existingConfig?.PATCHHIVE_AI_URL]);

  const set = k => v => setCfg(c => ({ ...c, [k]: v }));

  const save = async () => {
    await fetch_(`${API}/config`, {
      method:"POST", headers:{ "Content-Type":"application/json" },
      body: JSON.stringify(cfg),
    });
    if (onSaved) await onSaved();
    await loadAiStatus();
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  const providerEntries = Object.entries(aiStatus?.providers || {});
  const aiStatusColor = !aiStatus?.configured
    ? "#484868"
    : aiStatus?.ok
      ? "#2a8a4a"
      : "#c8922a";

  return (
    <div style={{ maxWidth:600 }}>
      <div style={{ fontSize:13, fontWeight:700, color:"#d4d4e8", marginBottom:16 }}>⚙ Configuration</div>

      <div style={{ ...S.panel, display:"flex", flexDirection:"column", gap:12 }}>
        <div style={{ fontSize:11, fontWeight:700, color:"#c41e3a" }}>GitHub Bot</div>
        <div style={S.field}>
          <label style={S.label}>Bot GitHub Token</label>
          <Input
            value={cfg.BOT_GITHUB_TOKEN}
            onChange={set("BOT_GITHUB_TOKEN")}
            type="password"
            placeholder={existingConfig?.BOT_GITHUB_TOKEN_SET ? "(saved, leave blank to keep)" : "ghp_…"}
          />
        </div>
        <div style={{ display:"grid", gridTemplateColumns:"1fr 1fr", gap:10 }}>
          <div style={S.field}>
            <label style={S.label}>Bot GitHub Username</label>
            <Input value={cfg.BOT_GITHUB_USER} onChange={set("BOT_GITHUB_USER")} placeholder="your-bot-user" />
          </div>
          <div style={S.field}>
            <label style={S.label}>Bot GitHub Email</label>
            <Input value={cfg.BOT_GITHUB_EMAIL} onChange={set("BOT_GITHUB_EMAIL")} placeholder="bot@example.com" />
          </div>
        </div>

        <Divider />
        <div style={{ fontSize:11, fontWeight:700, color:"#c41e3a" }}>AI Provider</div>
        <div style={S.field}>
          <label style={S.label}>Global API Key (used when agent has no key)</label>
          <Input
            value={cfg.PROVIDER_API_KEY}
            onChange={set("PROVIDER_API_KEY")}
            type="password"
            placeholder={existingConfig?.PROVIDER_API_KEY_SET ? "(saved, leave blank to keep)" : "optional when using PatchHive Local AI"}
          />
        </div>
        <div style={S.field}>
          <label style={S.label}>PatchHive Local AI URL</label>
          <Input
            value={cfg.PATCHHIVE_AI_URL}
            onChange={set("PATCHHIVE_AI_URL")}
            placeholder="http://127.0.0.1:8787/v1"
          />
        </div>
        <div style={{ background:"#0d0d18", border:"1px solid #1c1c30", borderRadius:6, padding:"10px 12px", display:"flex", flexDirection:"column", gap:8 }}>
          <div style={{ display:"flex", alignItems:"center", gap:8 }}>
            <span style={{ fontSize:10, fontWeight:700, color:aiStatusColor }}>Local AI Gateway</span>
            <span style={{ fontSize:10, color:aiStatusColor }}>
              {!aiStatus?.configured ? "not configured" : aiStatus?.ok ? "connected" : "needs attention"}
            </span>
            <div style={{ flex:1 }} />
            <Btn onClick={loadAiStatus} color="#484868" style={{ fontSize:10, padding:"4px 10px" }}>
              {loadingAiStatus ? "Checking…" : "Refresh"}
            </Btn>
          </div>
          <div style={{ fontSize:10, color:"#484868" }}>
            {!aiStatus?.configured
              ? "Set PATCHHIVE_AI_URL to use your local Codex/Copilot gateway."
              : aiStatus?.error || aiStatus?.url || "PatchHive Local AI configured."}
          </div>
          {providerEntries.map(([name, provider]) => {
            const providerColor = provider.ok && provider.logged_in ? "#2a8a4a" : provider.ok ? "#c8922a" : "#c41e3a";
            const models = Array.isArray(provider.models) ? provider.models : [];
            return (
              <div key={name} style={{ border:"1px solid #1c1c30", borderRadius:5, padding:"8px 10px", background:"#10101e" }}>
                <div style={{ display:"flex", alignItems:"center", gap:8, marginBottom:4 }}>
                  <span style={{ fontSize:10, fontWeight:700, color:providerColor }}>{name}</span>
                  <span style={{ fontSize:10, color:providerColor }}>
                    {provider.ok && provider.logged_in ? "ready" : provider.ok ? "reachable" : "not ready"}
                  </span>
                </div>
                <div style={{ fontSize:10, color:"#484868" }}>
                  {provider.error || provider.bootstrap_hint || provider.auth_mode || "No extra details."}
                </div>
                {models.length > 0 && (
                  <div style={{ fontSize:10, color:"#7f7fa1", marginTop:4 }}>
                    Models: {models.slice(0, 5).join(", ")}{models.length > 5 ? ` +${models.length - 5} more` : ""}
                  </div>
                )}
              </div>
            );
          })}
        </div>
        <div style={S.field}>
          <label style={S.label}>Ollama Base URL</label>
          <Input value={cfg.OLLAMA_BASE_URL} onChange={set("OLLAMA_BASE_URL")} placeholder="http://localhost:11434" />
        </div>

        <Divider />
        <div style={{ fontSize:11, fontWeight:700, color:"#c41e3a" }}>Hunt Settings</div>
        <div style={{ display:"grid", gridTemplateColumns:"1fr 1fr", gap:10 }}>
          <div style={S.field}>
            <label style={S.label}>Cost Budget USD (0 = unlimited)</label>
            <Input value={cfg.COST_BUDGET_USD} onChange={set("COST_BUDGET_USD")} type="number" />
          </div>
          <div style={S.field}>
            <label style={S.label}>Min Smith Confidence %</label>
            <Input value={cfg.MIN_REVIEW_CONFIDENCE} onChange={set("MIN_REVIEW_CONFIDENCE")} type="number" />
          </div>
        </div>

        <Divider />
        <div style={{ fontSize:11, fontWeight:700, color:"#c41e3a" }}>Webhooks</div>
        <div style={S.field}>
          <label style={S.label}>Webhook Secret</label>
          <Input
            value={cfg.WEBHOOK_SECRET}
            onChange={set("WEBHOOK_SECRET")}
            type="password"
            placeholder={existingConfig?.WEBHOOK_SECRET_SET ? "(saved, leave blank to keep)" : "your-secret"}
          />
        </div>

        <Btn onClick={save} color="#c41e3a" style={{ alignSelf:"flex-start", minWidth:120 }}>
          {saved ? "✓ Saved" : "Save Config"}
        </Btn>
      </div>
    </div>
  );
}
