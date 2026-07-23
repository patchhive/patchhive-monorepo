import { useEffect, useState } from "react";
import { Bot, LoaderCircle, Save, Sparkles, Trash2, Users } from "lucide-react";
import { useProviderModelDiscovery } from "@patchhivehq/ai-models/model-discovery";
import { GuidanceNotice, ProgressiveList, V3_TEXT } from "@patchhivehq/ui-v3";
import { Chip, readResponse, statusTone } from "./shared.jsx";

const ROLES = ["scout", "judge", "reaper", "smith", "gatekeeper"];
const PROVIDERS = ["openai", "anthropic", "gemini", "groq", "openrouter", "custom", "ollama"];
const DEFAULT_MODELS = { openai: "gpt-5.4-mini", anthropic: "claude-sonnet-4-6", gemini: "gemini-2.5-pro", groq: "llama-3.3-70b-versatile", openrouter: "openrouter/free", custom: "gpt-4.1-mini", ollama: "llama3.2" };
const OPENROUTER_BASE_URL = "https://openrouter.ai/api/v1";
const PROVIDER_LABELS = { openai: "OpenAI / Compatible", anthropic: "Anthropic", gemini: "Gemini", groq: "Groq", openrouter: "OpenRouter", custom: "Custom OpenAI-Compatible", ollama: "Ollama" };

function defaultBaseUrlForProvider(provider) {
  return provider === "openrouter" ? OPENROUTER_BASE_URL : "";
}

function nextProviderState(current, provider) {
  const priorDefault = defaultBaseUrlForProvider(current.provider);
  const keepCustomBase = current.base_url && current.base_url !== priorDefault;
  return {
    ...current,
    provider,
    model: DEFAULT_MODELS[provider] || "",
    base_url: defaultBaseUrlForProvider(provider) || (keepCustomBase ? current.base_url : ""),
  };
}

function blankAgent(role = "scout") {
  return { id: `${role}-${crypto.randomUUID().slice(0, 8)}`, name: `PatchHive ${role[0].toUpperCase()}${role.slice(1)}`, role, provider: "openai", model: DEFAULT_MODELS.openai, base_url: "", api_key: "", bot_token: "", bot_user: "", status: "idle", current_task: "", stats: { fixed: 0, skipped: 0, errors: 0, cost: 0 } };
}

function Input({ label, onChange, placeholder, type = "text", value }) {
  return <label className="block"><span className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>{label}</span><input className={`surface mt-1.5 h-10 w-full rounded-xl bg-transparent px-3 text-[12px] outline-none ${V3_TEXT.strong}`} onChange={(event) => onChange(event.target.value)} placeholder={placeholder} type={type} value={value || ""}/></label>;
}

