import { API } from "../config.js";
// RepoListsPanel.jsx
import { useState, useEffect } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { S, Input, Btn, EmptyState, Tag } from "@patchhivehq/ui";

export function RepoListsPanel({ apiKey = "" }) {
  const [repos, setRepos] = useState([]);
  const [input, setInput] = useState("");
  const [type, setType] = useState("allowlist");
  const sections = [
    { key: "allowlist", label: "Allowlist", color: "#2a8a4a", empty: "No explicitly allowed repos." },
    { key: "denylist", label: "Denylist", color: "#c41e3a", empty: "No explicitly denied repos." },
    { key: "opt_out", label: "Opt-Out", color: "#c8922a", empty: "No opted-out repos." },
  ];
  const fetch_ = createApiFetcher(apiKey);

  const load = () => fetch_(`${API}/repo-lists`).then(r => r.json()).then(d => setRepos(d.repos || []));
  useEffect(load, []);

  const add = async () => {
    if (!input.trim()) return;
    await fetch_(`${API}/repo-lists`, { method:"POST", headers:{"Content-Type":"application/json"}, body: JSON.stringify({ repo: input.trim(), list_type: type }) });
    setInput(""); load();
  };

  const remove = async repo => {
    await fetch_(`${API}/repo-lists/${encodeURIComponent(repo)}`, { method:"DELETE" });
    load();
  };

  const grouped = {
    allowlist: repos.filter(r => r.list_type === "allowlist"),
    denylist: repos.filter(r => r.list_type === "denylist" || r.list_type === "blocklist"),
    opt_out: repos.filter(r => r.list_type === "opt_out"),
  };

  return (
    <div>
      <div style={{ fontSize:13, fontWeight:700, color:"#d4d4e8", marginBottom:16 }}>◎ Repo Lists</div>
      <div style={{ ...S.panel, marginBottom:16 }}>
        <div style={{ display:"flex", gap:8, marginBottom:8 }}>
          <Input value={input} onChange={setInput} placeholder="owner/repo" style={{ flex:1 }} />
          <select value={type} onChange={e => setType(e.target.value)} style={{ ...S.select, width:"auto" }}>
            <option value="allowlist">Allowlist</option>
            <option value="denylist">Denylist</option>
            <option value="opt_out">Opt-Out</option>
          </select>
          <Btn onClick={add} color="#c41e3a">Add</Btn>
        </div>
        <div style={{ fontSize:10, color:"#484868" }}>
          Allowlist: hunt only these repos. Denylist: never hunt these repos. Opt-Out: strongest exclusion and should be respected across PatchHive.
        </div>
      </div>

      {sections.map(section => {
        const list = grouped[section.key];
        return (
          <div key={section.key} style={{ marginBottom:16 }}>
            <div style={{ fontSize:11, color: section.color, fontWeight:700, marginBottom:8 }}>{section.label} ({list.length})</div>
            {list.length === 0
              ? <div style={{ fontSize:10, color:"#484868" }}>{section.empty}</div>
              : list.map(r => (
                  <div key={r.repo} style={{ display:"flex", alignItems:"center", gap:8, padding:"4px 0" }}>
                    <span style={{ fontSize:11, color:"#d4d4e8", flex:1 }}>{r.repo}</span>
                    <button onClick={() => remove(r.repo)} style={{ background:"transparent", border:"none", cursor:"pointer", color:"#484868", fontSize:14 }}>×</button>
                  </div>
                ))
            }
          </div>
        );
      })}
    </div>
  );
}

