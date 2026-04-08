// primitives.jsx — PatchHive shared UI primitives
// Import from @patchhivehq/ui in any product

// ── Style constants ────────────────────────────────────────────────────────────

export const S = {
  field:  { display:"flex", flexDirection:"column", gap:4 },
  label:  { fontSize:9, color:"var(--text-muted)", textTransform:"uppercase", letterSpacing:"0.1em", fontWeight:600 },
  input:  { background:"var(--bg-input)", border:"1px solid var(--border-in)", borderRadius:4,
            color:"var(--text-input)", padding:"6px 9px", fontFamily:"inherit",
            fontSize:12, width:"100%", boxSizing:"border-box" },
  select: { background:"var(--bg-input)", border:"1px solid var(--border-in)", borderRadius:4,
            color:"var(--text-input)", padding:"6px 9px", fontFamily:"inherit",
            fontSize:12, width:"100%", boxSizing:"border-box" },
  panel:  { background:"var(--bg-panel)", border:"1px solid var(--border)", borderRadius:8, padding:16 },
};

// ── Input ──────────────────────────────────────────────────────────────────────

export function Input({ value, onChange, placeholder, type="text", style={} }) {
  return (
    <input
      type={type} value={value}
      onChange={e => onChange(e.target.value)}
      placeholder={placeholder}
      style={{ ...S.input, ...style }}
    />
  );
}

// ── Select ─────────────────────────────────────────────────────────────────────

export function Sel({ value, onChange, opts, style={} }) {
  return (
    <select value={value} onChange={e => onChange(e.target.value)} style={{ ...S.select, ...style }}>
      {opts.map(o => <option key={o.v ?? o} value={o.v ?? o}>{o.l ?? o}</option>)}
    </select>
  );
}

// ── Button ─────────────────────────────────────────────────────────────────────

export function Btn({ onClick, children, color="var(--accent)", style={}, disabled=false }) {
  return (
    <button onClick={onClick} disabled={disabled} style={{
      border:"none", borderRadius:4, cursor:disabled?"default":"pointer",
      fontFamily:"inherit", fontSize:11, padding:"6px 14px", fontWeight:700,
      letterSpacing:"0.04em", transition:"all 0.15s",
      background: disabled ? "var(--border)" : color + "22",
      color:       disabled ? "var(--text-dim)" : color,
      outline:     `1px solid ${disabled ? "var(--border)" : color + "55"}`,
      opacity:     disabled ? 0.5 : 1,
      ...style,
    }}>
      {children}
    </button>
  );
}

// ── Confidence bar ─────────────────────────────────────────────────────────────

export function ConfidenceBar({ value, size="md" }) {
  const color = value >= 75 ? "var(--green)" : value >= 50 ? "var(--gold)" : "var(--accent)";
  const h = size === "sm" ? 3 : 6;
  return (
    <div style={{ display:"flex", alignItems:"center", gap:6 }}>
      <div style={{ flex:1, height:h, background:"var(--border)", borderRadius:2, overflow:"hidden" }}>
        <div style={{ width:`${value}%`, height:"100%", background:color, borderRadius:2, transition:"width 0.3s" }} />
      </div>
      <span style={{ fontSize:9, color, fontWeight:700, minWidth:28 }}>{value}%</span>
    </div>
  );
}

// ── Score badge ────────────────────────────────────────────────────────────────

export function ScoreBadge({ score }) {
  const color = score >= 70 ? "var(--green)" : score >= 40 ? "var(--gold)" : "var(--accent)";
  return (
    <span style={{ fontSize:9, color, border:`1px solid ${color}55`, borderRadius:2,
                   padding:"1px 5px", fontWeight:700, flexShrink:0 }}>
      {score}/100
    </span>
  );
}

// ── Status dot ─────────────────────────────────────────────────────────────────

export function StatusDot({ status }) {
  const color = status==="working" ? "var(--accent)" : status==="idle" ? "var(--text-muted)" : "var(--green)";
  return (
    <span style={{
      display:"inline-block", width:6, height:6, borderRadius:"50%", background:color,
      animation: status==="working" ? "pulse 1s infinite" : "none", flexShrink:0,
    }} />
  );
}

