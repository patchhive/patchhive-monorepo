import { useEffect, useState } from "react";
import { applyTheme } from "@patchhivehq/ui";
import {
  ProductAppFrame,
  ProductSessionGate,
  useApiFetcher,
  useApiKeyAuth,
} from "@patchhivehq/product-shell";
import { API } from "./config.js";
import OverviewPanel from "./panels/OverviewPanel.jsx";
import ProductsPanel from "./panels/ProductsPanel.jsx";
import SettingsPanel from "./panels/SettingsPanel.jsx";
import ChecksPanel from "./panels/ChecksPanel.jsx";
import SetupPanel from "./panels/SetupPanel.jsx";

const TABS = [
  { id: "setup", label: "Launch" },
  { id: "overview", label: "Command" },
  { id: "products", label: "Fleet" },
  { id: "settings", label: "Policy" },
  { id: "checks", label: "Diagnostics" },
];

const shellBackdrop = {
  background:
    "radial-gradient(circle at 12% 8%, color-mix(in srgb, var(--accent) 20%, transparent) 0, transparent 34%), radial-gradient(circle at 85% 18%, color-mix(in srgb, var(--blue) 18%, transparent) 0, transparent 30%), linear-gradient(135deg, color-mix(in srgb, var(--bg) 86%, #02080b) 0%, #061015 100%)",
};

const commandDeckStyle = {
  position: "relative",
  overflow: "hidden",
  display: "grid",
  gap: 16,
  padding: 18,
  border: "1px solid color-mix(in srgb, var(--accent) 32%, var(--border))",
  borderRadius: 18,
  background:
    "linear-gradient(135deg, color-mix(in srgb, var(--bg-panel) 78%, #061317) 0%, color-mix(in srgb, var(--bg-panel) 86%, #0d1f25) 52%, color-mix(in srgb, var(--blue) 16%, var(--bg-panel)) 100%)",
  boxShadow: "0 18px 50px rgba(0,0,0,0.24), inset 0 1px 0 rgba(255,255,255,0.04)",
};

const commandReadoutStyle = {
  display: "grid",
  gap: 4,
  padding: "10px 12px",
  border: "1px solid var(--border)",
  borderRadius: 12,
  background: "rgba(0,0,0,0.18)",
};

function CommandDeck({ activeTab, running }) {
  const activeLabel = TABS.find((tab) => tab.id === activeTab)?.label || "Command";
  return (
    <div style={commandDeckStyle}>
      <div
        aria-hidden="true"
        style={{
          position: "absolute",
          inset: -120,
          background:
            "repeating-linear-gradient(90deg, transparent 0 34px, rgba(255,255,255,0.025) 35px 36px), repeating-linear-gradient(0deg, transparent 0 34px, rgba(255,255,255,0.02) 35px 36px)",
          maskImage: "radial-gradient(circle at 50% 20%, black 0%, transparent 70%)",
          pointerEvents: "none",
        }}
      />
      <div style={{ position: "relative", display: "flex", justifyContent: "space-between", gap: 16, flexWrap: "wrap" }}>
        <div style={{ display: "grid", gap: 6, maxWidth: 720 }}>
          <div style={{ fontSize: 11, letterSpacing: "0.18em", color: "var(--accent)", textTransform: "uppercase" }}>
            PatchHive command center
          </div>
          <div style={{ fontSize: 34, lineHeight: 1, fontWeight: 950, letterSpacing: "-0.05em" }}>
            HiveCore is online.
          </div>
          <div style={{ fontSize: 13, color: "var(--text-dim)", lineHeight: 1.55 }}>
            Fleet lifecycle, product health, policy defaults, and autonomous action gates for the full 11-product PatchHive suite.
          </div>
        </div>
        <div
          style={{
            width: 116,
            height: 116,
            borderRadius: "50%",
            display: "grid",
            placeItems: "center",
            border: "1px solid color-mix(in srgb, var(--accent) 48%, var(--border))",
            background:
              "radial-gradient(circle, color-mix(in srgb, var(--accent) 16%, transparent) 0 28%, transparent 29% 42%, color-mix(in srgb, var(--blue) 18%, transparent) 43% 44%, transparent 45%), conic-gradient(from 160deg, color-mix(in srgb, var(--accent) 70%, transparent), transparent, color-mix(in srgb, var(--blue) 70%, transparent), transparent)",
            boxShadow: "inset 0 0 30px rgba(0,0,0,0.35)",
          }}
        >
          <div style={{ fontSize: 32, fontWeight: 900 }}>⬢</div>
        </div>
      </div>
      <div style={{ position: "relative", display: "grid", gap: 10, gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))" }}>
        <div style={commandReadoutStyle}>
          <div style={{ fontSize: 10, color: "var(--text-dim)", letterSpacing: "0.14em", textTransform: "uppercase" }}>Active station</div>
          <div style={{ fontSize: 17, fontWeight: 850 }}>{activeLabel}</div>
        </div>
        <div style={commandReadoutStyle}>
          <div style={{ fontSize: 10, color: "var(--text-dim)", letterSpacing: "0.14em", textTransform: "uppercase" }}>Control link</div>
          <div style={{ fontSize: 17, fontWeight: 850, color: running ? "var(--gold)" : "var(--green)" }}>
            {running ? "Executing" : "Standing by"}
          </div>
        </div>
        <div style={commandReadoutStyle}>
          <div style={{ fontSize: 10, color: "var(--text-dim)", letterSpacing: "0.14em", textTransform: "uppercase" }}>Backend</div>
          <div style={{ fontSize: 17, fontWeight: 850 }}>8100 / API</div>
        </div>
        <div style={commandReadoutStyle}>
          <div style={{ fontSize: 10, color: "var(--text-dim)", letterSpacing: "0.14em", textTransform: "uppercase" }}>Fleet</div>
          <div style={{ fontSize: 17, fontWeight: 850 }}>11 products</div>
        </div>
        <div style={commandReadoutStyle}>
          <div style={{ fontSize: 10, color: "var(--text-dim)", letterSpacing: "0.14em", textTransform: "uppercase" }}>Launcher</div>
          <div style={{ fontSize: 17, fontWeight: 850 }}>8210 / Host ops</div>
        </div>
      </div>
    </div>
  );
}

