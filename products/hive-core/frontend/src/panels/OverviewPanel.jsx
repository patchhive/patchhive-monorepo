import { useEffect, useState } from "react";
import { Btn, EmptyState, S, Tag } from "@patchhivehq/ui";

const heroGridStyle = {
  display: "grid",
  gap: 14,
  gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))",
};

const metricGridStyle = {
  display: "grid",
  gap: 12,
  gridTemplateColumns: "repeat(auto-fit, minmax(140px, 1fr))",
};

function statusColor(status) {
  if (status === "online") return "var(--green)";
  if (status === "degraded") return "var(--gold)";
  if (status === "disabled") return "var(--text-dim)";
  if (status === "unconfigured") return "var(--blue)";
  return "var(--accent)";
}

function MetricCard({ label, value, tone = "var(--text)" }) {
  return (
    <div style={{ ...S.panel, display: "grid", gap: 6 }}>
      <div style={S.label}>{label}</div>
      <div style={{ fontSize: 26, fontWeight: 800, color: tone }}>{value}</div>
    </div>
  );
}

function ProductStrip({ product }) {
  return (
    <div
      style={{
        ...S.panel,
        display: "grid",
        gap: 8,
        borderColor: `${statusColor(product.status)}55`,
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", gap: 10, alignItems: "flex-start" }}>
        <div>
          <div style={{ fontSize: 15, fontWeight: 700, color: "var(--text)" }}>
            {product.icon} {product.title}
          </div>
          <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.5 }}>{product.role}</div>
        </div>
        <Tag color={statusColor(product.status)}>{product.status}</Tag>
      </div>
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        <Tag color="var(--accent)">{product.lane}</Tag>
        {product.health.startup_warns > 0 && (
          <Tag color="var(--gold)">
            {product.health.startup_warns} startup warn{product.health.startup_warns === 1 ? "" : "s"}
          </Tag>
        )}
        {product.health.startup_errors > 0 && (
          <Tag color="var(--accent)">
            {product.health.startup_errors} startup error{product.health.startup_errors === 1 ? "" : "s"}
          </Tag>
        )}
      </div>
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        {product.frontend_url && (
          <a href={product.frontend_url} target="_blank" rel="noreferrer" style={{ color: "var(--accent)", fontSize: 11 }}>
            Open app
          </a>
        )}
        {product.api_url && (
          <a href={product.api_url} target="_blank" rel="noreferrer" style={{ color: "var(--blue)", fontSize: 11 }}>
            API
          </a>
        )}
        <a
          href={`https://github.com/${product.repo}`}
          target="_blank"
          rel="noreferrer"
          style={{ color: "var(--text-dim)", fontSize: 11 }}
        >
          {product.repo}
        </a>
      </div>
    </div>
  );
}

