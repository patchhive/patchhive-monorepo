import { useEffect, useState } from "react";
import {
  applyTheme,
  Btn,
  LoginPage,
  PatchHiveFooter,
  PatchHiveHeader,
  TabBar,
} from "@patchhivehq/ui";
import { useApiKeyAuth } from "@patchhivehq/product-shell";
import { API } from "./config.js";
import OverviewPanel from "./panels/OverviewPanel.jsx";
import ChecksPanel from "./panels/ChecksPanel.jsx";

const TABS = [
  { id: "overview", label: "__PRODUCT_ICON__ Overview" },
  { id: "checks", label: "Checks" },
];

export default function App() {
  const { apiKey, checked, needsAuth, login, logout } = useApiKeyAuth({
    apiBase: API,
    storageKey: "__PRODUCT_SLUG___api_key",
  });
  const [tab, setTab] = useState("overview");

  useEffect(() => {
    applyTheme("__PRODUCT_THEME__");
  }, []);

  if (!checked) {
    return (
      <div style={{ minHeight: "100vh", background: "#080810", display: "flex", alignItems: "center", justifyContent: "center", color: "var(--accent)", fontSize: 26 }}>
        __PRODUCT_ICON__
      </div>
    );
  }

  if (needsAuth) {
    return (
      <LoginPage
        onLogin={login}
        icon="__PRODUCT_ICON__"
        title="__PRODUCT_TITLE__"
        subtitle="by PatchHive"
        storageKey="__PRODUCT_SLUG___api_key"
        apiBase={API}
      />
    );
  }

  return (
    <div style={{ minHeight: "100vh", background: "var(--bg)", color: "var(--text)", fontFamily: "'SF Mono','Fira Mono',monospace", fontSize: 12 }}>
      <PatchHiveHeader icon="__PRODUCT_ICON__" title="__PRODUCT_TITLE__" version="v0.1.0">
        <div style={{ fontSize: 10, color: "var(--text-dim)" }}>__PRODUCT_TAGLINE__</div>
        {apiKey && (
          <Btn onClick={logout} style={{ padding: "4px 10px" }}>
            Sign out
          </Btn>
        )}
      </PatchHiveHeader>

      <TabBar tabs={TABS} active={tab} onChange={setTab} />

      <div style={{ padding: 24, maxWidth: 1200, margin: "0 auto", display: "grid", gap: 16 }}>
        {tab === "overview" && <OverviewPanel apiKey={apiKey} />}
        {tab === "checks" && <ChecksPanel apiKey={apiKey} />}
      </div>

      <PatchHiveFooter product="__PRODUCT_TITLE__" />
    </div>
  );
}