// SchedulesPanel.jsx
export function SchedulesPanel({ apiKey = "" }) {
  const [schedules, setSchedules] = useState([]);
  const [form, setForm] = useState({ cron_expr:"nightly", config_json:"{}" });
  const fetch_ = createApiFetcher(apiKey);

  const load = () => fetch_(`${API}/schedules`).then(r => r.json()).then(d => setSchedules(d.schedules || []));
  useEffect(load, []);

  const create = async () => {
    let cfg; try { cfg = JSON.parse(form.config_json); } catch { return; }
    await fetch_(`${API}/schedules`, { method:"POST", headers:{"Content-Type":"application/json"}, body: JSON.stringify({ cron_expr: form.cron_expr, config_json: cfg }) });
    load();
  };

  const del = async id => { await fetch_(`${API}/schedules/${id}`, { method:"DELETE" }); load(); };
  const toggle = async id => { await fetch_(`${API}/schedules/${id}/toggle`, { method:"PATCH" }); load(); };

  return (
    <div>
      <div style={{ fontSize:13, fontWeight:700, color:"#d4d4e8", marginBottom:16 }}>◎ Scheduled Hunts</div>
      <div style={{ ...S.panel, marginBottom:16 }}>
        <div style={{ fontSize:11, fontWeight:700, color:"#d4d4e8", marginBottom:10 }}>New Schedule</div>
        <div style={{ display:"flex", flexDirection:"column", gap:8 }}>
          <div style={S.field}>
            <label style={S.label}>Frequency</label>
            <select value={form.cron_expr} onChange={e => setForm(f => ({...f,cron_expr:e.target.value}))} style={S.select}>
              <option value="hourly">Hourly</option>
              <option value="nightly">Nightly</option>
              <option value="weekly">Weekly</option>
            </select>
          </div>
          <div style={S.field}>
            <label style={S.label}>Run Config (JSON)</label>
            <textarea value={form.config_json} onChange={e => setForm(f => ({...f,config_json:e.target.value}))}
              style={{ ...S.input, height:80, resize:"vertical", fontFamily:"monospace" }} />
          </div>
          <Btn onClick={create} color="#c41e3a" style={{ alignSelf:"flex-start" }}>Create Schedule</Btn>
        </div>
      </div>

      {schedules.length === 0
        ? <EmptyState icon="◌" text="No schedules. Create one above." />
        : schedules.map(s => (
            <div key={s.id} style={{ ...S.panel, marginBottom:8 }}>
              <div style={{ display:"flex", alignItems:"center", gap:10 }}>
                <span style={{ fontSize:11, color: s.enabled ? "#2a8a4a" : "#484868" }}>{s.enabled ? "●" : "○"}</span>
                <span style={{ fontSize:11, color:"#d4d4e8", fontWeight:600 }}>{s.cron_expr}</span>
                <span style={{ fontSize:10, color:"#484868", flex:1 }}>next: {s.next_run?.slice(0,16)}</span>
                <Btn onClick={() => toggle(s.id)} color={s.enabled ? "#484868" : "#2a8a4a"} style={{ fontSize:10 }}>{s.enabled ? "Pause" : "Resume"}</Btn>
                <Btn onClick={() => del(s.id)} color="#c41e3a" style={{ fontSize:10 }}>Delete</Btn>
              </div>
            </div>
          ))
      }
    </div>
  );
}

