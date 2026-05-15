import { useEffect, useState } from "react";
import { applyTheme, PanelErrorBoundary } from "@patchhivehq/ui";
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
  flexWrap: "wrap",
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

const miniOrbitStyle = {
  position: "relative",
  flex: "1 1 320px",
  height: 82,
  minWidth: 280,
  maxWidth: 430,
  border: "1px solid color-mix(in srgb, var(--blue) 16%, var(--border))",
  borderRadius: 8,
  background:
    "radial-gradient(circle at 50% 50%, color-mix(in srgb, var(--blue) 18%, transparent) 0 16%, transparent 17%), color-mix(in srgb, var(--bg) 48%, transparent)",
  overflow: "hidden",
};

function deckStatusColor(status) {
  if (status === "online") return "var(--green)";
  if (status === "degraded") return "var(--gold)";
  if (status === "disabled") return "var(--text-dim)";
  if (status === "unconfigured") return "var(--blue)";
  return "var(--accent)";
}

function deckOrbitProducts(products) {
  return [...products]
    .map((product, index) => ({ product, index }))
    .sort((left, right) => {
      if (left.product.slug === "hive-core") return -1;
      if (right.product.slug === "hive-core") return 1;
      return left.index - right.index;
    })
    .map(({ product }) => product);
}

function MiniLauncherOrbit({ products }) {
  const orbitProducts = deckOrbitProducts(products || []);
  const count = Math.max(orbitProducts.length, 1);
  return (
    <div style={miniOrbitStyle} aria-label="PatchHive launcher orbit">
      <div
        aria-hidden="true"
        style={{
          position: "absolute",
          left: "50%",
          top: "50%",
          width: "58%",
          height: "66%",
          border: "1px solid color-mix(in srgb, var(--accent) 20%, transparent)",
          borderRadius: "50%",
          transform: "translate(-50%, -50%)",
        }}
      />
      <div
        style={{
          position: "absolute",
          left: "50%",
          top: "50%",
          transform: "translate(-50%, -50%)",
          display: "grid",
          placeItems: "center",
          width: 34,
          height: 34,
          borderRadius: 8,
          border: "1px solid color-mix(in srgb, var(--blue) 42%, var(--border))",
          background: "color-mix(in srgb, var(--blue) 18%, var(--bg-panel))",
          color: "var(--text)",
          fontWeight: 900,
        }}
      >
        ⬢
      </div>
      {orbitProducts.map((product, index) => {
        const angle = -90 + (index * 360) / count;
        const radians = (angle * Math.PI) / 180;
        const tone = deckStatusColor(product.status);
        const x = 50 + Math.cos(radians) * 38;
        const y = 50 + Math.sin(radians) * 34;
        return (
          <div
            key={product.slug}
            title={`${product.title}: ${product.status}`}
            style={{
              position: "absolute",
              left: `${x}%`,
              top: `${y}%`,
              transform: "translate(-50%, -50%)",
              display: "grid",
              placeItems: "center",
              width: product.slug === "hive-core" ? 29 : 22,
              height: product.slug === "hive-core" ? 29 : 22,
              borderRadius: 7,
              border: `1px solid color-mix(in srgb, ${tone} 56%, var(--border))`,
              background: `color-mix(in srgb, ${tone} ${product.slug === "hive-core" ? 20 : 12}%, var(--bg-panel))`,
              color: "var(--text)",
              fontSize: product.slug === "hive-core" ? 13 : 11,
              lineHeight: 1,
              boxShadow: `0 0 18px color-mix(in srgb, ${tone} 22%, transparent)`,
            }}
          >
            {product.icon || "•"}
          </div>
        );
      })}
    </div>
  );
}

function HiveCorePanelBoundary({ tab, children, setError }) {
  const label = TABS.find((item) => item.id === tab)?.label || "HiveCore panel";
  return (
    <PanelErrorBoundary
      key={tab}
      label={label}
      onError={(error) => {
        setError(`${label} panel render fault: ${error?.message || error}`);
      }}
    >
      {children}
    </PanelErrorBoundary>
  );
}

function CommandDeck({ activeTab, running, products }) {
  const activeLabel = TABS.find((tab) => tab.id === activeTab)?.label || "Command";
  const productCount = products?.length || 12;
  const runningCount = (products || []).filter((product) => ["online", "degraded"].includes(product.status)).length;
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
      <div style={{ position: "relative", display: "flex", alignItems: "center", gap: 10, minWidth: 0, flex: "1 1 260px" }}>
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
      <MiniLauncherOrbit products={products} />
      <div style={{ position: "relative", display: "flex", gap: 7, flexWrap: "wrap", justifyContent: "flex-end", flex: "1 1 280px" }}>
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
          <div style={{ fontSize: 11, fontWeight: 850 }}>{productCount} products</div>
        </div>
        <div style={commandReadoutStyle}>
          <div style={{ fontSize: 9, color: "var(--text-dim)", textTransform: "uppercase" }}>Online</div>
          <div style={{ fontSize: 11, fontWeight: 850, color: runningCount > 0 ? "var(--green)" : "var(--gold)" }}>
            {runningCount || "scan"}
          </div>
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
  const [suiteProducts, setSuiteProducts] = useState([]);
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

  useEffect(() => {
    if (!checked || needsAuth) return undefined;
    let cancelled = false;

    async function loadSuiteProducts() {
      try {
        const data = await fetchEnvelope("/products");
        if (!cancelled) setSuiteProducts(Array.isArray(data) ? data : []);
      } catch {
        if (!cancelled) setSuiteProducts([]);
      }
    }

    loadSuiteProducts();
    const timer = setInterval(loadSuiteProducts, 10000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [checked, needsAuth, apiKey]);

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
        <CommandDeck activeTab={tab} running={running} products={suiteProducts} />
        <HiveCorePanelBoundary tab={tab} setError={setError}>
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
        </HiveCorePanelBoundary>
      </ProductAppFrame>
    </ProductSessionGate>
  );
}