export default function OverviewPanel({ fetchEnvelope, setRunning, setError }) {
  const [overview, setOverview] = useState(null);

  async function refresh() {
    setRunning(true);
    setError("");
    try {
      const data = await fetchEnvelope("/overview");
      setOverview(data);
    } catch (err) {
      setOverview(null);
      setError(err.message || "HiveCore could not load the suite overview.");
    } finally {
      setRunning(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  if (!overview) {
    return (
      <div style={{ display: "grid", gap: 16 }}>
        <div style={{ ...S.panel, display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
          <div>
            <div style={{ fontSize: 18, fontWeight: 800 }}>Suite Overview</div>
            <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
              Live product health, suite defaults, and quick launch context.
            </div>
          </div>
          <Btn onClick={refresh}>Refresh</Btn>
        </div>
        <EmptyState icon="⬢" text="HiveCore has not loaded the suite snapshot yet." />
      </div>
    );
  }

  const preferred = overview.products.find(
    (product) => product.slug === overview.suite_settings.preferred_launch_product,
  );
  const featured = preferred || overview.products.find((product) => product.status === "online") || overview.products[0];
  const unstable = overview.products.filter((product) => product.status === "degraded" || product.status === "offline");

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div style={{ ...heroGridStyle, alignItems: "stretch" }}>
        <div
          style={{
            ...S.panel,
            display: "grid",
            gap: 12,
            background:
              "linear-gradient(135deg, color-mix(in srgb, var(--accent) 20%, var(--bg-panel)) 0%, var(--bg-panel) 55%, color-mix(in srgb, var(--blue) 14%, var(--bg-panel)) 100%)",
          }}
        >
          <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
            <div>
              <div style={{ fontSize: 22, fontWeight: 900, letterSpacing: "-0.03em" }}>{overview.suite_settings.operator_label}</div>
              <div style={{ fontSize: 12, color: "var(--accent)" }}>{overview.tagline}</div>
            </div>
            <Btn onClick={refresh}>Refresh suite</Btn>
          </div>
          <div style={{ fontSize: 13, lineHeight: 1.6, color: "var(--text-dim)" }}>{overview.suite_settings.mission}</div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            {overview.suite_settings.default_topics
              .split(",")
              .map((item) => item.trim())
              .filter(Boolean)
              .slice(0, 5)
              .map((item) => (
                <Tag key={item} color="var(--accent)">
                  {item}
                </Tag>
              ))}
          </div>
          {featured && (
            <div
              style={{
                display: "grid",
                gap: 6,
                borderTop: "1px solid var(--border)",
                paddingTop: 12,
              }}
            >
              <div style={S.label}>Preferred launch</div>
              <div style={{ fontSize: 16, fontWeight: 800 }}>
                {featured.icon} {featured.title}
              </div>
              <div style={{ fontSize: 12, color: "var(--text-dim)" }}>{featured.role}</div>
              {featured.frontend_url && (
                <a href={featured.frontend_url} target="_blank" rel="noreferrer" style={{ color: "var(--accent)", fontSize: 11 }}>
                  Open {featured.title}
                </a>
              )}
            </div>
          )}
        </div>

        <div style={{ ...S.panel, display: "grid", gap: 10 }}>
          <div style={{ fontSize: 16, fontWeight: 800 }}>Guardrails</div>
          <div style={{ fontSize: 12, color: "var(--text-dim)", lineHeight: 1.6 }}>
            HiveCore stores suite defaults here first. Product-by-product adoption comes next.
          </div>
          <div style={{ display: "grid", gap: 8 }}>
            <div>
              <div style={S.label}>Languages</div>
              <div style={{ fontSize: 12 }}>{overview.suite_settings.default_languages || "Not set"}</div>
            </div>
            <div>
              <div style={S.label}>Allowlist</div>
              <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
                {overview.suite_settings.repo_allowlist || "No explicit allowlist saved yet."}
              </div>
            </div>
            <div>
              <div style={S.label}>Denylist</div>
              <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
                {overview.suite_settings.repo_denylist || "No explicit denylist saved yet."}
              </div>
            </div>
          </div>
        </div>
      </div>

      <div style={metricGridStyle}>
        <MetricCard label="Products" value={overview.summary.total_products} />
        <MetricCard label="Online" value={overview.summary.online_products} tone="var(--green)" />
        <MetricCard label="Degraded" value={overview.summary.degraded_products} tone="var(--gold)" />
        <MetricCard label="Offline" value={overview.summary.offline_products} tone="var(--accent)" />
        <MetricCard label="Disabled" value={overview.summary.disabled_products} tone="var(--text-dim)" />
      </div>

      <div style={{ display: "grid", gap: 10 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
          <div>
            <div style={{ fontSize: 18, fontWeight: 800 }}>Launch Surface</div>
            <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
              Quick links into the specialist products, ordered by suite flow.
            </div>
          </div>
          <Tag color={unstable.length > 0 ? "var(--gold)" : "var(--green)"}>
            {unstable.length > 0 ? `${unstable.length} product${unstable.length === 1 ? "" : "s"} need attention` : "Suite looks steady"}
          </Tag>
        </div>
        <div style={{ display: "grid", gap: 12, gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))" }}>
          {overview.products.map((product) => (
            <ProductStrip key={product.slug} product={product} />
          ))}
        </div>
      </div>

      {overview.suite_settings.notes && (
        <div style={{ ...S.panel, display: "grid", gap: 8 }}>
          <div style={S.label}>Operator Notes</div>
          <div style={{ fontSize: 12, color: "var(--text-dim)", lineHeight: 1.6 }}>
            {overview.suite_settings.notes}
          </div>
        </div>
      )}
    </div>
  );
}
