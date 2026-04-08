import { API } from "../config.js";
import { useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { S, Input, Sel, Btn, EmptyState, IssueRow } from "@patchhivehq/ui";

const LANGS = ["python","javascript","typescript","rust","go","java","ruby","cpp"];

export default function DryRunPanel({ agents, apiKey = "", onViewDiff }) {
  const [params, setParams] = useState({ language:"python", min_stars:50, max_repos:5, max_issues:10, search_query:"", concurrency:1, cost_budget_usd:0, retry_count:3 });
  const [running, setRunning] = useState(false);
  const [issues, setIssues] = useState([]);
  const [report, setReport] = useState("");
  const [logs, setLogs] = useState([]);
  const set = k => v => setParams(p => ({ ...p, [k]: v }));

  const fetch_ = createApiFetcher(apiKey);

  const run = async () => {
    setRunning(true); setIssues([]); setReport(""); setLogs([]);
    const res = await fetch_(`${API}/dry-run`, {
      method:"POST", headers:{ "Content-Type":"application/json" },
      body: JSON.stringify({ ...params, min_stars:+params.min_stars, max_repos:+params.max_repos, max_issues:+params.max_issues, labels:["bug"] }),
    });
    const reader = res.body.getReader(); const dec = new TextDecoder(); let buf = "";
    const pump = async () => {
      const { done, value } = await reader.read();
      if (done) { setRunning(false); return; }
      buf += dec.decode(value, { stream:true });
      const parts = buf.split("\n\n"); buf = parts.pop();
      for (const p of parts) {
        const em = p.match(/^event: (.+)/m); const dm = p.match(/^data: (.+)/m);
        if (em && dm) {
          const ev = em[1].trim(); const d = JSON.parse(dm[1]);
          if (ev === "issues")          setIssues(d.issues || []);
          if (ev === "dry_run_report")  setReport(d.report || "");
          if (ev === "agent_log")       setLogs(l => [...l.slice(-100), d]);
          if (ev === "done")            setRunning(false);
        }
      }
      pump();
    };
    pump();
  };

  return (
    <div style={{ display:"grid", gridTemplateColumns:"280px 1fr", gap:16 }}>
      <div style={{ display:"flex", flexDirection:"column", gap:10 }}>
        <div style={S.panel}>
          <div style={{ fontSize:12, fontWeight:700, color:"#d4d4e8", marginBottom:12 }}>◌ Dry Stalk</div>
          <div style={{ fontSize:10, color:"#484868", marginBottom:12 }}>Scans and scores without making any changes or opening PRs.</div>
          <div style={{ display:"flex", flexDirection:"column", gap:8 }}>
            <div style={S.field}>
              <label style={S.label}>Search Query</label>
              <Input value={params.search_query} onChange={set("search_query")} placeholder="or use language/stars below" />
            </div>
            <div style={S.field}>
              <label style={S.label}>Language</label>
              <Sel value={params.language} onChange={set("language")} opts={LANGS} />
            </div>
            <div style={{ display:"grid", gridTemplateColumns:"1fr 1fr", gap:8 }}>
              <div style={S.field}><label style={S.label}>Min Stars</label><Input value={params.min_stars} onChange={set("min_stars")} type="number" /></div>
              <div style={S.field}><label style={S.label}>Max Repos</label><Input value={params.max_repos} onChange={set("max_repos")} type="number" /></div>
              <div style={S.field}><label style={S.label}>Max Issues</label><Input value={params.max_issues} onChange={set("max_issues")} type="number" /></div>
            </div>
          </div>
        </div>
        <Btn onClick={run} disabled={running} color="#c8922a" style={{ width:"100%" }}>
          {running ? "Stalking…" : "◌ Begin Dry Stalk"}
        </Btn>
      </div>

      <div>
        {report && (
          <div style={{ ...S.panel, marginBottom:12 }}>
            <div style={{ fontSize:11, fontWeight:700, color:"#c8922a", marginBottom:8 }}>⚖ Analysis Report</div>
            <div style={{ fontSize:11, color:"#d4d4e8", lineHeight:1.6, whiteSpace:"pre-wrap" }}>{report}</div>
          </div>
        )}
        <div style={{ fontSize:12, fontWeight:700, color:"#d4d4e8", marginBottom:10 }}>◌ Would Target ({issues.length})</div>
        {issues.length === 0
          ? <EmptyState icon="◌" text="Run a dry stalk to preview targets." />
          : <div style={{ display:"flex", flexDirection:"column", gap:6 }}>
              {issues.map((iss, i) => <IssueRow key={i} issue={iss} onViewDiff={onViewDiff} />)}
            </div>
        }
      </div>
    </div>
  );
}
