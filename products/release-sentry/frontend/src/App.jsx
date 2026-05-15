import { useEffect, useState } from "react";
import { applyTheme } from "@patchhivehq/ui";
import {
  ProductAppFrame,
  ProductSessionGate,
  useApiKeyAuth,
} from "@patchhivehq/product-shell";
import { API } from "./config.js";
import OverviewPanel from "./panels/OverviewPanel.jsx";
import HistoryPanel from "./panels/HistoryPanel.jsx";
import ChecksPanel from "./panels/ChecksPanel.jsx";

const TABS = [
  { id: "overview", label: "🚦 Release Gate" },
  { id: "history", label: "History" },
  { id: "checks", label: "Checks" },
];

export default function App() {
  const { apiKey, checked, needsAuth, login, logout, authError, bootstrapRequired, generateKey } = useApiKeyAuth({
    apiBase: API,
    storageKey: "release-sentry_api_key",
  });
  const [tab, setTab] = useState("overview");

  useEffect(() => {
    applyTheme("release-sentry");
  }, []);

  return (
    <ProductSessionGate
      checked={checked}
      needsAuth={needsAuth}
      onLogin={login}
      icon="🚦"
      title="ReleaseSentry"
      storageKey="release-sentry_api_key"
      apiBase={API}
      authError={authError}
      bootstrapRequired={bootstrapRequired}
      onGenerateKey={generateKey}
    >
      <ProductAppFrame
        icon="🚦"
        title="ReleaseSentry"
        product="ReleaseSentry"
        headerChildren={
          <div style={{ fontSize: 10, color: "var(--text-dim)" }}>
            Release readiness, changelog drift, CI health, and ship/no-ship evidence.
          </div>
        }
        tabs={TABS}
        activeTab={tab}
        onTabChange={setTab}
        maxWidth={1200}
        onSignOut={logout}
        showSignOut={Boolean(apiKey)}
      >
        {tab === "overview" && <OverviewPanel apiKey={apiKey} />}
        {tab === "history" && <HistoryPanel apiKey={apiKey} />}
        {tab === "checks" && <ChecksPanel apiKey={apiKey} />}
      </ProductAppFrame>
    </ProductSessionGate>
  );
}
