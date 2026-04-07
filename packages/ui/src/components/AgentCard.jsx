import { ROLE_META, PROVIDERS, StatusDot, Tag, ConfidenceBar } from "../index.js";

export default function AgentCard({ agent, onRemove, compact=false }) {
  const role = ROLE_META[agent.role] || { label: agent.role, icon: "◎", color: "#888" };
  const prov = PROVIDERS[agent.provider] || { label: agent.provider, color: "#888" };
  const stats = agent.stats || {};
  const total = (stats.fixed||0) + (stats.skipped||0) + (stats.errors||0);
  const fixRate = total > 0 ? Math.round(100 * (stats.fixed||0) / total) : 0;

  return (
    <div style={{
      background:"#0d0d18", border:`1px solid ${agent.status==="working" ? role.color+"55" : "#1c1c30"}`,
      borderRadius:6, padding: compact ? "8px 12px" : 14,
      transition:"border-color 0.2s",
    }}>
      <div style={{ display:"flex", alignItems:"center", gap:8, marginBottom: compact ? 0 : 8 }}>
        <StatusDot status={agent.status} />
        <span style={{ fontSize:13, color: role.color, fontWeight:700 }}>{role.icon}</span>
        <span style={{ fontSize:12, color:"#d4d4e8", fontWeight:600, flex:1 }}>{agent.name}</span>
        <Tag color={prov.color}>{prov.label}</Tag>
        <Tag color={role.color}>{role.label}</Tag>
        {onRemove && (
          <button onClick={() => onRemove(agent.id)} style={{
            background:"transparent", border:"none", cursor:"pointer",
            color:"#484868", fontSize:14, padding:"0 2px",
          }}>×</button>
        )}
      </div>

      {!compact && (
        <>
          <div style={{ fontSize:10, color:"#484868", marginBottom:8 }}>
            {agent.model} · {agent.current_task || role.desc}
          </div>
          {agent.status === "working" && (
            <div style={{ fontSize:10, color: role.color, marginBottom:8, animation:"pulse 1.5s infinite" }}>
              {agent.current_task || "Working…"}
            </div>
          )}
          <div style={{ display:"flex", gap:12, fontSize:10, color:"#484868" }}>
            <span style={{ color:"#2a8a4a" }}>✓ {stats.fixed||0} kills</span>
            <span>⊘ {stats.skipped||0} escaped</span>
            <span style={{ color:"#c41e3a" }}>✗ {stats.errors||0} errors</span>
            {total > 0 && <span style={{ color:"#c8922a" }}>{fixRate}% rate</span>}
            {(stats.cost||0) > 0 && <span>${(stats.cost||0).toFixed(4)}</span>}
          </div>
        </>
      )}
    </div>
  );
}
