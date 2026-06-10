import { createContext, useContext, useEffect, useMemo, useState } from "react";

const DEFAULT_RADAR_WINDOWS = {
  7: { label: "7 day live pass", outer: "7d", mid: "3d", inner: "24h" },
  14: { label: "14 day history pass", outer: "14d", mid: "7d", inner: "3d" },
  30: { label: "30 day deep sweep", outer: "30d", mid: "14d", inner: "7d" },
};

const PRODUCT_ACCENTS = {
  "repo-reaper": {
    accent: "#c41e3a",
    accentDim: "#8b1528",
    accentRgb: "196 30 58",
    accentGlow: "#ff637c",
    accentGlowRgb: "255 99 124",
  },
  "signal-hive": {
    accent: "#2a6aaa",
    accentDim: "#1a4a7a",
    accentRgb: "42 106 170",
    accentGlow: "#67bbe7",
    accentGlowRgb: "103 187 231",
  },
  "review-bee": {
    accent: "#d89f22",
    accentDim: "#8f6415",
    accentRgb: "216 159 34",
    accentGlow: "#ffd36a",
    accentGlowRgb: "255 211 106",
  },
  "trust-gate": {
    accent: "#8751b8",
    accentDim: "#553174",
    accentRgb: "135 81 184",
    accentGlow: "#c794ff",
    accentGlowRgb: "199 148 255",
  },
  "repo-memory": {
    accent: "#2a8a4a",
    accentDim: "#1a5a30",
    accentRgb: "42 138 74",
    accentGlow: "#65d98e",
    accentGlowRgb: "101 217 142",
  },
  "merge-keeper": {
    accent: "#1f9f8f",
    accentDim: "#17675e",
    accentRgb: "31 159 143",
    accentGlow: "#62e1d3",
    accentGlowRgb: "98 225 211",
  },
  "flake-sting": {
    accent: "#d96b27",
    accentDim: "#8b4317",
    accentRgb: "217 107 39",
    accentGlow: "#ff9b52",
    accentGlowRgb: "255 155 82",
  },
  "dep-triage": {
    accent: "#8aa62c",
    accentDim: "#586d1b",
    accentRgb: "138 166 44",
    accentGlow: "#c8db62",
    accentGlowRgb: "200 219 98",
  },
  "vuln-triage": {
    accent: "#d6406a",
    accentDim: "#8d2746",
    accentRgb: "214 64 106",
    accentGlow: "#ff7aa1",
    accentGlowRgb: "255 122 161",
  },
  "refactor-scout": {
    accent: "#30a783",
    accentDim: "#1d6d55",
    accentRgb: "48 167 131",
    accentGlow: "#70dfbd",
    accentGlowRgb: "112 223 189",
  },
  "release-sentry": {
    accent: "#e0c84d",
    accentDim: "#8a7728",
    accentRgb: "224 200 77",
    accentGlow: "#fff08a",
    accentGlowRgb: "255 240 138",
  },
  "hive-core": {
    accent: "#3a9fb3",
    accentDim: "#1d5d69",
    accentRgb: "58 159 179",
    accentGlow: "#7bd8e8",
    accentGlowRgb: "123 216 232",
  },
};

const PRODUCT_BRANDS = {
  "repo-reaper": { name: "RepoReaper", subtitle: "patch execution cell" },
  "signal-hive": { name: "SignalHive", subtitle: "reconnaissance cell" },
  "review-bee": { name: "ReviewBee", subtitle: "review resolution cell" },
  "trust-gate": { name: "TrustGate", subtitle: "trust guard cell" },
  "repo-memory": { name: "RepoMemory", subtitle: "memory comb" },
  "merge-keeper": { name: "MergeKeeper", subtitle: "comb readiness" },
  "flake-sting": { name: "FlakeSting", subtitle: "CI sting detector" },
  "dep-triage": { name: "DepTriage", subtitle: "dependency comb" },
  "vuln-triage": { name: "VulnTriage", subtitle: "security comb" },
  "refactor-scout": { name: "RefactorScout", subtitle: "cleanup scout cell" },
  "release-sentry": { name: "ReleaseSentry", subtitle: "release watch cell" },
  "hive-core": { name: "HiveCore", subtitle: "control center" },
};

