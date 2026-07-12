import { useEffect, useState } from "react";
import { BookOpenCheck, FileText, Save, ShieldCheck, Trash2 } from "lucide-react";
import { V3_TEXT } from "@patchhivehq/ui-v3";

const DEFAULT_RULES = {
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

const DEFAULT_TEMPLATES = {
  repo: "",
  check_title_template: "TrustGate: {{recommendation_upper}}",
  check_summary_template: "{{emoji}} TrustGate recommends **{{recommendation_upper}}** for this PR.\n\n{{summary}}",
  check_text_template: "{{findings_plaintext}}",
  comment_template: "## {{emoji}} TrustGate: {{recommendation_upper}}\n\n{{summary}}\n\n### Risk snapshot\n- Risk score: **{{risk_score}}**\n- Files changed: **{{files_changed}}**\n- Additions / deletions: **+{{additions}} / -{{deletions}}**\n- Tests changed: **{{tests_changed}}**\n- Blocking findings: **{{blocked_findings}}**\n- Warning findings: **{{warning_findings}}**\n\n### Findings\n{{findings_markdown}}\n\n### File hotspots\n{{file_hotspots_markdown}}\n\n### Next move\n{{next_move}}\n\n{{details_markdown}}",
  notes: "",
};

function splitList(value) {
  return String(value || "").split(",").map((item) => item.trim()).filter(Boolean);
}

function fromRules(rules = {}) {
  return {
    ...DEFAULT_RULES,
    ...rules,
    blocked_paths: (rules.blocked_paths || splitList(DEFAULT_RULES.blocked_paths)).join(", "),
    warn_paths: (rules.warn_paths || splitList(DEFAULT_RULES.warn_paths)).join(", "),
    require_test_for_paths: (rules.require_test_for_paths || splitList(DEFAULT_RULES.require_test_for_paths)).join(", "),
    test_paths: (rules.test_paths || splitList(DEFAULT_RULES.test_paths)).join(", "),
    suspicious_terms: (rules.suspicious_terms || splitList(DEFAULT_RULES.suspicious_terms)).join(", "),
    blocked_terms: (rules.blocked_terms || splitList(DEFAULT_RULES.blocked_terms)).join(", "),
    max_files: String(rules.max_files ?? DEFAULT_RULES.max_files),
    max_additions: String(rules.max_additions ?? DEFAULT_RULES.max_additions),
    max_deletions: String(rules.max_deletions ?? DEFAULT_RULES.max_deletions),
  };
}

function rulePayload(form) {
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

function Field({ hint, label, onChange, rows, type = "text", value }) {
  return <label className="block"><span className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>{label}</span>{hint ? <span className={`mt-1 block text-[10px] leading-relaxed ${V3_TEXT.mute}`}>{hint}</span> : null}<div className="surface-inset mt-2 rounded-xl px-3 py-2">{rows ? <textarea className={`w-full resize-y bg-transparent font-mono text-[11px] leading-relaxed outline-none ${V3_TEXT.strong}`} onChange={(event) => onChange(event.target.value)} rows={rows} value={value} /> : <input className={`h-8 w-full bg-transparent text-[12px] outline-none ${V3_TEXT.strong}`} onChange={(event) => onChange(event.target.value)} type={type} value={value} />}</div></label>;
}

function ActionButton({ children, danger = false, disabled, onClick }) {
  return <button className={`surface-inset inline-flex h-9 items-center gap-2 rounded-full px-4 text-[11px] font-medium disabled:opacity-50 ${danger ? "text-red-700 dark:text-red-300" : V3_TEXT.body}`} disabled={disabled} onClick={onClick} type="button">{children}</button>;
}

function Badge({ children }) {
  return <span className={`surface-inset inline-flex min-h-7 items-center rounded-full px-2.5 text-[9px] uppercase tracking-wider ${V3_TEXT.body}`}>{children}</span>;
}

async function readJson(response, fallback) {
  const data = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(data.error || data.message || fallback);
  return data;
}

export default function PolicyPanel({ apiBase, fetcher, form: runForm, onError, onRefresh }) {
  const [rules, setRules] = useState([]);
  const [templates, setTemplates] = useState([]);
  const [packs, setPacks] = useState([]);
  const [variables, setVariables] = useState([]);
  const [ruleForm, setRuleForm] = useState(() => ({ ...DEFAULT_RULES, repo: runForm.repo || "" }));
  const [templateForm, setTemplateForm] = useState(() => ({ ...DEFAULT_TEMPLATES, repo: runForm.repo || "" }));
  const [templateDefaults, setTemplateDefaults] = useState(DEFAULT_TEMPLATES);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);

  async function refresh() {
    setLoading(true);
    try {
      const [ruleData, packData, templateData] = await Promise.all([
        readJson(await fetcher(`${apiBase}/rules`), "Could not load rule sets."),
        readJson(await fetcher(`${apiBase}/rule-packs`), "Could not load rule packs."),
        readJson(await fetcher(`${apiBase}/templates`), "Could not load report templates."),
      ]);
      setRules(ruleData.rules || []);
      setPacks(packData.packs || []);
      setTemplates(templateData.templates || []);
      setVariables(templateData.variables || []);
      const defaults = { ...DEFAULT_TEMPLATES, ...(templateData.defaults || {}) };
      setTemplateDefaults(defaults);
      setTemplateForm((current) => current.check_title_template ? current : { ...defaults, repo: current.repo });
    } catch (error) {
      onError(error.message || "TrustGate could not load policy data.");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => { refresh(); }, [fetcher]);
  useEffect(() => {
    if (!runForm.repo) return;
    setRuleForm((current) => current.repo ? current : { ...current, repo: runForm.repo });
    setTemplateForm((current) => current.repo ? current : { ...current, repo: runForm.repo });
  }, [runForm.repo]);

  function setRepo(repo) {
    setRuleForm((current) => ({ ...current, repo }));
    setTemplateForm((current) => ({ ...current, repo }));
  }

  function loadRules(item) {
    setRuleForm(fromRules(item.rules));
    const matched = templates.find((template) => template.repo === item.repo);
    setTemplateForm(matched ? { ...templateDefaults, ...matched.templates } : { ...templateDefaults, repo: item.repo });
  }

  function loadTemplates(item) {
    setTemplateForm({ ...templateDefaults, ...item.templates });
    const matched = rules.find((rule) => rule.repo === item.repo);
    setRuleForm(matched ? fromRules(matched.rules) : { ...DEFAULT_RULES, repo: item.repo });
  }

  async function save(path, payload, successRepo) {
    if (!successRepo.trim()) {
      onError("TrustGate needs a repository in owner/name format before saving policy.");
      return;
    }
    setBusy(true);
    try {
      const data = await readJson(await fetcher(`${apiBase}/${path}`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload) }), `Could not save ${path}.`);
      setRepo(data.repo || successRepo.trim());
      await refresh();
      onRefresh();
    } catch (error) {
      onError(error.message || `TrustGate could not save ${path}.`);
    } finally {
      setBusy(false);
    }
  }

  async function remove(path, repo) {
    if (!window.confirm(`Delete the saved ${path} for ${repo}?`)) return;
    setBusy(true);
    try {
      await readJson(await fetcher(`${apiBase}/${path}/${encodeURIComponent(repo)}`, { method: "DELETE" }), `Could not delete ${path}.`);
      await refresh();
      onRefresh();
    } catch (error) {
      onError(error.message || `TrustGate could not delete ${path}.`);
    } finally {
      setBusy(false);
    }
  }

  return <div className="space-y-6">
    <section className="surface p-6 sm:p-8"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><ShieldCheck size={12} /> Policy memory</div><h1 className={`mt-2 font-display text-[42px] font-semibold ${V3_TEXT.strong}`}>Repo rules and report voice.</h1><p className={`mt-3 max-w-3xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>Configure what TrustGate blocks, what it escalates, how large an automated patch may grow, and how its maintained GitHub artifacts speak to maintainers.</p></section>

    <section className="surface p-5 sm:p-6"><div className="flex items-center justify-between gap-4"><div><div className={`text-[10px] uppercase tracking-[0.2em] ${V3_TEXT.mute}`}>Starter rule packs</div><div className={`mt-1 font-display text-[24px] ${V3_TEXT.strong}`}>{packs.length} policy templates</div></div>{loading ? <span className={`text-[11px] ${V3_TEXT.mute}`}>Loading…</span> : null}</div><div className="mt-5 grid gap-3 md:grid-cols-2 xl:grid-cols-4">{packs.map((pack) => <article className="surface-inset rounded-xl p-4" key={pack.id}><div className="flex items-center justify-between"><div className={`font-display text-[17px] ${V3_TEXT.strong}`}>{pack.label}</div><Badge>{pack.id}</Badge></div><p className={`mt-2 min-h-12 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>{pack.description}</p><ActionButton onClick={() => setRuleForm({ ...fromRules(pack.rules), repo: ruleForm.repo })}>Apply pack</ActionButton></article>)}</div></section>

    <div className="grid grid-cols-12 gap-6"><section className="surface col-span-12 xl:col-span-8 p-5 sm:p-6"><div className="flex items-center justify-between gap-4"><div><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.2em] ${V3_TEXT.mute}`}><BookOpenCheck size={12} /> Repo rule set</div><h2 className={`mt-1 font-display text-[28px] ${V3_TEXT.strong}`}>Review boundaries.</h2></div><ActionButton disabled={busy} onClick={() => save("rules", rulePayload(ruleForm), ruleForm.repo)}><Save size={13} /> Save rules</ActionButton></div><div className="mt-6 grid gap-4 sm:grid-cols-2"><div className="sm:col-span-2"><Field label="Repository" onChange={setRepo} value={ruleForm.repo} /></div><Field label="Blocked paths" onChange={(value) => setRuleForm((current) => ({ ...current, blocked_paths: value }))} rows={4} value={ruleForm.blocked_paths} /><Field label="Sensitive paths" onChange={(value) => setRuleForm((current) => ({ ...current, warn_paths: value }))} rows={4} value={ruleForm.warn_paths} /><Field label="Require tests for paths" onChange={(value) => setRuleForm((current) => ({ ...current, require_test_for_paths: value }))} rows={4} value={ruleForm.require_test_for_paths} /><Field label="Test path markers" onChange={(value) => setRuleForm((current) => ({ ...current, test_paths: value }))} rows={4} value={ruleForm.test_paths} /><Field hint="TODO and FIXME require standalone uppercase markers. token, secret, and password warn only when assigned hardcoded values." label="Suspicious terms" onChange={(value) => setRuleForm((current) => ({ ...current, suspicious_terms: value }))} rows={4} value={ruleForm.suspicious_terms} /><Field hint="Known credential prefixes require a token-shaped value, so ordinary words such as task-panel do not match sk-." label="Blocked terms" onChange={(value) => setRuleForm((current) => ({ ...current, blocked_terms: value }))} rows={4} value={ruleForm.blocked_terms} /></div><div className="mt-4 grid grid-cols-3 gap-3"><Field label="Max files" onChange={(value) => setRuleForm((current) => ({ ...current, max_files: value }))} type="number" value={ruleForm.max_files} /><Field label="Max additions" onChange={(value) => setRuleForm((current) => ({ ...current, max_additions: value }))} type="number" value={ruleForm.max_additions} /><Field label="Max deletions" onChange={(value) => setRuleForm((current) => ({ ...current, max_deletions: value }))} type="number" value={ruleForm.max_deletions} /></div><div className="mt-4"><Field label="Rule notes" onChange={(value) => setRuleForm((current) => ({ ...current, notes: value }))} rows={4} value={ruleForm.notes} /></div></section>
      <aside className="surface col-span-12 xl:col-span-4 p-5 sm:p-6"><div className={`text-[10px] uppercase tracking-[0.2em] ${V3_TEXT.mute}`}>Saved rule sets</div><div className="mt-4 space-y-3">{rules.length ? rules.map((item) => <article className="surface-inset rounded-xl p-4" key={item.repo}><div className="flex items-center justify-between gap-3"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>{item.repo}</div><Badge>saved</Badge></div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>{item.rules.notes || `${item.rules.blocked_paths.length} blocked paths · ${item.rules.warn_paths.length} sensitive paths`}</p><div className="mt-3 flex flex-wrap gap-2"><ActionButton onClick={() => loadRules(item)}>Load</ActionButton><ActionButton danger disabled={busy} onClick={() => remove("rules", item.repo)}><Trash2 size={12} /> Delete</ActionButton></div></article>) : <div className={`py-12 text-center text-[12px] leading-relaxed ${V3_TEXT.mute}`}>No repo-specific rule sets saved.<br />The default policy remains active.</div>}</div></aside></div>

    <div className="grid grid-cols-12 gap-6"><section className="surface col-span-12 xl:col-span-8 p-5 sm:p-6"><div className="flex items-center justify-between gap-4"><div><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.2em] ${V3_TEXT.mute}`}><FileText size={12} /> GitHub report templates</div><h2 className={`mt-1 font-display text-[28px] ${V3_TEXT.strong}`}>Maintainer-facing output.</h2></div><ActionButton disabled={busy} onClick={() => save("templates", { ...templateForm, repo: templateForm.repo.trim(), notes: templateForm.notes.trim() }, templateForm.repo)}><Save size={13} /> Save templates</ActionButton></div><div className="mt-6 grid gap-4 sm:grid-cols-2"><div className="sm:col-span-2"><Field label="Repository" onChange={setRepo} value={templateForm.repo} /></div><Field label="Check title template" onChange={(value) => setTemplateForm((current) => ({ ...current, check_title_template: value }))} rows={3} value={templateForm.check_title_template} /><Field label="Check summary template" onChange={(value) => setTemplateForm((current) => ({ ...current, check_summary_template: value }))} rows={5} value={templateForm.check_summary_template} /><Field label="Check text template" onChange={(value) => setTemplateForm((current) => ({ ...current, check_text_template: value }))} rows={7} value={templateForm.check_text_template} /><Field label="PR comment template" onChange={(value) => setTemplateForm((current) => ({ ...current, comment_template: value }))} rows={12} value={templateForm.comment_template} /><div className="sm:col-span-2"><Field label="Template notes" onChange={(value) => setTemplateForm((current) => ({ ...current, notes: value }))} rows={4} value={templateForm.notes} /></div></div><details className="surface-inset mt-5 rounded-xl p-4"><summary className={`cursor-pointer text-[12px] font-semibold ${V3_TEXT.strong}`}>Available template variables · {variables.length}</summary><div className="mt-4 grid gap-2 sm:grid-cols-2">{variables.map((variable) => <div className="rounded-lg border p-3" style={{ borderColor: "var(--surface-border-2)" }} key={variable.key}><code className={`text-[10px] ${V3_TEXT.strong}`}>{`{{${variable.key}}}`}</code><p className={`mt-1 text-[10px] leading-relaxed ${V3_TEXT.mute}`}>{variable.description}</p></div>)}</div></details></section>
      <aside className="surface col-span-12 xl:col-span-4 p-5 sm:p-6"><div className={`text-[10px] uppercase tracking-[0.2em] ${V3_TEXT.mute}`}>Saved template sets</div><div className="mt-4 space-y-3">{templates.length ? templates.map((item) => <article className="surface-inset rounded-xl p-4" key={item.repo}><div className="flex items-center justify-between gap-3"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>{item.repo}</div><Badge>custom voice</Badge></div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>{item.templates.notes || "Custom check and maintained-comment language."}</p><div className="mt-3 flex flex-wrap gap-2"><ActionButton onClick={() => loadTemplates(item)}>Load</ActionButton><ActionButton danger disabled={busy} onClick={() => remove("templates", item.repo)}><Trash2 size={12} /> Delete</ActionButton></div></article>) : <div className={`py-12 text-center text-[12px] leading-relaxed ${V3_TEXT.mute}`}>No repo-specific template sets saved.<br />The default report voice remains active.</div>}</div></aside></div>
  </div>;
}
