import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, Input, S, Tag } from "@patchhivehq/ui";

const DEFAULT_RULE_FORM = {
  repo: "",
  blocked_paths: ".github/workflows/, infra/, terraform/, migrations/, schema.sql",
  warn_paths: "auth/, permissions, billing, Dockerfile, docker-compose",
  require_test_for_paths: "src/, app/, lib/, server/, backend/",
  test_paths: "tests/, __tests__/, .test., .spec.",
  suspicious_terms: "TODO, FIXME, skip ci, eval(, exec(, unsafe, curl | sh, rm -rf, password, secret, token",
  blocked_terms: "BEGIN PRIVATE KEY, PRIVATE KEY-----, ghp_, github_pat_, sk-, AKIA",
  max_files: "12",
  max_additions: "400",
  max_deletions: "250",
  notes: "",
};

const LOCAL_DEFAULT_TEMPLATE_FORM = {
  repo: "",
  check_title_template: "TrustGate: {{recommendation_upper}}",
  check_summary_template:
    "{{emoji}} TrustGate recommends **{{recommendation_upper}}** for this PR.\n\n{{summary}}\n\nFiles changed: **{{files_changed}}**  |  Additions: **+{{additions}}**  |  Deletions: **-{{deletions}}**  |  Tests changed: **{{tests_changed}}**  |  Generated files: **{{generated_files}}**",
  check_text_template: "{{findings_plaintext}}",
  comment_template:
    "## {{emoji}} TrustGate: {{recommendation_upper}}\n\n{{summary}}\n\n### Risk snapshot\n- Risk score: **{{risk_score}}**\n- Files changed: **{{files_changed}}**\n- Additions / deletions: **+{{additions}} / -{{deletions}}**\n- Tests changed: **{{tests_changed}}**\n- Generated files: **{{generated_files}}**\n- Blocking findings: **{{blocked_findings}}**\n- Warning findings: **{{warning_findings}}**\n\n### Findings\n{{findings_markdown}}\n\n### File hotspots\n{{file_hotspots_markdown}}\n\n### Next move\n{{next_move}}\n\n{{details_markdown}}\n\n*TrustGate by PatchHive*",
  notes: "",
};

function splitList(value) {
  return value
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean);
}

function joinList(value = []) {
  return value.join(", ");
}

function toRulePayload(form) {
  return {
    repo: form.repo.trim(),
    blocked_paths: splitList(form.blocked_paths),
    warn_paths: splitList(form.warn_paths),
    require_test_for_paths: splitList(form.require_test_for_paths),
    test_paths: splitList(form.test_paths),
    suspicious_terms: splitList(form.suspicious_terms),
    blocked_terms: splitList(form.blocked_terms),
    max_files: Number(form.max_files) || 12,
    max_additions: Number(form.max_additions) || 400,
    max_deletions: Number(form.max_deletions) || 250,
    notes: form.notes.trim(),
  };
}

function fromRules(ruleSet) {
  return {
    repo: ruleSet.repo || "",
    blocked_paths: joinList(ruleSet.blocked_paths),
    warn_paths: joinList(ruleSet.warn_paths),
    require_test_for_paths: joinList(ruleSet.require_test_for_paths),
    test_paths: joinList(ruleSet.test_paths),
    suspicious_terms: joinList(ruleSet.suspicious_terms),
    blocked_terms: joinList(ruleSet.blocked_terms),
    max_files: String(ruleSet.max_files ?? 12),
    max_additions: String(ruleSet.max_additions ?? 400),
    max_deletions: String(ruleSet.max_deletions ?? 250),
    notes: ruleSet.notes || "",
  };
}

function toTemplatePayload(form) {
  return {
    repo: form.repo.trim(),
    check_title_template: form.check_title_template,
    check_summary_template: form.check_summary_template,
    check_text_template: form.check_text_template,
    comment_template: form.comment_template,
    notes: form.notes.trim(),
  };
}

