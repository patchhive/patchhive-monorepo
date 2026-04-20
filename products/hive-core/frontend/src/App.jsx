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

const TABS = [
  { id: "overview", label: "⬢ Overview" },
  { id: "products", label: "Products" },
  { id: "settings", label: "Settings" },
  { id: "checks", label: "Checks" },
];

export default function App() {
  const { apiKey, checked, needsAuth, login, logout, authError, bootstrapRequired, generateKey } =
    useApiKeyAuth({
      apiBase: API,
      storageKey: "hive-core_api_key",
    });
  const [tab, setTab] = useState("overview");
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
        title="HiveCore"
        product="HiveCore"
        running={running}
        headerChildren={
          <div style={{ display: "grid", gap: 2, textAlign: "right" }}>
            <div style={{ fontSize: 10, color: "var(--text-dim)" }}>
              Control the PatchHive suite from one clear surface.
            </div>
            <div style={{ fontSize: 10, color: "var(--accent)" }}>
              Saved defaults, live health, and launch links
            </div>
          </div>
        }
        tabs={TABS}
        activeTab={tab}
        onTabChange={setTab}
        error={error}
        maxWidth={1320}
        onSignOut={logout}
        showSignOut={Boolean(apiKey)}
      >
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