export default function SquadPanel({ agents, apiBase, authToken, config, cooldowns, fetcher, onError, onRefresh, presets }) {
  const [draft, setDraft] = useState(agents);
  const [presetName, setPresetName] = useState("");
  const [selectedPreset, setSelectedPreset] = useState("");
  const [defaults, setDefaults] = useState({ provider: "openai", model: DEFAULT_MODELS.openai, base_url: "", api_key: "", bot_token: "", bot_user: "" });
  const [agentReadyOnly, setAgentReadyOnly] = useState(true);
  const [freeOnly, setFreeOnly] = useState(false);
  const [working, setWorking] = useState(false);
  useEffect(() => setDraft(agents), [agents]);

  const modelDiscovery = useProviderModelDiscovery({
    apiBase,
    authToken,
    provider: defaults.provider,
    model: defaults.model,
    onModelChange: (model) => setDefaults((current) => ({ ...current, model })),
    providerKey: defaults.api_key,
    baseUrl: defaults.base_url,
    fallbackModels: config?.providers,
    agentReadyOnly,
    freeOnly,
    localGatewayConfigured: Boolean(config?.PATCHHIVE_AI_URL || config?.AI_LOCAL_STATUS?.ok || config?.AI_LOCAL_STATUS?.status === "ok"),
    globalKeyConfigured: Boolean(config?.PROVIDER_API_KEY_SET),
    autoLoad: false,
  });

  function updateAgent(id, key, value) {
    setDraft((current) => current.map((agent) => agent.id === id ? (key === "provider" ? nextProviderState(agent, value) : { ...agent, [key]: value }) : agent));
  }

  async function perform(action, fallback) {
    setWorking(true);
    try { await action(); await onRefresh(); } catch (error) { onError(error.message || fallback); } finally { setWorking(false); }
  }

  async function saveTeam(next = draft) {
    await readResponse(await fetcher(`${apiBase}/agents`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ agents: next.map(({ api_key_set: _apiKeySet, bot_token_set: _botTokenSet, ...agent }) => agent) }) }), "Could not save squad");
  }

  function applyDefaults() {
    setDraft((current) => current.map((agent) => ({ ...agent, provider: defaults.provider, model: defaults.model || DEFAULT_MODELS[defaults.provider] || agent.model, ...(defaults.base_url ? { base_url: defaults.base_url } : {}), ...(defaults.api_key ? { api_key: defaults.api_key } : {}), ...(defaults.bot_token ? { bot_token: defaults.bot_token } : {}), ...(defaults.bot_user ? { bot_user: defaults.bot_user } : {}) })));
  }

  const missingRoles = ROLES.filter((role) => !draft.some((agent) => agent.role === role));
  return <div className="mx-auto max-w-[1440px] space-y-6 px-3 py-6 sm:px-6">
    <section className="surface p-6 sm:p-8"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Users size={12}/> PatchHive squad</div><h1 className={`mt-2 font-display text-[42px] font-semibold ${V3_TEXT.strong}`}>Agents, models, and credentials.</h1><p className={`mt-3 max-w-3xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>Configure the active multi-agent team, test its provider path, reuse encrypted presets, and inspect cooldowns before authorizing a mission.</p><div className="mt-5 flex flex-wrap gap-2"><Chip tone={missingRoles.length ? "warn" : "ok"}>{draft.length} agents</Chip><Chip tone={missingRoles.length ? "warn" : "ok"}>{missingRoles.length ? `missing ${missingRoles.join(", ")}` : "all roles covered"}</Chip><Chip tone={Object.keys(cooldowns).length ? "warn" : "ok"}>{Object.keys(cooldowns).length ? `${Object.keys(cooldowns).length} cooldowns` : "no cooldowns"}</Chip></div></section>

    <section className="grid gap-6 lg:grid-cols-2">
      <article className="surface p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Sparkles size={12}/> Provider defaults</div>
        <div className="mt-5 grid gap-3 sm:grid-cols-2">
          <label>
            <span className={`text-[9px] uppercase tracking-wider ${V3_TEXT.mute}`}>Provider</span>
            <select className={`surface-inset mt-1.5 h-10 w-full rounded-xl bg-transparent px-3 text-[12px] ${V3_TEXT.strong}`} value={defaults.provider} onChange={(event) => setDefaults((current) => nextProviderState(current, event.target.value))}>
              {PROVIDERS.map((provider) => <option key={provider} value={provider}>{PROVIDER_LABELS[provider] || provider}</option>)}
            </select>
          </label>
          <label>
            <span className={`text-[9px] uppercase tracking-wider ${V3_TEXT.mute}`}>Model picker</span>
            <select className={`surface-inset mt-1.5 h-10 w-full rounded-xl bg-transparent px-3 text-[12px] ${V3_TEXT.strong}`} disabled={modelDiscovery.loading || !modelDiscovery.models.length} onChange={(event) => setDefaults((current) => ({ ...current, model: event.target.value }))} value={modelDiscovery.models.includes(defaults.model) ? defaults.model : ""}>
              {!modelDiscovery.models.includes(defaults.model) ? <option value="">Manual model below</option> : null}
              {modelDiscovery.models.map((model) => <option key={model} value={model}>{model}</option>)}
            </select>
          </label>
          <Input label="Base URL" onChange={(value) => setDefaults((current) => ({ ...current, base_url: value }))} placeholder="OpenAI-compatible endpoint" value={defaults.base_url}/>
          <Input label="Provider API key" onChange={(value) => setDefaults((current) => ({ ...current, api_key: value }))} placeholder="Leave blank to preserve/fallback" type="password" value={defaults.api_key}/>
          <Input label="Manual model" onChange={(value) => setDefaults((current) => ({ ...current, model: value }))} placeholder="Type any model id" value={defaults.model}/>
          <Input label="Bot token override" onChange={(value) => setDefaults((current) => ({ ...current, bot_token: value }))} placeholder="Optional per-agent token" type="password" value={defaults.bot_token}/>
          <Input label="Bot user override" onChange={(value) => setDefaults((current) => ({ ...current, bot_user: value }))} placeholder="patchhive" value={defaults.bot_user}/>
        </div>
        <div className="mt-4 flex flex-wrap items-center gap-2">
          <button className="surface-inset h-9 rounded-full px-4 text-[11px]" disabled={working || modelDiscovery.loading} onClick={() => modelDiscovery.loadModels({ includeProviderKey: true })} type="button">{modelDiscovery.loading ? "Discovering…" : "Discover models"}</button>
          <button className="surface-inset h-9 rounded-full px-4 text-[11px]" disabled={working || modelDiscovery.testing || !defaults.model} onClick={modelDiscovery.testModel} type="button">{modelDiscovery.testing ? "Testing…" : "Test model"}</button>
          <button className="h-9 rounded-full px-4 text-[11px] font-semibold text-white" disabled={working || !defaults.model} onClick={applyDefaults} style={{ backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))" }} type="button">Apply to draft</button>
          <label className="surface-inset inline-flex h-9 cursor-pointer items-center gap-2 rounded-full px-3 text-[11px]">
            <input checked={agentReadyOnly} onChange={(event) => setAgentReadyOnly(event.target.checked)} type="checkbox"/>
            Agent-ready only
          </label>
          <label className="surface-inset inline-flex h-9 cursor-pointer items-center gap-2 rounded-full px-3 text-[11px]">
            <input checked={freeOnly} onChange={(event) => setFreeOnly(event.target.checked)} type="checkbox"/>
            Free only
          </label>
          <Chip>{modelDiscovery.models.length} selectable models</Chip>
        </div>
        <div className={`mt-4 space-y-1 text-[11px] ${V3_TEXT.mute}`}>
          {modelDiscovery.statusText ? <div>{modelDiscovery.statusText}</div> : null}
          {modelDiscovery.filteredStatusText ? <div>{modelDiscovery.filteredStatusText}</div> : null}
          {modelDiscovery.agentFilteredStatusText ? <div>{modelDiscovery.agentFilteredStatusText}</div> : null}
          {modelDiscovery.freeFilteredStatusText ? <div>{modelDiscovery.freeFilteredStatusText}</div> : null}
          {modelDiscovery.testStatusText ? <div>{modelDiscovery.testStatusText}</div> : null}
        </div>
      </article>
      <article className="surface p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Save size={12}/> Team presets</div>
        <div className="mt-5 grid gap-3 sm:grid-cols-2">
          <Input label="Preset name" onChange={setPresetName} placeholder="trusted-rust-squad" value={presetName}/>
          <label><span className={`text-[9px] uppercase tracking-wider ${V3_TEXT.mute}`}>Saved presets</span><select className={`surface-inset mt-1.5 h-10 w-full rounded-xl bg-transparent px-3 text-[12px] ${V3_TEXT.strong}`} onChange={(event) => setSelectedPreset(event.target.value)} value={selectedPreset}><option value="">No preset selected</option>{presets.map((preset) => <option key={preset.name} value={preset.name}>{preset.name}</option>)}</select></label>
        </div>
        <div className="mt-4 flex flex-wrap gap-2"><button className="h-9 rounded-full px-4 text-[11px] font-semibold text-white disabled:opacity-40" disabled={working || !presetName.trim() || !draft.length} onClick={() => perform(async () => readResponse(await fetcher(`${apiBase}/presets`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ name: presetName.trim(), agents: draft.map(({ api_key_set: _a, bot_token_set: _b, ...agent }) => agent) }) }), "Could not save preset"), "Could not save preset")} style={{ backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))" }} type="button">Save preset</button><button className="surface-inset h-9 rounded-full px-4 text-[11px] disabled:opacity-40" disabled={working || !selectedPreset} onClick={() => perform(async () => readResponse(await fetcher(`${apiBase}/presets/${encodeURIComponent(selectedPreset)}/load`, { method: "POST" }), "Could not load preset"), "Could not load preset")} type="button">Activate preset</button><button className="surface-inset h-9 rounded-full px-4 text-[11px] text-red-700 disabled:opacity-40 dark:text-red-300" disabled={working || !selectedPreset} onClick={() => perform(async () => readResponse(await fetcher(`${apiBase}/presets/${encodeURIComponent(selectedPreset)}`, { method: "DELETE" }), "Could not delete preset"), "Could not delete preset")} type="button">Delete</button></div>
        <GuidanceNotice label="Secret storage">Browser responses expose only configured flags. Provider keys and bot-token overrides stay encrypted in SQLite when a stable RepoReaper or suite encryption key is configured.</GuidanceNotice>
      </article>
    </section>

    <section className="surface p-6"><div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between"><div><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Bot size={12}/> Active team draft</div><h2 className={`mt-2 font-display text-[28px] font-semibold ${V3_TEXT.strong}`}>{draft.length} configured agents</h2></div><div className="flex flex-wrap gap-2"><button className="surface-inset h-9 rounded-full px-4 text-[11px]" disabled={working} onClick={() => setDraft(ROLES.map(blankAgent))} type="button">Recruit starter team</button><button className="surface-inset h-9 rounded-full px-4 text-[11px]" disabled={working} onClick={() => setDraft((current) => [...current, blankAgent(missingRoles[0] || "reaper")])} type="button">Add agent</button><button className="h-9 rounded-full px-4 text-[11px] font-semibold text-white disabled:opacity-40" disabled={working || !draft.length} onClick={() => perform(() => saveTeam(), "Could not save squad")} style={{ backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))" }} type="button">{working ? "Saving…" : "Save active squad"}</button></div></div><div className="mt-5"><ProgressiveList initialCount={6} batchCount={20} itemLabel="agents" items={draft} empty={<div className={`surface-inset rounded-xl p-10 text-center text-[12px] ${V3_TEXT.mute}`}>No agents configured.</div>} renderItem={(agent) => <article className="surface-inset rounded-xl p-4" key={agent.id}><div className="flex flex-col gap-4 lg:flex-row lg:items-start"><div className="grid flex-1 gap-3 sm:grid-cols-2 lg:grid-cols-4"><Input label="Name" onChange={(value) => updateAgent(agent.id, "name", value)} value={agent.name}/><label><span className={`text-[9px] uppercase tracking-wider ${V3_TEXT.mute}`}>Role</span><select className={`surface mt-1.5 h-10 w-full rounded-xl bg-transparent px-3 text-[12px] ${V3_TEXT.strong}`} value={agent.role} onChange={(event) => updateAgent(agent.id, "role", event.target.value)}>{ROLES.map((role) => <option key={role}>{role}</option>)}</select></label><label><span className={`text-[9px] uppercase tracking-wider ${V3_TEXT.mute}`}>Provider</span><select className={`surface mt-1.5 h-10 w-full rounded-xl bg-transparent px-3 text-[12px] ${V3_TEXT.strong}`} value={agent.provider} onChange={(event) => updateAgent(agent.id, "provider", event.target.value)}>{PROVIDERS.map((provider) => <option key={provider}>{provider}</option>)}</select></label><Input label="Model" onChange={(value) => updateAgent(agent.id, "model", value)} value={agent.model}/><Input label="Base URL" onChange={(value) => updateAgent(agent.id, "base_url", value)} value={agent.base_url}/><Input label={`Provider key${agent.api_key_set ? " · configured" : ""}`} onChange={(value) => updateAgent(agent.id, "api_key", value)} placeholder={agent.api_key_set ? "Leave blank to preserve" : "Optional override"} type="password" value={agent.api_key}/><Input label={`Bot token${agent.bot_token_set ? " · configured" : ""}`} onChange={(value) => updateAgent(agent.id, "bot_token", value)} placeholder={agent.bot_token_set ? "Leave blank to preserve" : "Optional override"} type="password" value={agent.bot_token}/><Input label="Bot user" onChange={(value) => updateAgent(agent.id, "bot_user", value)} value={agent.bot_user}/></div><div className="flex shrink-0 items-center gap-2"><Chip tone={statusTone(agent.status)}>{agent.status || "idle"}</Chip><button aria-label={`Remove ${agent.name}`} className="surface grid h-9 w-9 place-items-center rounded-full text-red-700 dark:text-red-300" onClick={() => setDraft((current) => current.filter((item) => item.id !== agent.id))} type="button"><Trash2 size={13}/></button></div></div></article>}/></div></section>

    {Object.keys(cooldowns).length ? <section className="surface p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><LoaderCircle size={12}/> Provider cooldowns</div><div className="mt-4 flex flex-wrap gap-2">{Object.entries(cooldowns).map(([provider, until]) => <button className="surface-inset rounded-full px-3 py-2 text-[11px]" key={provider} onClick={() => perform(async () => readResponse(await fetcher(`${apiBase}/cooldowns/${encodeURIComponent(provider)}`, { method: "DELETE" }), "Could not clear cooldown"), "Could not clear cooldown")} type="button">{provider} · {String(until)} · clear</button>)}</div></section> : null}
  </div>;
}