function fromTemplates(templateSet) {
  return {
    repo: templateSet.repo || "",
    check_title_template:
      templateSet.check_title_template || LOCAL_DEFAULT_TEMPLATE_FORM.check_title_template,
    check_summary_template:
      templateSet.check_summary_template || LOCAL_DEFAULT_TEMPLATE_FORM.check_summary_template,
    check_text_template:
      templateSet.check_text_template ?? LOCAL_DEFAULT_TEMPLATE_FORM.check_text_template,
    comment_template: templateSet.comment_template || LOCAL_DEFAULT_TEMPLATE_FORM.comment_template,
    notes: templateSet.notes || "",
  };
}

export default function RulesPanel({ apiKey, initialRepo }) {
  const [form, setForm] = useState(DEFAULT_RULE_FORM);
  const [templateForm, setTemplateForm] = useState(LOCAL_DEFAULT_TEMPLATE_FORM);
  const [templateDefaults, setTemplateDefaults] = useState(LOCAL_DEFAULT_TEMPLATE_FORM);
  const [rules, setRules] = useState([]);
  const [templates, setTemplates] = useState([]);
  const [packs, setPacks] = useState([]);
  const [variables, setVariables] = useState([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const fetch_ = createApiFetcher(apiKey);

  const setRuleField = (key, value) => setForm((prev) => ({ ...prev, [key]: value }));
  const setTemplateField = (key, value) =>
    setTemplateForm((prev) => ({ ...prev, [key]: value }));

  const setRepoValue = (value) => {
    setForm((prev) => ({ ...prev, repo: value }));
    setTemplateForm((prev) => ({ ...prev, repo: value }));
  };

  const refresh = async () => {
    try {
      const [rulesRes, packsRes, templatesRes] = await Promise.all([
        fetch_(`${API}/rules`).then((res) => res.json()),
        fetch_(`${API}/rule-packs`).then((res) => res.json()),
        fetch_(`${API}/templates`).then((res) => res.json()),
      ]);
      setRules(rulesRes.rules || []);
      setPacks(packsRes.packs || []);
      setTemplates(templatesRes.templates || []);
      setTemplateDefaults(fromTemplates(templatesRes.defaults || LOCAL_DEFAULT_TEMPLATE_FORM));
      setVariables(templatesRes.variables || []);
    } catch {
      setRules([]);
      setPacks([]);
      setTemplates([]);
      setVariables([]);
      setTemplateDefaults(LOCAL_DEFAULT_TEMPLATE_FORM);
    }
  };

  useEffect(() => {
    refresh();
  }, [apiKey]);

  useEffect(() => {
    if (initialRepo && !form.repo && !templateForm.repo) {
      setRepoValue(initialRepo);
    }
  }, [initialRepo, form.repo, templateForm.repo]);

  const applyPack = (pack) => {
    const next = fromRules(pack.rules || {});
    next.repo = form.repo;
    setForm(next);
  };

  const loadRuleSet = (item) => {
    setForm(fromRules(item.rules));
    const matchingTemplate = templates.find((entry) => entry.repo === item.repo);
    if (matchingTemplate) {
      setTemplateForm(fromTemplates(matchingTemplate.templates));
    } else {
      setTemplateForm((prev) => ({ ...templateDefaults, repo: item.repo || prev.repo }));
    }
  };

  const loadTemplateSet = (item) => {
    setTemplateForm(fromTemplates(item.templates));
    const matchingRules = rules.find((entry) => entry.repo === item.repo);
    if (matchingRules) {
      setForm(fromRules(matchingRules.rules));
    } else {
      setForm((prev) => ({ ...prev, repo: item.repo }));
    }
  };

  const saveRules = async () => {
    if (!form.repo.trim()) {
      setError("TrustGate needs an owner/repo before it can save rules.");
      return;
    }

    setBusy(true);
    setError("");
    try {
      const res = await fetch_(`${API}/rules`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(toRulePayload(form)),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "TrustGate could not save these rules.");
      }
      await refresh();
      setRepoValue(data.repo || form.repo.trim());
    } catch (err) {
      setError(err.message || "TrustGate could not save these rules.");
    } finally {
      setBusy(false);
    }
  };

  const saveTemplates = async () => {
    if (!templateForm.repo.trim()) {
      setError("TrustGate needs an owner/repo before it can save templates.");
      return;
    }

    setBusy(true);
    setError("");
    try {
      const res = await fetch_(`${API}/templates`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(toTemplatePayload(templateForm)),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "TrustGate could not save these templates.");
      }
      await refresh();
      setRepoValue(data.repo || templateForm.repo.trim());
    } catch (err) {
      setError(err.message || "TrustGate could not save these templates.");
    } finally {
      setBusy(false);
    }
  };

  const removeRules = async (repo) => {
    setBusy(true);
    setError("");
    try {
      const res = await fetch_(`${API}/rules/${encodeURIComponent(repo)}`, {
        method: "DELETE",
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "TrustGate could not delete this rule set.");
      }
      if (form.repo === repo) {
        setForm(DEFAULT_RULE_FORM);
      }
      await refresh();
    } catch (err) {
      setError(err.message || "TrustGate could not delete this rule set.");
    } finally {
      setBusy(false);
    }
  };

  const removeTemplates = async (repo) => {
    setBusy(true);
    setError("");
    try {
      const res = await fetch_(`${API}/templates/${encodeURIComponent(repo)}`, {
        method: "DELETE",
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "TrustGate could not delete this template set.");
      }
      if (templateForm.repo === repo) {
        setTemplateForm({ ...templateDefaults, repo: repo });
      }
      await refresh();
    } catch (err) {
      setError(err.message || "TrustGate could not delete this template set.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div style={{ display: "grid", gridTemplateColumns: "1.2fr 0.8fr", gap: 18 }}>
      <div style={{ ...S.panel, display: "grid", gap: 18 }}>
        <div>
          <div style={{ fontSize: 18, fontWeight: 700 }}>Repo Rule Set</div>
          <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>
            Repo-specific rules are TrustGate&apos;s core memory. They define which file paths are blocked,
            which paths are merely sensitive, what counts as suspicious content, and how large a diff is
            allowed to grow before it should be escalated.
          </div>
        </div>

        {packs.length > 0 && (
          <div style={{ display: "grid", gap: 10 }}>
            <div style={{ fontSize: 14, fontWeight: 700 }}>Starter Rule Packs</div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: 10 }}>
              {packs.map((pack) => (
                <div
                  key={pack.id}
                  style={{
                    border: "1px solid var(--border)",
                    borderRadius: 8,
                    padding: 12,
                    display: "grid",
                    gap: 8,
                  }}
                >
                  <div style={{ display: "flex", justifyContent: "space-between", gap: 8, flexWrap: "wrap" }}>
                    <div style={{ fontWeight: 700 }}>{pack.label}</div>
                    <Tag color="var(--blue)">{pack.id}</Tag>
                  </div>
                  <div style={{ color: "var(--text-dim)", fontSize: 11, lineHeight: 1.5 }}>{pack.description}</div>
                  <Btn onClick={() => applyPack(pack)} color="var(--blue)">
                    Apply Pack
                  </Btn>
                </div>
              ))}
            </div>
          </div>
        )}

        {error && (
          <div
            style={{
              border: "1px solid var(--accent)44",
              background: "var(--accent)10",
              color: "var(--accent)",
              borderRadius: 8,
              padding: "10px 12px",
            }}
          >
            {error}
          </div>
        )}

        <div style={S.field}>
          <div style={S.label}>Repo</div>
          <Input value={form.repo} onChange={setRepoValue} placeholder="owner/repo" />
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Blocked Paths</div>
            <textarea
              value={form.blocked_paths}
              onChange={(event) => setRuleField("blocked_paths", event.target.value)}
              style={{ ...S.input, minHeight: 90, resize: "vertical" }}
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Sensitive Paths</div>
            <textarea
              value={form.warn_paths}
              onChange={(event) => setRuleField("warn_paths", event.target.value)}
              style={{ ...S.input, minHeight: 90, resize: "vertical" }}
            />
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Require Tests For Paths</div>
            <textarea
              value={form.require_test_for_paths}
              onChange={(event) => setRuleField("require_test_for_paths", event.target.value)}
              style={{ ...S.input, minHeight: 90, resize: "vertical" }}
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Test Path Markers</div>
            <textarea
              value={form.test_paths}
              onChange={(event) => setRuleField("test_paths", event.target.value)}
              style={{ ...S.input, minHeight: 90, resize: "vertical" }}
            />
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Suspicious Terms</div>
            <textarea
              value={form.suspicious_terms}
              onChange={(event) => setRuleField("suspicious_terms", event.target.value)}
              style={{ ...S.input, minHeight: 90, resize: "vertical" }}
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Blocked Terms</div>
            <textarea
              value={form.blocked_terms}
              onChange={(event) => setRuleField("blocked_terms", event.target.value)}
              style={{ ...S.input, minHeight: 90, resize: "vertical" }}
            />
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Max Files</div>
            <Input value={form.max_files} onChange={(value) => setRuleField("max_files", value)} type="number" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Max Additions</div>
            <Input
              value={form.max_additions}
              onChange={(value) => setRuleField("max_additions", value)}
              type="number"
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Max Deletions</div>
            <Input
              value={form.max_deletions}
              onChange={(value) => setRuleField("max_deletions", value)}
              type="number"
            />
          </div>
        </div>

        <div style={S.field}>
          <div style={S.label}>Rule Notes</div>
          <textarea
            value={form.notes}
            onChange={(event) => setRuleField("notes", event.target.value)}
            style={{ ...S.input, minHeight: 90, resize: "vertical" }}
            placeholder="Why these rules exist, what the repo cares about, or which changes need a human in the loop."
          />
        </div>

        <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
          <Btn onClick={saveRules} disabled={busy}>
            {busy ? "Saving..." : "Save Rule Set"}
          </Btn>
          <Btn onClick={() => setForm({ ...DEFAULT_RULE_FORM, repo: form.repo })} color="var(--text-dim)">
            Reset To Defaults
          </Btn>
        </div>

        <div style={{ borderTop: "1px solid var(--border)", paddingTop: 18, display: "grid", gap: 14 }}>
          <div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>GitHub Report Templates</div>
            <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>
              These templates control how TrustGate speaks in GitHub check output and maintained PR
              comments for this repo. The hidden PatchHive comment marker is still managed automatically,
              so templates only control the human-facing content.
            </div>
          </div>

          {variables.length > 0 && (
            <div style={{ display: "grid", gap: 8 }}>
              <div style={{ fontSize: 13, fontWeight: 700 }}>Available Variables</div>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 8 }}>
                {variables.map((variable) => (
                  <div
                    key={variable.key}
                    style={{
                      border: "1px solid var(--border)",
                      borderRadius: 8,
                      padding: 10,
                      display: "grid",
                      gap: 6,
                    }}
                  >
                    <Tag color="var(--blue)">{`{{${variable.key}}}`}</Tag>
                    <div style={{ color: "var(--text-dim)", fontSize: 11, lineHeight: 1.5 }}>
                      {variable.description}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
            <div style={S.field}>
              <div style={S.label}>Check Title Template</div>
              <textarea
                value={templateForm.check_title_template}
                onChange={(event) => setTemplateField("check_title_template", event.target.value)}
                style={{ ...S.input, minHeight: 80, resize: "vertical" }}
              />
            </div>
            <div style={S.field}>
              <div style={S.label}>Check Summary Template</div>
              <textarea
                value={templateForm.check_summary_template}
                onChange={(event) => setTemplateField("check_summary_template", event.target.value)}
                style={{ ...S.input, minHeight: 120, resize: "vertical" }}
              />
            </div>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
            <div style={S.field}>
              <div style={S.label}>Check Text Template</div>
              <textarea
                value={templateForm.check_text_template}
                onChange={(event) => setTemplateField("check_text_template", event.target.value)}
                style={{ ...S.input, minHeight: 150, resize: "vertical" }}
              />
            </div>
            <div style={S.field}>
              <div style={S.label}>PR Comment Template</div>
              <textarea
                value={templateForm.comment_template}
                onChange={(event) => setTemplateField("comment_template", event.target.value)}
                style={{ ...S.input, minHeight: 260, resize: "vertical" }}
              />
            </div>
          </div>

          <div style={S.field}>
            <div style={S.label}>Template Notes</div>
            <textarea
              value={templateForm.notes}
              onChange={(event) => setTemplateField("notes", event.target.value)}
              style={{ ...S.input, minHeight: 80, resize: "vertical" }}
              placeholder="Tone guidance, escalation language, maintainer context, or repo-specific communication preferences."
            />
          </div>

          <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
            <Btn onClick={saveTemplates} disabled={busy}>
              {busy ? "Saving..." : "Save Template Set"}
            </Btn>
            <Btn
              onClick={() => setTemplateForm({ ...templateDefaults, repo: templateForm.repo || form.repo })}
              color="var(--text-dim)"
            >
              Reset Templates
            </Btn>
          </div>
        </div>
      </div>

      <div style={{ display: "grid", gap: 18, alignContent: "start" }}>
        <div style={{ ...S.panel, display: "grid", gap: 12 }}>
          <div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>Saved Rule Sets</div>
            <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
              These get picked up automatically when TrustGate reviews a diff for the same repo.
            </div>
          </div>

          {rules.length === 0 ? (
            <EmptyState icon="◌" text="Save a repo rule set and it will show up here." />
          ) : (
            rules.map((item) => (
              <div
                key={item.repo}
                style={{
                  border: "1px solid var(--border)",
                  borderRadius: 8,
                  padding: 12,
                  display: "grid",
                  gap: 10,
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                  <div style={{ fontWeight: 700 }}>{item.repo}</div>
                  <Tag color="var(--green)">saved</Tag>
                </div>
                <div style={{ color: "var(--text-dim)", fontSize: 11, lineHeight: 1.5 }}>
                  {item.rules.notes || "No notes yet. This repo will use the saved thresholds and path rules above."}
                </div>
                <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                  <Tag color="var(--accent)">{item.rules.blocked_paths.length} blocked paths</Tag>
                  <Tag color="var(--gold)">{item.rules.warn_paths.length} sensitive paths</Tag>
                  <Tag color="var(--blue)">{item.rules.suspicious_terms.length} suspicious terms</Tag>
                </div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  <Btn onClick={() => loadRuleSet(item)} color="var(--blue)">
                    Load
                  </Btn>
                  <Btn onClick={() => removeRules(item.repo)} color="var(--accent)" disabled={busy}>
                    Delete
                  </Btn>
                </div>
              </div>
            ))
          )}
        </div>

        <div style={{ ...S.panel, display: "grid", gap: 12 }}>
          <div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>Saved Template Sets</div>
            <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
              Repo teams can tune GitHub-facing TrustGate language without changing the review engine.
            </div>
          </div>

          {templates.length === 0 ? (
            <EmptyState icon="✎" text="Save a repo template set and it will show up here." />
          ) : (
            templates.map((item) => (
              <div
                key={item.repo}
                style={{
                  border: "1px solid var(--border)",
                  borderRadius: 8,
                  padding: 12,
                  display: "grid",
                  gap: 10,
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                  <div style={{ fontWeight: 700 }}>{item.repo}</div>
                  <Tag color="var(--blue)">custom voice</Tag>
                </div>
                <div style={{ color: "var(--text-dim)", fontSize: 11, lineHeight: 1.5 }}>
                  {item.templates.notes ||
                    "No notes yet. TrustGate will use this repo's custom GitHub check and PR comment wording."}
                </div>
                <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                  <Tag color="var(--blue)">check title</Tag>
                  <Tag color="var(--blue)">check summary</Tag>
                  <Tag color="var(--blue)">PR comment</Tag>
                </div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  <Btn onClick={() => loadTemplateSet(item)} color="var(--blue)">
                    Load
                  </Btn>
                  <Btn onClick={() => removeTemplates(item.repo)} color="var(--accent)" disabled={busy}>
                    Delete
                  </Btn>
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
