export function toneClass(tone) {
  return tone ? ` ${tone}` : "";
}

export function DeckBar({
  activeTab,
  brandEyebrow = "PatchHive v2 track",
  brandName,
  navLabel = "PatchHive v2 surfaces",
  onTabChange,
  tabs,
}) {
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
