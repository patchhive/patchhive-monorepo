import { useEffect, useState } from "react";
import { applyTheme } from "@patchhivehq/ui";
import {
  ProductAppFrame,
  ProductSessionGate,
  useApiKeyAuth,
} from "@patchhivehq/product-shell";
import { API } from "./config.js";
import OverviewPanel from "./panels/OverviewPanel.jsx";
import ChecksPanel from "./panels/ChecksPanel.jsx";

const TABS = [
  { id: "overview", label: "__PRODUCT_ICON__ Overview" },
  { id: "checks", label: "Checks" },
];

export default function App() {
  const { apiKey, checked, needsAuth, login, logout, authError, bootstrapRequired, generateKey } = useApiKeyAuth({
    apiBase: API,
    storageKey: "__PRODUCT_SLUG___api_key",
  });
  const [tab, setTab] = useState("overview");

  useEffect(() => {
    applyTheme("__PRODUCT_THEME__");
  }, []);

  return (
    <ProductSessionGate
      checked={checked}
      needsAuth={needsAuth}
      onLogin={login}
      icon="__PRODUCT_ICON__"
      title="__PRODUCT_TITLE__"
      storageKey="__PRODUCT_SLUG___api_key"
      apiBase={API}
      authError={authError}
      bootstrapRequired={bootstrapRequired}
      onGenerateKey={generateKey}
    >
      <ProductAppFrame
        icon="__PRODUCT_ICON__"
        title="__PRODUCT_TITLE__"
        product="__PRODUCT_TITLE__"
        headerChildren={
          <div style={{ fontSize: 10, color: "var(--text-dim)" }}>__PRODUCT_TAGLINE__</div>
        }
        tabs={TABS}
        activeTab={tab}
        onTabChange={setTab}
        maxWidth={1200}
        onSignOut={logout}
        showSignOut={Boolean(apiKey)}
      >
        {tab === "overview" && <OverviewPanel apiKey={apiKey} />}
        {tab === "checks" && <ChecksPanel apiKey={apiKey} />}
      </ProductAppFrame>
    </ProductSessionGate>
  );
}
