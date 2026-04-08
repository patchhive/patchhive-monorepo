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
        <div style={{ fontSize: 18, fontWeight: 700 }}>Starter Overview</div>
        <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>
          This panel exists so new products start from a working PatchHive shell instead of a blank page. Replace this
          copy and route with the real product loop as soon as the product direction settles.
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <Tag color="var(--accent)">shared starter</Tag>
          <Tag color="var(--blue)">replace early</Tag>
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
        <EmptyState icon="◌" text="No overview payload was returned yet." />
      )}
    </div>
  );
}
