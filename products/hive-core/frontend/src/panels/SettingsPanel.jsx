import { useEffect, useState } from "react";
import { Btn, Input, S, Tag } from "@patchhivehq/ui";

const textareaStyle = {
  ...S.input,
  minHeight: 88,
  resize: "vertical",
};

function ProductEditor({ product, onChange }) {
  return (
    <div style={{ ...S.panel, display: "grid", gap: 10 }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 10, alignItems: "flex-start" }}>
        <div style={{ display: "grid", gap: 3 }}>
          <div style={{ fontSize: 15, fontWeight: 800 }}>
            {product.icon} {product.title}
          </div>
          <div style={{ fontSize: 11, color: "var(--text-dim)" }}>{product.role}</div>
        </div>
        <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 11, color: "var(--text-dim)" }}>
          <input
            type="checkbox"
            checked={product.enabled}
            onChange={(event) => onChange({ ...product, enabled: event.target.checked })}
          />
          enabled
        </label>
      </div>

      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        <Tag color="var(--accent)">{product.lane}</Tag>
        <Tag color="var(--blue)">default UI {product.default_frontend_url}</Tag>
        <Tag color="var(--blue)">default API {product.default_api_url}</Tag>
        <Tag color={product.api_key_configured ? "var(--green)" : "var(--gold)"}>
          {product.api_key_configured ? "Access token saved" : "Access token needed"}
        </Tag>
      </div>

      <div style={{ display: "grid", gap: 10, gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))" }}>
        <div style={S.field}>
          <div style={S.label}>Frontend Override</div>
          <Input
            value={product.override_frontend_url}
            onChange={(value) => onChange({ ...product, override_frontend_url: value })}
            placeholder={product.default_frontend_url}
          />
        </div>
        <div style={S.field}>
          <div style={S.label}>API Override</div>
          <Input
            value={product.override_api_url}
            onChange={(value) => onChange({ ...product, override_api_url: value })}
            placeholder={product.default_api_url}
          />
        </div>
        <div style={S.field}>
          <div style={S.label}>Product Access Token</div>
          <Input
            type="password"
            value={product.api_key || ""}
            onChange={(value) => onChange({ ...product, api_key: value })}
            placeholder={
              product.api_key_configured
                ? "Stored - enter a new token to replace"
                : "Paste a service token or legacy API key"
            }
          />
        </div>
      </div>

      <div style={S.field}>
        <div style={S.label}>Notes</div>
        <textarea
          value={product.notes}
          onChange={(event) => onChange({ ...product, notes: event.target.value })}
          style={{ ...textareaStyle, minHeight: 68 }}
          placeholder="Subdomain, environment, or operator notes for this product."
        />
      </div>
    </div>
  );
}

