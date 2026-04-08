import { S, Input, Sel, Btn, EmptyState, IssueRow } from "@patchhivehq/ui";

const LANGS = ["python","javascript","typescript","rust","go","java","ruby","cpp"];

export default function RunPanel({ running, onStart, params, setParams, issues, logs, agents, runStats, runCost, onViewDiff }) {
  const set = k => v => setParams(p => ({ ...p, [k]: v }));
  const issueList = Object.values(issues);
  const fixed   = issueList.filter(i => i.status === "fixed").length;
  const skipped = issueList.filter(i => i.status === "skipped" || i.status === "rejected").length;
  const active  = issueList.filter(i => i.status === "running").length;

  return (
    <div style={{ display:"grid", gridTemplateColumns:"300px 1fr", gap:16 }}>

      {/* Controls */}
      <div style={{ display:"flex", flexDirection:"column", gap:12 }}>
        <div style={{ ...S.panel }}>
          <div style={{ fontSize:12, fontWeight:700, color:"#d4d4e8", marginBottom:12 }}>🔱 Hunt Parameters</div>

          <div style={{ display:"flex", flexDirection:"column", gap:10 }}>
            <div style={S.field}>
              <label style={S.label}>Custom Search Query</label>
              <Input value={params.search_query} onChange={set("search_query")} placeholder="e.g. topic:cli language:rust stars:>100" />
            </div>
            <div style={S.field}>
              <label style={S.label}>Language (if no query)</label>
              <Sel value={params.language} onChange={set("language")} opts={LANGS} />
            </div>
            <div style={{ display:"grid", gridTemplateColumns:"1fr 1fr", gap:8 }}>
              <div style={S.field}>
                <label style={S.label}>Min Stars</label>
                <Input value={params.min_stars} onChange={set("min_stars")} type="number" />
              </div>
              <div style={S.field}>
                <label style={S.label}>Max Repos</label>
                <Input value={params.max_repos} onChange={set("max_repos")} type="number" />
              </div>
              <div style={S.field}>
                <label style={S.label}>Max Issues</label>
                <Input value={params.max_issues} onChange={set("max_issues")} type="number" />
              </div>
              <div style={S.field}>
                <label style={S.label}>Concurrency</label>
                <Input value={params.concurrency} onChange={set("concurrency")} type="number" />
              </div>
              <div style={S.field}>
                <label style={S.label}>Test Retries</label>
                <Input value={params.retry_count} onChange={set("retry_count")} type="number" />
              </div>
              <div style={S.field}>
                <label style={S.label}>Cost Budget $</label>
                <Input value={params.cost_budget_usd} onChange={set("cost_budget_usd")} type="number" placeholder="0 = unlimited" />
              </div>
            </div>
          </div>
        </div>

        <Btn onClick={onStart} disabled={running} color="#c41e3a" style={{ width:"100%", padding:"10px", fontSize:13, fontWeight:700 }}>
          {running ? "⚔ Hunt in Progress…" : "⚔ Begin Hunt"}
        </Btn>

        {/* Stats */}
        {(issueList.length > 0 || runStats) && (
          <div style={{ ...S.panel }}>
            <div style={{ fontSize:10, color:"#484868", fontWeight:700, letterSpacing:"0.08em", marginBottom:8 }}>HUNT STATS</div>
            <div style={{ display:"grid", gridTemplateColumns:"1fr 1fr", gap:6, fontSize:11 }}>
              <div style={{ color:"#484868" }}>Total targets</div>
              <div style={{ color:"#d4d4e8", textAlign:"right" }}>{issueList.length}</div>
              <div style={{ color:"#484868" }}>Kills confirmed</div>
              <div style={{ color:"#2a8a4a", textAlign:"right" }}>{fixed}</div>
              <div style={{ color:"#484868" }}>Active</div>
              <div style={{ color:"#c8922a", textAlign:"right" }}>{active}</div>
              <div style={{ color:"#484868" }}>Escaped</div>
              <div style={{ color:"#484868", textAlign:"right" }}>{skipped}</div>
              {runCost > 0 && <>
                <div style={{ color:"#484868" }}>Run cost</div>
                <div style={{ color:"#c8922a", textAlign:"right" }}>${runCost.toFixed(4)}</div>
              </>}
            </div>
          </div>
        )}
      </div>

      {/* Issue list */}
      <div>
        <div style={{ fontSize:12, fontWeight:700, color:"#d4d4e8", marginBottom:12 }}>
          ◎ Target Queue {issueList.length > 0 && `(${issueList.length})`}
        </div>
        {issueList.length === 0
          ? <EmptyState icon="◌" text="No targets queued. Configure and begin a hunt." />
          : <div style={{ display:"flex", flexDirection:"column", gap:6 }}>
              {issueList.map(issue => (
                <IssueRow key={issue.id} issue={issue} onViewDiff={onViewDiff} />
              ))}
            </div>
        }
      </div>
    </div>
  );
}
