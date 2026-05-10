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
    "linear-gradient(135deg, #03080b 0%, color-mix(in srgb, var(--bg) 88%, #05181c) 48%, #061015 100%)",
};

const commandDeckStyle = {
  position: "relative",
  overflow: "hidden",
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
  gap: 14,
  padding: "10px 12px",
  border: "1px solid color-mix(in srgb, var(--accent) 22%, var(--border))",
  borderRadius: 8,
  background:
    "linear-gradient(90deg, color-mix(in srgb, var(--bg-panel) 72%, #020509) 0%, color-mix(in srgb, var(--bg-panel) 92%, #051117) 54%, color-mix(in srgb, var(--blue) 10%, var(--bg-panel)) 100%)",
  boxShadow: "inset 0 1px 0 rgba(255,255,255,0.04)",
};

const commandReadoutStyle = {
  display: "flex",
  alignItems: "baseline",
  gap: 6,
  padding: "6px 8px",
  border: "1px solid color-mix(in srgb, var(--accent) 14%, var(--border))",
  borderRadius: 7,
  background: "color-mix(in srgb, var(--bg) 54%, transparent)",
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
      <div style={{ position: "relative", display: "flex", alignItems: "center", gap: 10, minWidth: 0 }}>
        <div style={{ color: "var(--accent)", fontSize: 18, lineHeight: 1 }}>⬢</div>
        <div style={{ display: "grid", gap: 2, minWidth: 0 }}>
          <div style={{ fontSize: 11, color: "var(--accent)", fontWeight: 900, textTransform: "uppercase" }}>
            HiveCore command kernel
          </div>
          <div style={{ fontSize: 11, color: "var(--text-dim)" }}>
            Suite lifecycle, launch authority, and autonomous action telemetry.
          </div>
        </div>
      </div>
      <div style={{ position: "relative", display: "flex", gap: 7, flexWrap: "wrap", justifyContent: "flex-end" }}>
        <div style={commandReadoutStyle}>
          <div style={{ fontSize: 9, color: "var(--text-dim)", textTransform: "uppercase" }}>Station</div>
          <div style={{ fontSize: 11, fontWeight: 850 }}>{activeLabel}</div>
        </div>
        <div style={commandReadoutStyle}>
          <div style={{ fontSize: 9, color: "var(--text-dim)", textTransform: "uppercase" }}>Link</div>
          <div style={{ fontSize: 11, fontWeight: 850, color: running ? "var(--gold)" : "var(--green)" }}>
            {running ? "Executing" : "Standing by"}
          </div>
        </div>
        <div style={commandReadoutStyle}>
          <div style={{ fontSize: 9, color: "var(--text-dim)", textTransform: "uppercase" }}>Fleet</div>
          <div style={{ fontSize: 11, fontWeight: 850 }}>11 products</div>
        </div>
        <div style={commandReadoutStyle}>
          <div style={{ fontSize: 9, color: "var(--text-dim)", textTransform: "uppercase" }}>Launcher</div>
          <div style={{ fontSize: 11, fontWeight: 850 }}>8210</div>
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
