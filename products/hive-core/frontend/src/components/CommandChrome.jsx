import { S } from "@patchhivehq/ui";

export const commandText = {
  kicker: {
    fontSize: 10,
    letterSpacing: 0,
    textTransform: "uppercase",
    color: "var(--text-dim)",
  },
  title: {
    fontSize: 20,
    lineHeight: 1.08,
    fontWeight: 950,
    letterSpacing: 0,
  },
  body: {
    fontSize: 12,
    color: "var(--text-dim)",
    lineHeight: 1.55,
  },
};

export const commandGridStyle = {
  display: "grid",
  gap: 12,
};

export const tacticalGridStyle = {
  display: "grid",
  gap: 12,
  gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))",
};

export function commandPanelStyle(tone = "var(--border)", extra = {}) {
  return {
    ...S.panel,
    position: "relative",
    overflow: "hidden",
    borderRadius: 8,
    border: `1px solid color-mix(in srgb, ${tone} 24%, var(--border))`,
    background:
      "linear-gradient(180deg, color-mix(in srgb, var(--bg-panel) 94%, #071015) 0%, color-mix(in srgb, var(--bg) 72%, #020509) 100%)",
    boxShadow: "inset 0 1px 0 rgba(255,255,255,0.035)",
    ...extra,
  };
}

export function CommandPanel({ tone = "var(--border)", style, children }) {
  return <div style={commandPanelStyle(tone, style)}>{children}</div>;
}

export function CommandHero({ kicker, title, body, tone = "var(--accent)", actions, children }) {
  return (
    <CommandPanel tone={tone} style={{ display: "grid", gap: 14 }}>
      <div
        aria-hidden="true"
        style={{
          position: "absolute",
          inset: 0,
          background:
            "linear-gradient(90deg, transparent, color-mix(in srgb, var(--accent) 8%, transparent), transparent), repeating-linear-gradient(90deg, rgba(255,255,255,0.022) 0 1px, transparent 1px 42px)",
          pointerEvents: "none",
        }}
      />
      <div style={{ position: "relative", display: "flex", justifyContent: "space-between", gap: 14, flexWrap: "wrap", alignItems: "flex-start" }}>
        <div style={{ display: "grid", gap: 5, maxWidth: 760 }}>
          <div style={{ ...commandText.kicker, color: tone }}>{kicker}</div>
          <div style={{ ...commandText.title, fontSize: 24 }}>{title}</div>
          {body && <div style={commandText.body}>{body}</div>}
        </div>
        {actions && <div style={{ display: "flex", gap: 8, flexWrap: "wrap", justifyContent: "flex-end" }}>{actions}</div>}
      </div>
      {children && <div style={{ position: "relative" }}>{children}</div>}
    </CommandPanel>
  );
}

export function SectionHeader({ kicker, title, body, tone = "var(--accent)", badge }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start", flexWrap: "wrap" }}>
      <div style={{ display: "grid", gap: 4 }}>
        {kicker && <div style={{ ...commandText.kicker, color: tone }}>{kicker}</div>}
        <div style={commandText.title}>{title}</div>
        {body && <div style={commandText.body}>{body}</div>}
      </div>
      {badge}
    </div>
  );
}

export function MetricTile({ label, value, tone = "var(--text)", detail }) {
  return (
    <CommandPanel tone={tone} style={{ display: "grid", gap: 7, minHeight: 96 }}>
      <div style={commandText.kicker}>{label}</div>
      <div style={{ fontSize: 28, lineHeight: 1, fontWeight: 950, color: tone }}>{value}</div>
      {detail && <div style={{ fontSize: 10, color: "var(--text-dim)", lineHeight: 1.4 }}>{detail}</div>}
    </CommandPanel>
  );
}