const ProductV2RuntimeContext = createContext({
  authConfigured: false,
  runtime: null,
});

export function applySuiteAccent(productKey = "signal-hive") {
  if (typeof document === "undefined") {
    return;
  }

  const accent = PRODUCT_ACCENTS[productKey] || PRODUCT_ACCENTS["signal-hive"];
  const root = document.documentElement;
  root.dataset.patchhiveProduct = productKey;
  root.style.setProperty("--accent", accent.accent);
  root.style.setProperty("--accent-dim", accent.accentDim);
  root.style.setProperty("--accent-rgb", accent.accentRgb);
  root.style.setProperty("--signal", accent.accent);
  root.style.setProperty("--signal-rgb", accent.accentRgb);
  root.style.setProperty("--signal2", accent.accentGlow);
  root.style.setProperty("--signal2-rgb", accent.accentGlowRgb);
}

export function toneClass(tone) {
  return tone ? ` ${tone}` : "";
}

function productTabStorageKey(productKey) {
  return `${productKey || "patchhive"}_active_tab`;
}

function validProductTab(tabs, id) {
  return tabs.some((tab) => tab.id === id);
}

function fallbackProductTab(tabs, defaultTab) {
  if (validProductTab(tabs, defaultTab)) {
    return defaultTab;
  }
  return tabs[0]?.id || "";
}

function readProductTab(productKey, tabs, defaultTab) {
  const fallback = fallbackProductTab(tabs, defaultTab);
  if (typeof window === "undefined") {
    return fallback;
  }

  try {
    const saved = window.localStorage.getItem(productTabStorageKey(productKey));
    return validProductTab(tabs, saved) ? saved : fallback;
  } catch {
    return fallback;
  }
}

export function usePersistentProductTab(productKey, tabs, defaultTab) {
  const [activeTab, setActiveTab] = useState(() => readProductTab(productKey, tabs, defaultTab));

  useEffect(() => {
    if (!validProductTab(tabs, activeTab)) {
      setActiveTab(fallbackProductTab(tabs, defaultTab));
      return;
    }

    if (typeof window === "undefined") {
      return;
    }

    try {
      window.localStorage.setItem(productTabStorageKey(productKey), activeTab);
    } catch {
      // Browsers can block storage; tab persistence is helpful but not required.
    }
  }, [productKey, tabs, defaultTab, activeTab]);

  return [activeTab, setActiveTab];
}

function runtimeTone(status) {
  if (status === "ok" || status === "online" || status === "ready") {
    return "ok";
  }
  if (status === "error" || status === "down" || status === "blocked") {
    return "hot";
  }
  return "warn";
}

function formatRuntimeTime(value) {
  if (!value) {
    return "";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return String(value);
  }
  const seconds = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 48) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

function runtimeCells(cells, runtime, authConfigured) {
  if (!runtime) {
    return cells;
  }

  const status = runtime.health?.status;
  const checkWarnings = (runtime.checks || []).filter((check) => (
    check.level === "warn" || check.level === "error"
  )).length;
  const latestRunAt = runtime.latestRun?.created_at || runtime.latestRun?.updated_at || runtime.latestRun?.completed_at;
  const latestRunLabel = formatRuntimeTime(latestRunAt);

  return cells.map((cell) => {
    const label = String(cell.label || "").toLowerCase();
    if (label === "system" && status) {
      return { ...cell, tone: runtimeTone(status), value: status };
    }
    if (label === "auth") {
      return { ...cell, tone: authConfigured ? "sig" : "warn", value: authConfigured ? "configured" : "open" };
    }
    if (label === "checks" && runtime.checks) {
      return { ...cell, tone: checkWarnings ? "warn" : "ok", value: checkWarnings ? `${checkWarnings} warnings` : "clear" };
    }
    if (label.startsWith("last") && latestRunLabel) {
      return { ...cell, value: latestRunLabel };
    }
    return cell;
  });
}