export default function SettingsPanel({ fetchEnvelope, setRunning, setError }) {
  const [settings, setSettings] = useState(null);
  const [saving, setSaving] = useState(false);
  const [savedMessage, setSavedMessage] = useState("");

  async function refresh() {
    setRunning(true);
    setError("");
    setSavedMessage("");
    try {
      const data = await fetchEnvelope("/settings");
      setSettings(data);
    } catch (err) {
      setSettings(null);
      setError(err.message || "HiveCore could not load suite settings.");
    } finally {
      setRunning(false);
    }
  }

  async function save() {
    if (!settings) return;
    setSaving(true);
    setRunning(true);
    setError("");
    setSavedMessage("");
    try {
      const data = await fetchEnvelope("/settings", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          suite_settings: {
            operator_label: settings.suite_settings.operator_label,
            mission: settings.suite_settings.mission,
            default_topics: settings.suite_settings.default_topics,
            default_languages: settings.suite_settings.default_languages,
            repo_allowlist: settings.suite_settings.repo_allowlist,
            repo_denylist: settings.suite_settings.repo_denylist,
            opt_out_notes: settings.suite_settings.opt_out_notes,
            preferred_launch_product: settings.suite_settings.preferred_launch_product,
            notes: settings.suite_settings.notes,
          },
          products: settings.products.map((product) => ({
            slug: product.slug,
            frontend_url: product.override_frontend_url,
            api_url: product.override_api_url,
            api_key: product.api_key || "",
            enabled: product.enabled,
            notes: product.notes,
          })),
        }),
      });
      setSettings(data);
      setSavedMessage("HiveCore saved the suite settings.");
    } catch (err) {
      setError(err.message || "HiveCore could not save suite settings.");
    } finally {
      setSaving(false);
      setRunning(false);
    }
  }

  function updateSuiteField(field, value) {
    setSettings((current) => ({
      ...current,
      suite_settings: {
        ...current.suite_settings,
        [field]: value,
      },
    }));
  }

  function updateProduct(index, nextProduct) {
    setSettings((current) => ({
      ...current,
      products: current.products.map((product, productIndex) =>
        productIndex === index ? nextProduct : product,
      ),
    }));
  }

  useEffect(() => {
    refresh();
  }, []);

  if (!settings) {
    return (
      <div style={{ display: "grid", gap: 16 }}>
        <div style={{ ...S.panel, display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
          <div>
            <div style={{ fontSize: 18, fontWeight: 800 }}>Settings</div>
            <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
              Persist suite defaults, launch targets, subdomain overrides, and product access tokens.
            </div>
          </div>
          <Btn onClick={refresh}>Reload</Btn>
        </div>
      </div>
    );
  }

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div style={{ ...S.panel, display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
        <div>
          <div style={{ fontSize: 18, fontWeight: 800 }}>Suite Settings</div>
          <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
            Save the defaults HiveCore should hold for the whole PatchHive suite.
          </div>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <Btn onClick={refresh} disabled={saving}>
            Reload
          </Btn>
          <Btn onClick={save} disabled={saving} color="var(--green)">
            {saving ? "Saving..." : "Save settings"}
          </Btn>
        </div>
      </div>

      {savedMessage && (
        <div style={{ ...S.panel, color: "var(--green)", padding: "12px 14px" }}>{savedMessage}</div>
      )}

      <div style={{ ...S.panel, display: "grid", gap: 12 }}>
        <div style={{ display: "grid", gap: 10, gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))" }}>
          <div style={S.field}>
            <div style={S.label}>Operator Label</div>
            <Input
              value={settings.suite_settings.operator_label}
              onChange={(value) => updateSuiteField("operator_label", value)}
              placeholder="PatchHive operator"
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Preferred Launch Product</div>
            <select
              value={settings.suite_settings.preferred_launch_product}
              onChange={(event) => updateSuiteField("preferred_launch_product", event.target.value)}
              style={S.select}
            >
              {settings.products.map((product) => (
                <option key={product.slug} value={product.slug}>
                  {product.title}
                </option>
              ))}
            </select>
          </div>
        </div>

        <div style={S.field}>
          <div style={S.label}>Mission</div>
          <textarea
            value={settings.suite_settings.mission}
            onChange={(event) => updateSuiteField("mission", event.target.value)}
            style={textareaStyle}
          />
        </div>

        <div style={{ display: "grid", gap: 10, gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))" }}>
          <div style={S.field}>
            <div style={S.label}>Default Topics</div>
            <Input
              value={settings.suite_settings.default_topics}
              onChange={(value) => updateSuiteField("default_topics", value)}
              placeholder="developer tooling, ci reliability"
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Default Languages</div>
            <Input
              value={settings.suite_settings.default_languages}
              onChange={(value) => updateSuiteField("default_languages", value)}
              placeholder="rust,typescript,python"
            />
          </div>
        </div>

        <div style={{ display: "grid", gap: 10, gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))" }}>
          <div style={S.field}>
            <div style={S.label}>Repo Allowlist</div>
            <textarea
              value={settings.suite_settings.repo_allowlist}
              onChange={(event) => updateSuiteField("repo_allowlist", event.target.value)}
              style={{ ...textareaStyle, minHeight: 72 }}
              placeholder="org/repo, org2/repo2"
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Repo Denylist</div>
            <textarea
              value={settings.suite_settings.repo_denylist}
              onChange={(event) => updateSuiteField("repo_denylist", event.target.value)}
              style={{ ...textareaStyle, minHeight: 72 }}
              placeholder="org/do-not-touch"
            />
          </div>
        </div>

        <div style={S.field}>
          <div style={S.label}>Opt-out Notes</div>
          <textarea
            value={settings.suite_settings.opt_out_notes}
            onChange={(event) => updateSuiteField("opt_out_notes", event.target.value)}
            style={{ ...textareaStyle, minHeight: 72 }}
          />
        </div>

        <div style={S.field}>
          <div style={S.label}>Operator Notes</div>
          <textarea
            value={settings.suite_settings.notes}
            onChange={(event) => updateSuiteField("notes", event.target.value)}
            style={{ ...textareaStyle, minHeight: 88 }}
          />
        </div>
      </div>

      <div style={{ display: "grid", gap: 12 }}>
        {settings.products.map((product, index) => (
          <ProductEditor
            key={product.slug}
            product={product}
            onChange={(nextProduct) => updateProduct(index, nextProduct)}
          />
        ))}
      </div>
    </div>
  );
}
