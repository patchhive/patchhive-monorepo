import { useMemo, useState } from "react";
import { EmptyState, S, Sel, Tag } from "@patchhivehq/ui";

const METRICS = [
  { v: "total_signals", l: "Signals", color: "var(--accent)" },
  { v: "total_stale_issues", l: "Stale Issues", color: "var(--gold)" },
  { v: "avg_priority_score", l: "Average Priority", color: "var(--green)" },
  { v: "top_priority_score", l: "Top Priority", color: "var(--accent)" },
];

function shortDate(value) {
  const date = new Date(value);
  return `${date.getMonth() + 1}/${date.getDate()}`;
}

export default function ScanTimelineChart({ timeline }) {
  const [metric, setMetric] = useState("total_signals");
  const meta = METRICS.find((item) => item.v === metric) || METRICS[0];
  const points = timeline?.points || [];

  const chart = useMemo(() => {
    if (points.length === 0) {
      return null;
    }

    const width = 760;
    const height = 220;
    const padding = 26;
    const values = points.map((point) => Number(point[metric] || 0));
    const min = Math.min(...values);
    const max = Math.max(...values);
    const span = max - min || 1;

    const dots = points.map((point, index) => {
      const x = padding + (index * (width - padding * 2)) / Math.max(points.length - 1, 1);
      const normalized = (Number(point[metric] || 0) - min) / span;
      const y = height - padding - normalized * (height - padding * 2);
      return { ...point, x, y, value: Number(point[metric] || 0) };
    });

    const line = dots.map((dot) => `${dot.x},${dot.y}`).join(" ");
    return { width, height, padding, min, max, dots, line };
  }, [metric, points]);

  if (!timeline || points.length === 0) {
    return <EmptyState icon="◌" text="Run more matching scans to build a trend timeline." />;
  }

  if (!chart || points.length === 1) {
    return (
      <div style={{ ...S.panel, display: "grid", gap: 12 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
          <div>
            <div style={{ fontSize: 15, fontWeight: 700 }}>Timeline</div>
            <div style={{ color: "var(--text-dim)", fontSize: 11 }}>SignalHive will fill this in as more similar scans accumulate.</div>
          </div>
          <Tag>{points.length} scan</Tag>
        </div>
        <EmptyState icon="◎" text="One matching scan is saved so far. Run another one with the same parameters to unlock trend visuals." />
      </div>
    );
  }

  return (
    <div style={{ ...S.panel, display: "grid", gap: 12 }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-end", flexWrap: "wrap" }}>
        <div style={{ display: "grid", gap: 4 }}>
          <div style={{ fontSize: 15, fontWeight: 700 }}>Timeline</div>
          <div style={{ color: "var(--text-dim)", fontSize: 11 }}>
            Showing the last {points.length} scans with the same parameters.
          </div>
        </div>
        <div style={{ minWidth: 200, ...S.field }}>
          <div style={S.label}>Metric</div>
          <Sel value={metric} onChange={setMetric} opts={METRICS} />
        </div>
      </div>

      <svg viewBox={`0 0 ${chart.width} ${chart.height}`} style={{ width: "100%", height: "auto", overflow: "visible" }}>
        {[0, 1, 2, 3].map((tick) => {
          const y = chart.padding + (tick * (chart.height - chart.padding * 2)) / 3;
          return (
            <line
              key={`grid-${tick}`}
              x1={chart.padding}
              y1={y}
              x2={chart.width - chart.padding}
              y2={y}
              stroke="var(--border)"
              strokeDasharray="3 6"
            />
          );
        })}

        <polyline
          fill="none"
          stroke={meta.color}
          strokeWidth="3"
          strokeLinejoin="round"
          strokeLinecap="round"
          points={chart.line}
        />

        {chart.dots.map((dot, index) => {
          const active = dot.id === timeline.current_scan_id;
          return (
            <g key={dot.id}>
              <circle
                cx={dot.x}
                cy={dot.y}
                r={active ? 6 : 4}
                fill={active ? "var(--accent)" : meta.color}
                stroke="var(--bg-panel, #0f1520)"
                strokeWidth="2"
              />
              <text x={dot.x} y={chart.height - 6} textAnchor="middle" fill="var(--text-dim)" fontSize="10">
                {shortDate(dot.created_at)}
              </text>
              {active && (
                <text x={dot.x} y={dot.y - 12} textAnchor="middle" fill="var(--text)" fontSize="10">
                  {Math.round(dot.value * 10) / 10}
                </text>
              )}
            </g>
          );
        })}

        <text x="6" y={chart.padding + 4} fill="var(--text-dim)" fontSize="10">
          {Math.round(chart.max * 10) / 10}
        </text>
        <text x="6" y={chart.height - chart.padding + 4} fill="var(--text-dim)" fontSize="10">
          {Math.round(chart.min * 10) / 10}
        </text>
      </svg>

      <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
        <Tag color={meta.color}>{meta.l}</Tag>
        <Tag>Current {Math.round((points[points.length - 1]?.[metric] || 0) * 10) / 10}</Tag>
      </div>
    </div>
  );
}
