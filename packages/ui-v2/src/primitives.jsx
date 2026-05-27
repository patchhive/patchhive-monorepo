import { useEffect, useMemo, useState } from "react";

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
  return (
    <div className="topline">
      {cells.map(({ label, value, tone = "" }) => (
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
          <span className={`big${toneClass(metric.tone)}`}>{metric.value}</span>
          <span className="micro">{metric.sub}</span>
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

  useEffect(() => {
    if (!selectedItem || selectedItem.minWindow > windowDays) {
      setSelectedItem(visibleItems[0] || normalizedItems[0]);
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
            onClick={() => setSelectedItem(item)}
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
      <div className="range-panel">
        <span className="chip signal">{activeWindow.count || `${visibleItems.length} ${signalLabel}`}</span>
        <div className="range-switch" aria-label="Radar history window">
          {windowKeys.map((days) => (
            <button
              className={`range-btn${windowDays === days ? " active" : ""}`}
              key={days}
              onClick={() => setWindowDays(days)}
              type="button"
            >
              {days}d
            </button>
          ))}
        </div>
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
