export default function DiffViewer({ diff, onClose }) {
  if (!diff) return null;

  const lines = diff.split("\n");

  return (
    <div style={{
      position:"fixed", inset:0, background:"#080810cc", zIndex:1000,
      display:"flex", alignItems:"center", justifyContent:"center", padding:24,
    }} onClick={onClose}>
      <div style={{
        background:"#0d0d18", border:"1px solid #1c1c30", borderRadius:8,
        width:"100%", maxWidth:900, maxHeight:"80vh", overflow:"hidden",
        display:"flex", flexDirection:"column",
      }} onClick={e => e.stopPropagation()}>
        <div style={{ display:"flex", alignItems:"center", padding:"12px 16px", borderBottom:"1px solid #1c1c30" }}>
          <span style={{ fontSize:12, color:"#c41e3a", fontWeight:700, fontFamily:"monospace" }}>⚔ patch diff</span>
          <div style={{ flex:1 }} />
          <button onClick={onClose} style={{
            background:"transparent", border:"none", cursor:"pointer",
            color:"#484868", fontSize:18, lineHeight:1,
          }}>×</button>
        </div>
        <div style={{ overflow:"auto", padding:"12px 0", fontFamily:"monospace", fontSize:11 }}>
          {lines.map((line, i) => {
            const color = line.startsWith("+") ? "#2a8a4a"
                        : line.startsWith("-") ? "#c41e3a"
                        : line.startsWith("@@") ? "#7b2d8b"
                        : "#484868";
            const bg = line.startsWith("+") ? "#2a8a4a11"
                     : line.startsWith("-") ? "#c41e3a11"
                     : "transparent";
            return (
              <div key={i} style={{ color, background:bg, padding:"1px 16px", whiteSpace:"pre-wrap", wordBreak:"break-all" }}>
                {line || " "}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