export default function App() {
  const { apiKey, checked, needsAuth, login, logout, authError, bootstrapRequired, generateKey } =
    useApiKeyAuth({
      apiBase: API,
      storageKey: "hive-core_api_key",
    });
  const [tab, setTab] = useState("setup");
  const [running, setRunning] = useState(false);
  const [error, setError] = useState("");
  const fetch_ = useApiFetcher(apiKey);

  useEffect(() => {
    applyTheme("hive-core");
  }, []);

  async function fetchEnvelope(path, options = {}) {
    const res = await fetch_(`${API}${path}`, options);
    const payload = await res.json().catch(() => null);
    if (!res.ok) {
      throw new Error(payload?.error?.message || payload?.error || "HiveCore request failed.");
    }
    return payload?.data ?? payload;
  }

  return (
    <ProductSessionGate
      checked={checked}
      needsAuth={needsAuth}
      onLogin={login}
      icon="⬢"
      title="HiveCore"
      storageKey="hive-core_api_key"
      apiBase={API}
      authError={authError}
      bootstrapRequired={bootstrapRequired}
      onGenerateKey={generateKey}
      loadingColor="#3a9fb3"
    >
      <ProductAppFrame
        icon="⬢"
        title="HiveCore Command"
        product="HiveCore"
        running={running}
        headerChildren={
          <div style={{ display: "grid", gap: 2, textAlign: "right" }}>
            <div style={{ fontSize: 10, color: "var(--text-dim)" }}>
              Control the PatchHive suite from one clear surface.
            </div>
            <div style={{ fontSize: 10, color: "var(--accent)" }}>
              Launch authority and suite telemetry
            </div>
          </div>
        }
        tabs={TABS}
        activeTab={tab}
        onTabChange={setTab}
        error={error}
        maxWidth={1440}
        shellStyle={shellBackdrop}
        contentStyle={{ paddingTop: 18 }}
        onSignOut={logout}
        showSignOut={Boolean(apiKey)}
      >
        <CommandDeck activeTab={tab} running={running} />
        {tab === "setup" && (
          <SetupPanel
            fetchEnvelope={fetchEnvelope}
            setRunning={setRunning}
            setError={setError}
          />
        )}
        {tab === "overview" && (
          <OverviewPanel
            fetchEnvelope={fetchEnvelope}
            setRunning={setRunning}
            setError={setError}
          />
        )}
        {tab === "products" && (
          <ProductsPanel
            fetchEnvelope={fetchEnvelope}
            setRunning={setRunning}
            setError={setError}
          />
        )}
        {tab === "settings" && (
          <SettingsPanel
            fetchEnvelope={fetchEnvelope}
            setRunning={setRunning}
            setError={setError}
          />
        )}
        {tab === "checks" && <ChecksPanel apiKey={apiKey} />}
      </ProductAppFrame>
    </ProductSessionGate>
  );
}
