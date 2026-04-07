import { useState } from "react";
import { S, Input, Btn } from "../index.js";

export default function LoginPage({
  onLogin,
  icon = "🔱",
  title = "PatchHive",
  subtitle = "by PatchHive",
  storageKey = "patchhive_api_key",
  apiBase = "http://localhost:8000",
}) {
  const [key,     setKey]     = useState("");
  const [err,     setErr]     = useState("");
  const [loading, setLoading] = useState(false);

  const submit = async () => {
    if (!key.trim()) return;
    setLoading(true); setErr("");
    try {
      const r = await fetch(`${apiBase}/auth/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ api_key: key }),
      });
      if (r.ok) {
        localStorage.setItem(storageKey, key);
        onLogin(key);
      } else {
        setErr("Invalid API key.");
      }
    } catch {
      setErr("Cannot reach backend.");
    }
    setLoading(false);
  };

  return (
    <div style={{
      minHeight:"100vh", background:"var(--bg)",
      display:"flex", alignItems:"center", justifyContent:"center",
      fontFamily:"'SF Mono','Fira Mono',monospace",
    }}>
      <div style={{ width:360, padding:32, background:"var(--bg-panel)", border:"1px solid var(--border)", borderRadius:8 }}>
        <div style={{ fontSize:22, fontWeight:700, color:"var(--accent)", marginBottom:4, letterSpacing:"-0.02em" }}>
          {icon} {title}
        </div>
        <div style={{ fontSize:10, color:"var(--text-dim)", marginBottom:24 }}>{subtitle}</div>
        <div style={{ fontSize:11, color:"var(--text-input)", marginBottom:16 }}>Enter your API key to continue.</div>
        <Input value={key} onChange={setKey} placeholder="API key..." type="password" style={{ marginBottom:12 }} />
        {err && <div style={{ fontSize:11, color:"var(--accent)", marginBottom:10 }}>{err}</div>}
        <Btn onClick={submit} disabled={loading} style={{ width:"100%" }}>
          {loading ? "Authenticating..." : "Enter"}
        </Btn>
      </div>
    </div>
  );
}