function productBrand(productKey = "signal-hive", productName) {
  const brand = PRODUCT_BRANDS[productKey] || PRODUCT_BRANDS["signal-hive"];
  return {
    name: productName || brand.name,
    subtitle: brand.subtitle,
  };
}

export function PatchHiveProductFooter({ productKey = "signal-hive", productName }) {
  const brand = productBrand(productKey, productName);
  return (
    <footer className="patchhive-footer" aria-label="PatchHive product identity">
      <span className="footer-brand">{brand.name} by PatchHive</span>
      <span className="footer-subtitle">{brand.subtitle}</span>
      <span className="footer-meta">Autonomous maintenance suite</span>
    </footer>
  );
}

export function ProductV2Shell({ authConfigured = false, children, productKey = "signal-hive", productName, runtime = null, showFooter = true }) {
  const contextValue = useMemo(() => ({ authConfigured, runtime }), [authConfigured, runtime]);
  return (
    <ProductV2RuntimeContext.Provider value={contextValue}>
      {children}
      {showFooter && <PatchHiveProductFooter productKey={productKey} productName={productName} />}
    </ProductV2RuntimeContext.Provider>
  );
}

export function ProductV2AuthGate({
  apiBase,
  auth,
  keyPrefix = "",
  productKey = "signal-hive",
  productName,
}) {
  const [key, setKey] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [generatedKey, setGeneratedKey] = useState("");
  const [copiedKey, setCopiedKey] = useState(false);

  useEffect(() => {
    applySuiteAccent(productKey);
  }, [productKey]);

  const submit = async (event) => {
    event.preventDefault();
    if (!key.trim()) {
      return;
    }
    setBusy(true);
    setError("");
    try {
      const response = await fetch(`${apiBase}/auth/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ api_key: key.trim() }),
      });
      if (!response.ok) {
        throw new Error("Invalid API key.");
      }
      auth.login(key.trim());
    } catch (err) {
      setError(err.message || `Cannot reach ${productName}.`);
    } finally {
      setBusy(false);
    }
  };

  const generate = async () => {
    setBusy(true);
    setError("");
    try {
      const nextKey = await auth.generateKey({ autoLogin: false });
      setGeneratedKey(nextKey);
      setKey(nextKey);
      setCopiedKey(false);
    } catch (err) {
      setError(err.message || "Could not generate an API key.");
    } finally {
      setBusy(false);
    }
  };

  const copyGeneratedKey = async () => {
    if (!generatedKey) {
      return;
    }
    try {
      await navigator.clipboard.writeText(generatedKey);
      setCopiedKey(true);
    } catch {
      setError("Could not copy generated API key.");
    }
  };

  if (!auth.checked) {
    return (
      <div className="auth-shell">
        <div className="auth-card">
          <span className="micro">// {productName}</span>
          <div className="auth-title">Checking session</div>
          <div className="auth-meter" />
        </div>
      </div>
    );
  }

  return (
    <div className="auth-shell">
      <form className="auth-card" onSubmit={submit}>
        <span className="micro">// Operator access</span>
        <div className="auth-title">{productName}</div>
        <p className="auth-copy">
          {auth.bootstrapRequired ? "Generate the first local API key or enter an existing one." : `Enter the local ${productName} API key.`}
        </p>
        <label className="v2-field">
          <span>API endpoint</span>
          <input className="v2-input" readOnly value={apiBase} />
        </label>
        <label className="v2-field">
          <span>API key</span>
          <input
            className="v2-input"
            onChange={(event) => setKey(event.target.value)}
            placeholder={`${keyPrefix}...`}
            type="password"
            value={key}
          />
        </label>
        {(auth.authError || error) && <div className="status-banner red">{error || auth.authError}</div>}
        {generatedKey && (
          <div className="status-banner green">
            Generated key for this browser session. Copy it now, then press Enter: <span className="break-all">{generatedKey}</span>
          </div>
        )}
        {copiedKey && <div className="status-banner signal">API key copied.</div>}
        <button className="btn primary" disabled={busy || !key.trim()} type="submit">
          {busy ? "Authenticating" : "Enter"}
        </button>
        {generatedKey && (
          <button className="btn" disabled={busy} onClick={copyGeneratedKey} type="button">
            Copy generated key
          </button>
        )}
        {auth.bootstrapRequired && (
          <button className="btn" disabled={busy} onClick={generate} type="button">
            {busy ? "Generating" : "Generate local API key"}
          </button>
        )}
      </form>
    </div>
  );
}

export function DeckBar({
  activeTab,
  brandEyebrow = "PatchHive v2 track",
  brandName,
  navLabel = "PatchHive v2 surfaces",
  onTabChange,
  productKey = "signal-hive",
  tabs,
}) {
  useEffect(() => {
    applySuiteAccent(productKey);
  }, [productKey]);

  return (
    <div className="deckbar">
      <div className="deckbrand">
        <div className="deckmark" aria-hidden="true" />
        <div className="decktitle">
          <span className="deckeyebrow">{brandEyebrow}</span>
          <span className="deckname">{brandName}</span>
        </div>
      </div>
      <div className="decknav" role="tablist" aria-label={navLabel}>
        {tabs.map((tab) => (
          <button
            className={`navtab${activeTab === tab.id ? " active" : ""}`}
            key={tab.id}
            onClick={() => onTabChange(tab.id)}
            role="tab"
            type="button"
          >
            {tab.label}
          </button>
        ))}
      </div>
    </div>
  );
}

export function SuiteTopline({ cells }) {
  const { authConfigured, runtime } = useContext(ProductV2RuntimeContext);
  const renderedCells = runtimeCells(cells, runtime, authConfigured);
  return (
    <div className="topline">
      {renderedCells.map(({ label, value, tone = "" }) => (
        <div className="topcell" key={label}>
          <span className="label">{label}</span>
          <span className={`value${toneClass(tone)}`}>{value}</span>
        </div>
      ))}
    </div>
  );
}

export function ProductRail({ sections, stats }) {
  return (
    <aside className="rail">
      {sections.map((section, sectionIndex) => (
        <div key={section.title}>
          <div className={`railhead${sectionIndex > 0 ? " railgap" : ""}`}>{section.title}</div>
          {section.items.map((item) => (
            <div className={`railitem${item.active ? " active" : ""}`} key={item.label}>
              <span>{item.label}</span>
              {item.pin ? <span className="pin" /> : item.badge ? <span className={`chip ${item.badgeTone || ""}`}>{item.badge}</span> : <span>{item.value}</span>}
            </div>
          ))}
        </div>
      ))}
      {stats && (
        <>
          <div className="railhead railgap">{stats.title}</div>
          <div className="railstats">
            {stats.items.map((item) => (
              <div className="railstat" key={item.label}>
                <span className="label">{item.label}</span>
                <span className={item.large ? `big${toneClass(item.tone)}` : `value${toneClass(item.tone)}`}>{item.value}</span>
              </div>
            ))}
          </div>
        </>
      )}
    </aside>
  );
}

export function MetricBand({ metrics }) {
  return (
    <div className="metrics">
      {metrics.map((metric) => (
        <div className="metric" key={metric.label}>
          <span className="label">{metric.label}</span>
          <span className="metric-main">
            <span className={`big${toneClass(metric.tone)}`}>{metric.value}</span>
            {metric.unit && <span className="metric-unit">{metric.unit}</span>}
          </span>
          {metric.sub && <span className="micro">{metric.sub}</span>}
        </div>
      ))}
    </div>
  );
}

export function Panel({ eyebrow, title, action, children }) {
  return (
    <section className="panel">
      <div className="panelhead">
        <div>
          <span className="micro">// {eyebrow}</span>
          <div className="paneltitle">{title}</div>
        </div>
        {action}
      </div>
      {children}
    </section>
  );
}

export function HistoryDetailGrid({ children, className = "" }) {
  const classes = ["history-detail-grid", "atlas-layout", "suite-four-layout", className].filter(Boolean).join(" ");
  return <div className={classes}>{children}</div>;
}

function defaultMinWindow(index) {
  if (index < 3) {
    return 7;
  }
  if (index === 3) {
    return 14;
  }
  return 30;
}

export function SuiteRadar({
  ariaLabel = "PatchHive signal radar",
  detailLabel = "Selected signal",
  echoes = [],
  feed = [],
  gainLabel = "Signal gain",
  itemQueryParam = "radar",
  items,
  signalLabel = "signals",
  vectorLabel = "Sweep vector",
  windows = DEFAULT_RADAR_WINDOWS,
}) {
  const params = useMemo(
    () => (typeof window === "undefined" ? new URLSearchParams() : new URLSearchParams(window.location.search)),
    [],
  );
  const windowKeys = useMemo(() => Object.keys(windows).map(Number).sort((a, b) => a - b), [windows]);
  const firstWindow = windowKeys[0] || 7;
  const [windowDays, setWindowDays] = useState(() => {
    const raw = Number(params.get("window") || firstWindow);
    return windows[raw] ? raw : firstWindow;
  });
  const normalizedItems = useMemo(
    () => items.map((item, index) => ({ ...item, minWindow: item.minWindow || defaultMinWindow(index) })),
    [items],
  );
  const visibleItems = useMemo(
    () => normalizedItems.filter((item) => item.minWindow <= windowDays),
    [normalizedItems, windowDays],
  );
  const [selectedItem, setSelectedItem] = useState(() => {
    const requested = params.get(itemQueryParam);
    return normalizedItems.find((item) => item.id === requested) || normalizedItems[0];
  });

  const updateRadarUrl = (item, days) => {
    if (typeof window === "undefined") {
      return;
    }
    const nextParams = new URLSearchParams(window.location.search);
    if (item?.id) {
      nextParams.set(itemQueryParam, item.id);
    } else {
      nextParams.delete(itemQueryParam);
    }
    if (days) {
      nextParams.set("window", String(days));
    }
    const query = nextParams.toString();
    window.history.replaceState(
      null,
      "",
      `${window.location.pathname}${query ? `?${query}` : ""}${window.location.hash || ""}`,
    );
  };

  useEffect(() => {
    if (!selectedItem || selectedItem.minWindow > windowDays) {
      const nextItem = visibleItems[0] || normalizedItems[0];
      setSelectedItem(nextItem);
      updateRadarUrl(nextItem, windowDays);
    }
  }, [normalizedItems, selectedItem, visibleItems, windowDays]);

  const activeWindow = windows[windowDays] || windows[firstWindow] || DEFAULT_RADAR_WINDOWS[7];
  const visibleEchoes = echoes.filter((echo, index) => (echo.minWindow || defaultMinWindow(index + 3)) <= windowDays);
  const stats = selectedItem?.stats || [];

  if (!selectedItem) {
    return null;
  }

  return (
    <div className="signal-map" data-window={windowDays}>
      <div className="range-panel">
        <span className="chip signal">{activeWindow.count || `${visibleItems.length} ${signalLabel}`}</span>
        <div className="range-switch" aria-label="Radar history window">
          {windowKeys.map((days) => (
            <button
              className={`range-btn${windowDays === days ? " active" : ""}`}
              key={days}
              onClick={() => {
                const nextItem = selectedItem?.minWindow <= days
                  ? selectedItem
                  : normalizedItems.find((item) => item.minWindow <= days) || normalizedItems[0];
                setWindowDays(days);
                setSelectedItem(nextItem);
                updateRadarUrl(nextItem, days);
              }}
              type="button"
            >
              {days}d
            </button>
          ))}
        </div>
      </div>
      <div className="radar-frame">
        <div className="radar-screen" aria-label={ariaLabel}>
          <span className="radar-bearing n">000</span>
          <span className="radar-bearing e">090</span>
          <span className="radar-bearing s">180</span>
          <span className="radar-bearing w">270</span>
          <span className="range-label r1">{activeWindow.outer}</span>
          <span className="range-label r2">{activeWindow.mid}</span>
          <span className="range-label r3">{activeWindow.inner}</span>
          <span className="radar-density" />
          <span className="radar-sweep" />
          <span className="radar-line" />
          <span className="radar-trace trace-a" />
          <span className="radar-trace trace-b" />
          <span className="radar-trace trace-c" />
          {visibleItems.map((item, index) => (
            <button
              aria-label={`Show ${item.title || item.id}`}
              className={`node ${item.tone || ""}${selectedItem.id === item.id ? " active" : ""}`}
              data-label={item.label || item.id}
              key={item.id}
              onClick={() => {
                setSelectedItem(item);
                updateRadarUrl(item, windowDays);
              }}
              style={{ ...item.position, "--ping-delay": item.pingDelay || `${index * 0.28}s` }}
              type="button"
            />
          ))}
          {visibleEchoes.map((echo, index) => (
            <span
              className={`echo ${echo.tone || ""}`}
              key={`${echo.position.left}-${echo.position.top}-${index}`}
              style={echo.position}
            />
          ))}
        </div>
      </div>

      <div className="radar-readout">
        <div className="readout-card">
          <span className="label">{vectorLabel}</span>
          <span className={`readout-value${toneClass(selectedItem.vectorTone)}`}>{selectedItem.vector || selectedItem.id}</span>
          <span className="micro">{activeWindow.label}</span>
        </div>
        <div className="readout-card">
          <span className="label">{gainLabel}</span>
          <span className={`readout-value${toneClass(selectedItem.gainTone)}`}>{selectedItem.gain || selectedItem.value}</span>
          <span className="micro">{selectedItem.gainMeta || selectedItem.value}</span>
        </div>
        <div className="readout-card selected-scan">
          <span className="label">{detailLabel}</span>
          <span className="readout-value">{selectedItem.detail || selectedItem.title}</span>
          {stats.length > 0 && (
            <div className="selected-grid">
              {stats.map((stat) => (
                <div className="selected-stat" key={stat.label}>
                  <span className="micro">{stat.label}</span>
                  <strong>{stat.value}</strong>
                </div>
              ))}
            </div>
          )}
          <span className="micro">{selectedItem.summary}</span>
        </div>
        {feed.length > 0 && (
          <div className="readout-feed">
            {feed.map((line) => (
              <div className={`readout-line ${line.tone || ""}`} key={line.text}>{line.text}</div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export function PlaceholderSurface({ body, referenceHref = "/prototype-static.html", title }) {
  return (
    <div className="placeholder-shell">
      <div className="eyebrow">// UI v2 extraction queue</div>
      <h1>{title}</h1>
      <p className="subline">{body}</p>
      <div className="panel placeholder-panel">
        <div className="panelhead">
          <div>
            <span className="micro">// Reference</span>
            <div className="paneltitle">Static prototype preserved</div>
          </div>
          <a className="btn" href={referenceHref}>Open reference</a>
        </div>
        <div className="panelbody">
          <p className="muted">
            The Atlas board is now React. The Ledger and Watch Floor directions remain in the static reference until their reusable pieces are extracted.
          </p>
        </div>
      </div>
    </div>
  );
}