// ── Tag ────────────────────────────────────────────────────────────────────────

export function Tag({ children, color="var(--text-dim)" }) {
  return (
    <span style={{ fontSize:9, color, border:`1px solid ${color}55`, borderRadius:2, padding:"1px 5px", fontWeight:600 }}>
      {children}
    </span>
  );
}

// ── Divider ────────────────────────────────────────────────────────────────────

export function Divider() {
  return <div style={{ borderTop:"1px solid var(--border)", margin:"12px 0" }} />;
}

// ── Empty state ────────────────────────────────────────────────────────────────

export function EmptyState({ icon="◌", text }) {
  return (
    <div style={{ textAlign:"center", padding:"48px 24px", color:"var(--text-muted)" }}>
      <div style={{ fontSize:32, marginBottom:8, opacity:0.3 }}>{icon}</div>
      <div style={{ fontSize:11 }}>{text}</div>
    </div>
  );
}

// ── PatchHive header ───────────────────────────────────────────────────────────
// Shared header shell — products pass their own title/icon/children

export function PatchHiveHeader({ icon, title, version, phase, phaseLabel, phaseIcon, running, children }) {
  return (
    <div style={{
      borderBottom:"1px solid var(--border)", padding:"10px 24px",
      display:"flex", alignItems:"center", gap:12,
      position:"sticky", top:0, background:"var(--bg)", zIndex:100,
    }}>
      <div style={{ fontSize:16, fontWeight:700, color:"var(--accent)", letterSpacing:"-0.02em" }}>
        {icon} {title}
      </div>
      {version && (
        <div style={{ fontSize:9, color:"var(--text-dim)", border:"1px solid var(--border)", borderRadius:3, padding:"1px 5px" }}>
          {version}
        </div>
      )}
      {phase && phaseLabel && phaseIcon && (
        <div style={{ display:"flex", gap:4, marginLeft:4 }}>
          {Object.keys(phaseLabel).map(p => (
            <div key={p} style={{
              padding:"2px 8px", borderRadius:20, fontSize:9, fontWeight:600,
              background: p===phase ? "var(--accent)22" : "transparent",
              color:      p===phase ? "var(--accent)" : "var(--text-muted)",
              border:     p===phase ? "1px solid var(--accent)44" : "1px solid transparent",
            }}>
              {phaseIcon[p]} {phaseLabel[p]}
            </div>
          ))}
        </div>
      )}
      <div style={{ flex:1 }} />
      {children}
      {running && <div style={{ width:7, height:7, borderRadius:"50%", background:"var(--accent)", animation:"pulse 1s infinite" }} />}
    </div>
  );
}

// ── PatchHive footer ───────────────────────────────────────────────────────────

export function PatchHiveFooter({ product }) {
  return (
    <div style={{ borderTop:"1px solid var(--border)", padding:"10px 24px", textAlign:"center", fontSize:9, color:"var(--text-muted)" }}>
      {product} by PatchHive
    </div>
  );
}

// ── Tab bar ────────────────────────────────────────────────────────────────────

export function TabBar({ tabs, active, onChange }) {
  return (
    <div style={{ display:"flex", borderBottom:"1px solid var(--border)", padding:"0 24px", overflowX:"auto" }}>
      {tabs.map(t => (
        <button key={t.id} onClick={() => onChange(t.id)} style={{
          background:"transparent", border:"none", cursor:"pointer",
          fontFamily:"inherit", fontSize:11, padding:"9px 14px", flexShrink:0,
          color: active===t.id ? "var(--accent)" : "var(--text-dim)",
          borderBottom: `2px solid ${active===t.id ? "var(--accent)" : "transparent"}`,
        }}>
          {t.label}
        </button>
      ))}
    </div>
  );
}

// ── Utilities ──────────────────────────────────────────────────────────────────

export function timeAgo(d) {
  if (!d) return "—";
  const m = Math.floor((Date.now() - new Date(d)) / 60000);
  if (m < 1)  return "just now";
  if (m < 60) return `${m}m ago`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ago`;
  return `${Math.floor(h / 24)}d ago`;
}
