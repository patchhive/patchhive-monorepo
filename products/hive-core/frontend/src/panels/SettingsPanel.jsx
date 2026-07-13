import { useEffect, useState } from "react";
import { Btn, Input, S, Tag } from "@patchhivehq/ui";
import {
  CommandHero,
  CommandPanel,
  SectionHeader,
  commandGridStyle,
  commandPanelStyle,
} from "../components/CommandChrome.jsx";

const textareaStyle = {
  ...S.input,
  minHeight: 88,
  resize: "vertical",
};

function authTone(product) {
  if (product.slug === "hive-core") return "var(--blue)";
  if (product.service_token_configured) return "var(--green)";
  if (product.legacy_api_key_configured) return "var(--gold)";
  return "var(--accent)";
}

function authLabel(product) {
  if (product.slug === "hive-core") return "Native control plane";
  if (product.service_token_configured) return "Service token saved";
  if (product.legacy_api_key_configured) return "Legacy operator key saved";
  return "Service token needed";
}

function ProductEditor({ product, onChange, onProvision }) {
  const [operatorApiKey, setOperatorApiKey] = useState("");
  const [provisioning, setProvisioning] = useState(false);
  const isNative = product.slug === "hive-core";
  const apiTarget = product.override_api_url || product.default_api_url;

  async function provision() {
    setProvisioning(true);
    try {
      await onProvision(product, operatorApiKey);
      setOperatorApiKey("");
    } finally {
      setProvisioning(false);
    }
  }

  return (
    <div style={commandPanelStyle(authTone(product), { display: "grid", gap: 10 })}>
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
        <Tag color={authTone(product)}>{authLabel(product)}</Tag>
        {product.legacy_api_key_configured && !product.service_token_configured && (
          <Tag color="var(--gold)">Replace with service token</Tag>
        )}
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
        {!isNative && (
          <div style={S.field}>
            <div style={S.label}>Service Token</div>
            <Input
              type="password"
              value={product.service_token || ""}
              onChange={(value) => onChange({ ...product, service_token: value })}
              placeholder={
                product.service_token_configured
                  ? "Stored - enter a new token to replace"
                  : product.legacy_api_key_configured
                    ? "Paste a service token to replace the legacy key"
                    : "Paste an existing service token"
              }
            />
          </div>
        )}
      </div>

      {isNative ? (
        <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.55 }}>
          HiveCore uses its own operator session here. It does not need a saved per-product service token to call itself.
        </div>
      ) : (
        <div style={{ display: "grid", gap: 10 }}>
          <div style={{ display: "grid", gap: 10, gridTemplateColumns: "minmax(240px, 1fr) auto" }}>
            <div style={S.field}>
              <div style={S.label}>One-time Operator API Key</div>
              <Input
                type="password"
                value={operatorApiKey}
                onChange={setOperatorApiKey}
                placeholder="Optional manual fallback when suite bootstrap is not available. Not stored."
              />
            </div>
            <div style={{ display: "grid", alignItems: "end" }}>
              <Btn onClick={provision} disabled={provisioning} color="var(--green)">
                {provisioning
                  ? "Working..."
                  : product.service_token_configured
                    ? "Rotate service token"
                    : "Provision service token"}
              </Btn>
            </div>
          </div>

          <div style={{ fontSize: 11, color: "var(--text-dim)", lineHeight: 1.55 }}>
            HiveCore will call <span style={{ color: "var(--text)" }}>{apiTarget}</span> and use
            <code> /auth/generate-service-token </code>
            for first-time setup or
            <code> /auth/rotate-service-token </code>
            when the product already has service auth enabled.
            If suite bootstrap is configured, HiveCore can use that automatically. Otherwise, when the product already requires operator login, the one-time operator API key is used only for that request and is not stored in HiveCore.
          </div>
        </div>
      )}

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
  const [repositoryPolicies, setRepositoryPolicies] = useState(null);
  const [prBudgets, setPrBudgets] = useState(null);
  const [newRepository, setNewRepository] = useState("");
  const [saving, setSaving] = useState(false);
  const [savedMessage, setSavedMessage] = useState("");

  async function refresh() {
    setRunning(true);
    setError("");
    setSavedMessage("");
    try {
      const [settingsData, policyData, budgetData] = await Promise.all([
        fetchEnvelope("/settings"),
        fetchEnvelope("/repository-policies"),
        fetchEnvelope("/pr-budgets"),
      ]);
      setSettings(settingsData);
      setRepositoryPolicies(policyData);
      setPrBudgets(budgetData);
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
            service_token: product.service_token || "",
            enabled: product.enabled,
            notes: product.notes,
          })),
        }),
      });
      const policyData = await fetchEnvelope("/repository-policies", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          policies: repositoryPolicies.policies.map((policy) => ({
            repository: policy.repository,
            trusted: policy.trusted,
            operator_excluded: policy.operator_excluded,
            notes: policy.notes,
          })),
        }),
      });
      const budgetData = await fetchEnvelope("/pr-budgets", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          suite_limit: Number(prBudgets.suite_limit),
          products: prBudgets.products.map((product) => ({
            product: product.product,
            limit: Number(product.limit),
          })),
        }),
      });
      setSettings(data);
      setRepositoryPolicies(policyData);
      setPrBudgets(budgetData);
      setSavedMessage("HiveCore saved suite settings, repository safety, and PR budgets.");
    } catch (err) {
      setError(err.message || "HiveCore could not save suite settings.");
    } finally {
      setSaving(false);
      setRunning(false);
    }
  }

  async function provisionServiceToken(product, operatorApiKey) {
    setRunning(true);
    setError("");
    setSavedMessage("");
    try {
      const data = await fetchEnvelope(`/products/${product.slug}/provision-service-token`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          operator_api_key: operatorApiKey || "",
          api_url: product.override_api_url || "",
        }),
      });

      setSettings((current) => ({
        ...current,
        products: current.products.map((item) =>
          item.slug === data.product.slug ? { ...data.product } : item,
        ),
      }));
      setSavedMessage(data.message || `HiveCore provisioned a service token for ${product.title}.`);
    } catch (err) {
      setError(err.message || "HiveCore could not provision the product service token.");
      throw err;
    } finally {
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

  function addRepositoryPolicy() {
    const repository = newRepository.trim().toLowerCase();
    if (!repository || repositoryPolicies.policies.some((item) => item.repository === repository)) {
      return;
    }
    setRepositoryPolicies((current) => ({
      ...current,
      policies: [
        ...current.policies,
        { repository, trusted: false, operator_excluded: false, notes: "" },
      ],
    }));
    setNewRepository("");
  }

  function updateRepositoryPolicy(index, patch) {
    setRepositoryPolicies((current) => ({
      ...current,
      policies: current.policies.map((policy, policyIndex) =>
        policyIndex === index ? { ...policy, ...patch } : policy,
      ),
    }));
  }

  function removeRepositoryPolicy(index) {
    setRepositoryPolicies((current) => ({
      ...current,
      policies: current.policies.filter((_, policyIndex) => policyIndex !== index),
    }));
  }

  function updateProductBudget(index, value) {
    setPrBudgets((current) => ({
      ...current,
      products: current.products.map((product, productIndex) =>
        productIndex === index ? { ...product, limit: value } : product,
      ),
    }));
  }

  async function releaseReservation(reservation) {
    setRunning(true);
    setError("");
    try {
      await fetchEnvelope(`/pr-budgets/reservations/${reservation.id}/release`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ reason: "Released manually by the HiveCore operator." }),
      });
      setPrBudgets(await fetchEnvelope("/pr-budgets"));
      setSavedMessage(`Released the PR slot for ${reservation.repository}.`);
    } catch (err) {
      setError(err.message || "HiveCore could not release the PR slot.");
    } finally {
      setRunning(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  if (!settings || !repositoryPolicies || !prBudgets) {
    return (
      <div style={{ display: "grid", gap: 16 }}>
        <CommandPanel tone="var(--blue)" style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
          <div>
            <div style={{ fontSize: 18, fontWeight: 800 }}>Settings</div>
            <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
              Persist suite defaults, launch targets, and product service-token provisioning.
            </div>
          </div>
          <Btn onClick={refresh}>Reload</Btn>
        </CommandPanel>
      </div>
    );
  }

  return (
    <div style={{ ...commandGridStyle, gap: 14 }}>
      <CommandHero
        kicker="Policy control"
        title="Suite settings"
        body="Save the defaults HiveCore should hold for the whole PatchHive suite."
        tone="var(--blue)"
        actions={
          <>
          <Btn onClick={refresh} disabled={saving}>
            Reload
          </Btn>
          <Btn onClick={save} disabled={saving} color="var(--green)">
            {saving ? "Saving..." : "Save settings"}
          </Btn>
          </>
        }
      />

      {savedMessage && (
        <CommandPanel tone="var(--green)" style={{ color: "var(--green)", padding: "12px 14px" }}>{savedMessage}</CommandPanel>
      )}

      <CommandPanel tone="var(--accent)" style={{ display: "grid", gap: 12 }}>
        <SectionHeader
          kicker="Global defaults"
          title="Operator policy"
          body="These values become the suite-wide baseline before individual products override them."
        />
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
      </CommandPanel>

      <CommandPanel tone="var(--gold)" style={{ display: "grid", gap: 14 }}>
        <SectionHeader
          kicker="Repository safety"
          title="Trust and exclusions"
          body="Excluded repositories are blocked suite-wide. Trusted repositories may use product operations that are too risky for unknown repositories, including RepoReaper test execution."
          tone="var(--gold)"
        />
        {!repositoryPolicies.public_opt_out_available && (
          <div style={{ ...commandPanelStyle("var(--gold)"), fontSize: 11, color: "var(--text-dim)", lineHeight: 1.55 }}>
            Public owner opt-out is not connected yet. These operator exclusions are enforced locally by HiveCore; the patchhive.dev owner registry remains a later service.
          </div>
        )}
        <div style={{ display: "grid", gap: 10, gridTemplateColumns: "minmax(220px, 1fr) auto" }}>
          <Input value={newRepository} onChange={setNewRepository} placeholder="owner/repository" />
          <Btn onClick={addRepositoryPolicy}>Add repository</Btn>
        </div>
        {repositoryPolicies.policies.length === 0 ? (
          <div style={{ fontSize: 12, color: "var(--text-dim)" }}>No structured repository policies saved yet.</div>
        ) : (
          <div style={{ display: "grid", gap: 10 }}>
            {repositoryPolicies.policies.map((policy, index) => (
              <div key={policy.repository} style={commandPanelStyle(policy.operator_excluded ? "var(--red)" : policy.trusted ? "var(--green)" : "var(--blue)", { display: "grid", gap: 10 })}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap" }}>
                  <strong>{policy.repository}</strong>
                  <div style={{ display: "flex", gap: 12, flexWrap: "wrap", alignItems: "center" }}>
                    <label style={{ fontSize: 11, color: "var(--text-dim)" }}>
                      <input
                        type="checkbox"
                        checked={policy.trusted}
                        onChange={(event) => updateRepositoryPolicy(index, { trusted: event.target.checked, operator_excluded: event.target.checked ? false : policy.operator_excluded })}
                      />{" "}trusted
                    </label>
                    <label style={{ fontSize: 11, color: "var(--text-dim)" }}>
                      <input
                        type="checkbox"
                        checked={policy.operator_excluded}
                        onChange={(event) => updateRepositoryPolicy(index, { operator_excluded: event.target.checked, trusted: event.target.checked ? false : policy.trusted })}
                      />{" "}exclude all automation
                    </label>
                    <Btn onClick={() => removeRepositoryPolicy(index)}>Remove</Btn>
                  </div>
                </div>
                <Input
                  value={policy.notes}
                  onChange={(value) => updateRepositoryPolicy(index, { notes: value })}
                  placeholder="Why this repository is trusted or excluded"
                />
              </div>
            ))}
          </div>
        )}
      </CommandPanel>

      <CommandPanel tone="var(--green)" style={{ display: "grid", gap: 14 }}>
        <SectionHeader
          kicker="Outbound limits"
          title="Pull-request budgets"
          body="A product must have room in both its own budget and the suite-wide ceiling before it can open a pull request. The smaller remaining budget always wins."
          tone="var(--green)"
        />
        <div style={{ display: "grid", gap: 10, gridTemplateColumns: "minmax(220px, 1fr) auto" }}>
          <div style={S.field}>
            <div style={S.label}>Suite-wide PR ceiling</div>
            <Input
              type="number"
              value={String(prBudgets.suite_limit)}
              onChange={(value) => setPrBudgets((current) => ({ ...current, suite_limit: value }))}
            />
          </div>
          <div style={{ display: "flex", alignItems: "end", paddingBottom: 4 }}>
            <Tag color="var(--green)">{prBudgets.suite_used} used · {prBudgets.suite_remaining} remaining</Tag>
          </div>
        </div>
        <div style={{ display: "grid", gap: 10, gridTemplateColumns: "repeat(auto-fit, minmax(210px, 1fr))" }}>
          {prBudgets.products.map((product, index) => (
            <div key={product.product} style={commandPanelStyle("var(--green)", { display: "grid", gap: 8 })}>
              <strong>{product.product}</strong>
              <Input
                type="number"
                value={String(product.limit)}
                onChange={(value) => updateProductBudget(index, value)}
              />
              <div style={{ fontSize: 11, color: "var(--text-dim)" }}>{product.used} used · {product.remaining} remaining</div>
            </div>
          ))}
        </div>
        <div style={{ display: "grid", gap: 8 }}>
          <div style={S.label}>Recent reservations</div>
          {prBudgets.reservations.length === 0 ? (
            <div style={{ fontSize: 12, color: "var(--text-dim)" }}>No PR budget reservations yet.</div>
          ) : prBudgets.reservations.map((reservation) => (
            <div key={reservation.id} style={commandPanelStyle("var(--blue)", { display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" })}>
              <div>
                <strong>{reservation.product} · {reservation.repository}</strong>
                <div style={{ fontSize: 11, color: "var(--text-dim)" }}>{reservation.status}{reservation.pr_url ? ` · ${reservation.pr_url}` : ""}</div>
              </div>
              {(["reserved", "committed"].includes(reservation.status)) && (
                <Btn onClick={() => releaseReservation(reservation)}>Release slot</Btn>
              )}
            </div>
          ))}
        </div>
      </CommandPanel>

      <div style={{ display: "grid", gap: 12 }}>
        <SectionHeader
          kicker="Product endpoints"
          title="Service-token control"
          body="Each product stays standalone, but HiveCore stores the machine credentials needed for orchestration."
          tone="var(--green)"
        />
        {settings.products.map((product, index) => (
          <ProductEditor
            key={product.slug}
            product={product}
            onChange={(nextProduct) => updateProduct(index, nextProduct)}
            onProvision={provisionServiceToken}
          />
        ))}
      </div>
    </div>
  );
}
