import { useEffect, useState } from "react";
import { Btn, EmptyState, S, Tag } from "@patchhivehq/ui";

function statusColor(status) {
  if (status === "online") return "var(--green)";
  if (status === "degraded") return "var(--gold)";
  if (status === "disabled") return "var(--text-dim)";
  if (status === "unconfigured") return "var(--blue)";
  return "var(--accent)";
}

function ProductCard({ product }) {
  const health = product.health || {};

  return (
    <div
      style={{
        ...S.panel,
        display: "grid",
        gap: 10,
        borderColor: `${statusColor(product.status)}55`,
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start" }}>
        <div style={{ display: "grid", gap: 4 }}>
          <div style={{ fontSize: 16, fontWeight: 800 }}>
            {product.icon} {product.title}
          </div>
          <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.5 }}>{product.role}</div>
        </div>
        <Tag color={statusColor(product.status)}>{product.status}</Tag>
      </div>

      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        <Tag color="var(--accent)">{product.lane}</Tag>
        {health.version && <Tag color="var(--blue)">v{health.version}</Tag>}
        {health.startup_warns > 0 && <Tag color="var(--gold)">{health.startup_warns} warn</Tag>}
        {health.startup_errors > 0 && <Tag color="var(--accent)">{health.startup_errors} error</Tag>}
      </div>

      <div style={{ display: "grid", gap: 6, gridTemplateColumns: "repeat(auto-fit, minmax(150px, 1fr))" }}>
        <div>
          <div style={S.label}>Frontend</div>
          <div style={{ fontSize: 11, color: "var(--text-dim)", wordBreak: "break-all" }}>
            {product.frontend_url || "Not configured"}
          </div>
        </div>
        <div>
          <div style={S.label}>API</div>
          <div style={{ fontSize: 11, color: "var(--text-dim)", wordBreak: "break-all" }}>
            {product.api_url || "Not configured"}
          </div>
        </div>
        <div>
          <div style={S.label}>Startup</div>
          <div style={{ fontSize: 11, color: "var(--text-dim)" }}>
            {health.startup_errors || health.startup_warns || health.startup_infos
              ? `${health.startup_errors || 0} error · ${health.startup_warns || 0} warn`
              : "No startup detail yet"}
          </div>
        </div>
        <div>
          <div style={S.label}>Database</div>
          <div style={{ fontSize: 11, color: health.db_ok === false ? "var(--accent)" : "var(--text-dim)" }}>
            {health.db_ok == null ? "Unknown" : health.db_ok ? "Healthy" : "Unhealthy"}
          </div>
        </div>
      </div>

      <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
        {product.frontend_url && (
          <a href={product.frontend_url} target="_blank" rel="noreferrer" style={{ color: "var(--accent)", fontSize: 11 }}>
            Open app
          </a>
        )}
        {product.api_url && (
          <a href={`${product.api_url.replace(/\/$/, "")}/health`} target="_blank" rel="noreferrer" style={{ color: "var(--blue)", fontSize: 11 }}>
            Health
          </a>
        )}
        <a href={`https://github.com/${product.repo}`} target="_blank" rel="noreferrer" style={{ color: "var(--text-dim)", fontSize: 11 }}>
          {product.repo}
        </a>
      </div>

      {product.notes && (
        <div style={{ borderTop: "1px solid var(--border)", paddingTop: 10, fontSize: 11, color: "var(--text-dim)" }}>
          {product.notes}
        </div>
      )}

      {health.error && (
        <div
          style={{
            borderTop: "1px solid var(--border)",
            paddingTop: 10,
            fontSize: 11,
            color: "var(--gold)",
          }}
        >
          {health.error}
        </div>
      )}
    </div>
  );
}

export default function ProductsPanel({ fetchEnvelope, setRunning, setError }) {
  const [products, setProducts] = useState([]);

  async function refresh() {
    setRunning(true);
    setError("");
    try {
      const data = await fetchEnvelope("/products");
      setProducts(data || []);
    } catch (err) {
      setProducts([]);
      setError(err.message || "HiveCore could not load product health.");
    } finally {
      setRunning(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div style={{ ...S.panel, display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
        <div>
          <div style={{ fontSize: 18, fontWeight: 800 }}>Products</div>
          <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
            Live health polling over each product's configured API target.
          </div>
        </div>
        <Btn onClick={refresh}>Refresh products</Btn>
      </div>

      {products.length === 0 ? (
        <EmptyState icon="⬢" text="HiveCore does not have any product status to show yet." />
      ) : (
        <div style={{ display: "grid", gap: 12, gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))" }}>
          {products.map((product) => (
            <ProductCard key={product.slug} product={product} />
          ))}
        </div>
      )}
    </div>
  );
}