// WebhookPanel.jsx
export function WebhookPanel({ watchMode, onToggleWatch }) {
  const host = window.location.hostname;
  const url = `http://${host}:8000/webhook/github`;

  return (
    <div style={{ maxWidth:600 }}>
      <div style={{ fontSize:13, fontWeight:700, color:"#d4d4e8", marginBottom:16 }}>⚡ Webhooks & Watch Mode</div>

      <div style={{ ...S.panel, marginBottom:16 }}>
        <div style={{ fontSize:11, fontWeight:700, color:"#d4d4e8", marginBottom:10 }}>Watch Mode</div>
        <div style={{ fontSize:10, color:"#484868", marginBottom:12 }}>
          When enabled, newly labeled "bug" issues automatically trigger a hunt. Requires the GitHub webhook below to be configured.
        </div>
        <div style={{ display:"flex", alignItems:"center", gap:12 }}>
          <div style={{ fontSize:13, color: watchMode ? "#2a8a4a" : "#484868" }}>
            {watchMode ? "● Active" : "○ Inactive"}
          </div>
          <Btn onClick={onToggleWatch} color={watchMode ? "#484868" : "#2a8a4a"}>
            {watchMode ? "Disable Watch Mode" : "Enable Watch Mode"}
          </Btn>
        </div>
      </div>

      <div style={S.panel}>
        <div style={{ fontSize:11, fontWeight:700, color:"#d4d4e8", marginBottom:10 }}>GitHub Webhook Setup</div>
        <div style={{ fontSize:10, color:"#484868", marginBottom:12 }}>Configure this in your GitHub repo → Settings → Webhooks.</div>
        <div style={{ display:"flex", flexDirection:"column", gap:8 }}>
          {[
            ["Payload URL", url],
            ["Content type", "application/json"],
            ["Events", "Issues, Issue comments"],
          ].map(([label, val]) => (
            <div key={label} style={{ display:"flex", gap:8, alignItems:"center" }}>
              <span style={{ fontSize:10, color:"#484868", minWidth:100 }}>{label}</span>
              <code style={{ fontSize:10, color:"#d4d4e8", background:"#10101e", padding:"3px 8px", borderRadius:3, flex:1 }}>{val}</code>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// PRTrackingPanel.jsx
export function PRTrackingPanel({ apiKey = "" }) {
  const [prs, setPrs] = useState([]);
  const fetch_ = createApiFetcher(apiKey);

  const load = () => fetch_(`${API}/pr-tracking`).then(r => r.json()).then(d => setPrs(d.prs || []));
  useEffect(load, []);

  return (
    <div>
      <div style={{ display:"flex", alignItems:"center", marginBottom:12, gap:8 }}>
        <span style={{ fontSize:13, fontWeight:700, color:"#d4d4e8" }}>↗ PR Monitor</span>
        <div style={{ flex:1 }}/>
        <Btn onClick={load} color="#484868" style={{ fontSize:10 }}>↻ Refresh</Btn>
      </div>

      {prs.length === 0
        ? <EmptyState icon="↗" text="No PRs tracked yet." />
        : <div style={{ ...S.panel }}>
            <div style={{ display:"grid", gridTemplateColumns:"60px 1fr 100px 80px 80px", gap:10, fontSize:9, color:"#484868", fontWeight:700, letterSpacing:"0.08em", padding:"0 0 8px", borderBottom:"1px solid #1c1c30", marginBottom:8 }}>
              <div>PR</div><div>REPO</div><div>STATUS</div><div>REVIEW</div><div>MERGED</div>
            </div>
            {prs.map(pr => (
              <div key={`${pr.repo}-${pr.pr_number}`} style={{ display:"grid", gridTemplateColumns:"60px 1fr 100px 80px 80px", gap:10, fontSize:11, padding:"5px 0", borderBottom:"1px solid #10101e", alignItems:"center" }}>
                <div style={{ color:"#2a8a4a" }}>#{pr.pr_number}</div>
                <div style={{ color:"#d4d4e8", fontSize:10 }}>{pr.repo}</div>
                <div style={{ color: pr.state === "open" ? "#c8922a" : pr.state === "closed" ? "#484868" : "#d4d4e8", fontSize:10 }}>{pr.state}</div>
                <div style={{ fontSize:10, color:"#484868" }}>{pr.review_state || "—"}</div>
                <div style={{ fontSize:10, color: pr.merged ? "#2a8a4a" : "#484868" }}>{pr.merged ? "✓ merged" : "—"}</div>
              </div>
            ))}
          </div>
      }
    </div>
  );
}

// StartupChecksPanel.jsx
export function StartupChecksPanel({ apiKey = "" }) {
  const [checks, setChecks] = useState([]);
  const fetch_ = createApiFetcher(apiKey);

  useEffect(() => {
    fetch_(`${API}/startup/checks`)
      .then(r => r.json()).then(d => setChecks(d.checks || []));
  }, []);

  const color = l => ({ ok:"#2a8a4a", warn:"#c8922a", error:"#c41e3a" })[l] || "#484868";
  const icon  = l => ({ ok:"✓", warn:"⚠", error:"✗" })[l] || "◌";

  return (
    <div>
      <div style={{ fontSize:13, fontWeight:700, color:"#d4d4e8", marginBottom:16 }}>◎ Startup Checks</div>
      {checks.length === 0
        ? <EmptyState icon="◌" text="No checks loaded." />
        : <div style={{ display:"flex", flexDirection:"column", gap:6 }}>
            {checks.map((c, i) => (
              <div key={i} style={{ display:"flex", alignItems:"center", gap:10, padding:"8px 12px", background:"#0d0d18", border:`1px solid ${color(c.level)}33`, borderRadius:5 }}>
                <span style={{ color: color(c.level), fontSize:12 }}>{icon(c.level)}</span>
                <span style={{ fontSize:11, color:"#d4d4e8" }}>{c.msg}</span>
              </div>
            ))}
          </div>
      }
    </div>
  );
}
