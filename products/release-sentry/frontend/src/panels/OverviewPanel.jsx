import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, S, Tag } from "@patchhivehq/ui";

export default function OverviewPanel({ apiKey }) {
  const [overview, setOverview] = useState(null);
  const fetch_ = createApiFetcher(apiKey);

  const refresh = () => {
    fetch_(`${API}/overview`)
      .then((res) => res.json())
      .then(setOverview)
      .catch(() => setOverview(null));
  };

  useEffect(() => {
    refresh();
  }, [apiKey]);

  return (
    <div style={{ display: "grid", gap: 18 }}>
        <div style={{ ...S.panel, display: "grid", gap: 10 }}>
        <div style={{ fontSize: 18, fontWeight: 700 }}>Release Readiness</div>
        <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>
          ReleaseSentry will gather the proof behind a release decision: CI health, tag/version/changelog drift,
          dependency and security pressure, open blockers, and what changed since the last ship point.
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <Tag color="var(--accent)">ready / watch / hold</Tag>
          <Tag color="var(--blue)">release evidence</Tag>
          <Tag color="var(--gold)">read-only first</Tag>
        </div>
        <div>
          <Btn onClick={refresh}>Refresh Overview</Btn>
        </div>
      </div>

      {overview ? (
        <div style={{ ...S.panel, display: "grid", gap: 10 }}>
          <div style={{ fontSize: 16, fontWeight: 700 }}>{overview.product}</div>
          <div style={{ color: "var(--accent)", fontSize: 12 }}>{overview.tagline}</div>
          <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{overview.message}</div>
        </div>
      ) : (
        <EmptyState icon="🚦" text="No release readiness payload was returned yet." />
      )}
    </div>
  );
}
