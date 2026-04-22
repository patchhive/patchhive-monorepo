import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, Input, S, Tag } from "@patchhivehq/ui";

const DEFAULT_LESSON_FORM = {
  repo: "",
  title: "",
  outcome: "",
  lesson: "",
  prevention: "",
  affected_paths: "",
  evidence: "",
  disposition: "policy",
  pinned: true,
};

const DEFAULT_CANDIDATE_FORM = {
  repo: "",
  source_type: "operator",
  source_ref: "",
  title: "",
  outcome: "",
  lesson: "",
  prevention: "",
  affected_paths: "",
  evidence: "",
};

const SOURCE_OPTIONS = [
  ["operator", "Operator"],
  ["trustgate-block", "TrustGate block"],
  ["trustgate-warn", "TrustGate warn"],
  ["repo-reaper-rejection", "RepoReaper rejection"],
  ["reviewbee-thread", "ReviewBee thread"],
  ["reverted-pr", "Reverted PR"],
];

function lines(text) {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
}

function joined(items) {
  return (items || []).join("\n");
}

function confidenceColor(confidence) {
  if (confidence >= 84) return "var(--green)";
  if (confidence >= 72) return "var(--gold)";
  return "var(--blue)";
}

function candidateReviewForm(candidate) {
  return {
    title: candidate.title || "",
    outcome: candidate.outcome || "",
    lesson: candidate.lesson || "",
    prevention: candidate.prevention || "",
    affected_paths: joined(candidate.affected_paths),
    evidence: joined(candidate.evidence),
    disposition: "policy",
    pinned: true,
  };
}

export default function FailGuardPanel({
  apiKey,
  activeRepo,
  setActiveRepo,
  running,
  setRunning,
  setError,
  onCaptured,
  onShowMemory,
}) {
  const [form, setForm] = useState(() => ({ ...DEFAULT_LESSON_FORM, repo: activeRepo || "" }));
  const [candidateForm, setCandidateForm] = useState(() => ({ ...DEFAULT_CANDIDATE_FORM, repo: activeRepo || "" }));
  const [candidateStatus, setCandidateStatus] = useState("open");
  const [candidates, setCandidates] = useState([]);
  const [candidatesLoading, setCandidatesLoading] = useState(false);
  const [selectedCandidate, setSelectedCandidate] = useState(null);
  const [reviewForm, setReviewForm] = useState(null);
  const [result, setResult] = useState(null);
  const fetch_ = createApiFetcher(apiKey);

  useEffect(() => {
    if (activeRepo && !form.repo) {
      setForm((current) => ({ ...current, repo: activeRepo }));
    }
    if (activeRepo && !candidateForm.repo) {
      setCandidateForm((current) => ({ ...current, repo: activeRepo }));
    }
  }, [activeRepo, form.repo, candidateForm.repo]);

  useEffect(() => {
    loadCandidates(activeRepo, candidateStatus);
  }, [apiKey, activeRepo, candidateStatus]);

  function update(key, value) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  function updateCandidate(key, value) {
    setCandidateForm((current) => ({ ...current, [key]: value }));
  }

  function updateReview(key, value) {
    setReviewForm((current) => ({ ...current, [key]: value }));
  }

  async function loadCandidates(repo = activeRepo, status = candidateStatus) {
    setCandidatesLoading(true);
    try {
      const params = new URLSearchParams();
      params.set("status", status);
      if (repo) params.set("repo", repo);
      const res = await fetch_(`${API}/failguard/candidates?${params.toString()}`);
      const data = await res.json();
      setCandidates(data.candidates || []);
    } catch {
      setCandidates([]);
    } finally {
      setCandidatesLoading(false);
    }
  }

  async function captureLesson() {
    setRunning(true);
    setError("");
    setResult(null);
    try {
      const res = await fetch_(`${API}/failguard/lessons`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          repo: form.repo.trim(),
          title: form.title.trim(),
          outcome: form.outcome.trim(),
          lesson: form.lesson.trim(),
          prevention: form.prevention.trim(),
          affected_paths: lines(form.affected_paths),
          evidence: lines(form.evidence),
          disposition: form.disposition,
          pinned: form.pinned,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "RepoMemory could not capture that FailGuard lesson.");
      }
      setResult(data);
      setActiveRepo(data.run.repo);
      onCaptured?.(data.run);
      await loadCandidates(data.run.repo, candidateStatus);
    } catch (err) {
      setError(err.message || "RepoMemory could not capture that FailGuard lesson.");
    } finally {
      setRunning(false);
    }
  }

  async function suggestCandidate() {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/failguard/candidates`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          repo: candidateForm.repo.trim(),
          source_type: candidateForm.source_type,
          source_ref: candidateForm.source_ref.trim(),
          title: candidateForm.title.trim(),
          outcome: candidateForm.outcome.trim(),
          lesson: candidateForm.lesson.trim(),
          prevention: candidateForm.prevention.trim(),
          affected_paths: lines(candidateForm.affected_paths),
          evidence: lines(candidateForm.evidence),
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "FailGuard could not queue that candidate.");
      }
      setActiveRepo(data.candidate.repo);
      setCandidateForm((current) => ({
        ...DEFAULT_CANDIDATE_FORM,
        repo: data.candidate.repo || current.repo,
        source_type: current.source_type,
      }));
      setSelectedCandidate(data.candidate);
      setReviewForm(candidateReviewForm(data.candidate));
      await loadCandidates(data.candidate.repo, candidateStatus);
    } catch (err) {
      setError(err.message || "FailGuard could not queue that candidate.");
    } finally {
      setRunning(false);
    }
  }

  function selectCandidate(candidate) {
    setSelectedCandidate(candidate);
    setReviewForm(candidateReviewForm(candidate));
  }

  async function promoteCandidate() {
    if (!selectedCandidate || !reviewForm) return;
    setRunning(true);
    setError("");
    setResult(null);
    try {
      const res = await fetch_(`${API}/failguard/candidates/${selectedCandidate.id}/promote`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          title: reviewForm.title.trim(),
          outcome: reviewForm.outcome.trim(),
          lesson: reviewForm.lesson.trim(),
          prevention: reviewForm.prevention.trim(),
          affected_paths: lines(reviewForm.affected_paths),
          evidence: lines(reviewForm.evidence),
          disposition: reviewForm.disposition,
          pinned: reviewForm.pinned,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "FailGuard could not promote that candidate.");
      }
      setResult(data);
      setActiveRepo(data.run.repo);
      onCaptured?.(data.run);
      setSelectedCandidate(null);
      setReviewForm(null);
      await loadCandidates(data.run.repo, candidateStatus);
    } catch (err) {
      setError(err.message || "FailGuard could not promote that candidate.");
    } finally {
      setRunning(false);
    }
  }

  async function dismissCandidate(candidate = selectedCandidate) {
    if (!candidate) return;
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/failguard/candidates/${candidate.id}/dismiss`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ reason: "Dismissed during FailGuard review." }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "FailGuard could not dismiss that candidate.");
      }
      if (selectedCandidate?.id === candidate.id) {
        setSelectedCandidate(null);
        setReviewForm(null);
      }
      await loadCandidates(candidate.repo, candidateStatus);
    } catch (err) {
      setError(err.message || "FailGuard could not dismiss that candidate.");
    } finally {
      setRunning(false);
    }
  }

  const lessonRequired =
    form.repo.trim() &&
    form.title.trim() &&
    form.outcome.trim() &&
    form.lesson.trim() &&
    form.prevention.trim();
  const candidateRequired =
    candidateForm.repo.trim() &&
    candidateForm.title.trim() &&
    candidateForm.outcome.trim();
  const reviewRequired =
    reviewForm?.title.trim() &&
    reviewForm?.outcome.trim() &&
    reviewForm?.lesson.trim() &&
    reviewForm?.prevention.trim();

  return (
    <div style={{ display: "grid", gap: 18 }}>
      <div style={{ ...S.panel, display: "grid", gap: 14 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start", flexWrap: "wrap" }}>
          <div style={{ display: "grid", gap: 6 }}>
            <div style={{ fontSize: 18, fontWeight: 800 }}>Suggested FailGuard lessons</div>
            <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>
              Review bad outcomes before they become RepoMemory policy.
            </div>
          </div>
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
            <Tag color="var(--accent)">candidate queue</Tag>
            <Tag color="var(--green)">operator promoted</Tag>
            <Tag color="var(--blue)">policy memory</Tag>
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Repo</div>
            <Input value={candidateForm.repo} onChange={(value) => updateCandidate("repo", value)} placeholder="owner/repo" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Source</div>
            <select
              value={candidateForm.source_type}
              onChange={(event) => updateCandidate("source_type", event.target.value)}
              style={S.select}
            >
              {SOURCE_OPTIONS.map(([value, label]) => (
                <option key={value} value={value}>{label}</option>
              ))}
            </select>
          </div>
          <div style={S.field}>
            <div style={S.label}>Source Ref</div>
            <Input value={candidateForm.source_ref} onChange={(value) => updateCandidate("source_ref", value)} placeholder="run id, PR URL, review id" />
          </div>
        </div>

        <div style={S.field}>
          <div style={S.label}>Candidate Title</div>
          <Input
            value={candidateForm.title}
            onChange={(value) => updateCandidate("title", value)}
            placeholder="Generated patch skipped webhook signing"
          />
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Bad Outcome</div>
            <textarea
              value={candidateForm.outcome}
              onChange={(event) => updateCandidate("outcome", event.target.value)}
              style={{ ...S.input, minHeight: 96, resize: "vertical" }}
              placeholder="Smith rejected a generated patch because webhook verification failed open."
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Evidence</div>
            <textarea
              value={candidateForm.evidence}
              onChange={(event) => updateCandidate("evidence", event.target.value)}
              style={{ ...S.input, minHeight: 96, resize: "vertical" }}
              placeholder={"Smith rejection run-7\nhttps://github.com/owner/repo/pull/123"}
            />
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Draft Lesson</div>
            <textarea
              value={candidateForm.lesson}
              onChange={(event) => updateCandidate("lesson", event.target.value)}
              style={{ ...S.input, minHeight: 84, resize: "vertical" }}
              placeholder="Leave blank for a deterministic draft."
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Draft Prevention</div>
            <textarea
              value={candidateForm.prevention}
              onChange={(event) => updateCandidate("prevention", event.target.value)}
              style={{ ...S.input, minHeight: 84, resize: "vertical" }}
              placeholder="Leave blank for a deterministic draft."
            />
          </div>
        </div>

        <div style={S.field}>
          <div style={S.label}>Affected Paths</div>
          <textarea
            value={candidateForm.affected_paths}
            onChange={(event) => updateCandidate("affected_paths", event.target.value)}
            style={{ ...S.input, minHeight: 72, resize: "vertical" }}
            placeholder={"backend/src/routes/webhook.rs\ncrates/patchhive-product-core/src/auth.rs"}
          />
        </div>

        <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
          <Btn onClick={suggestCandidate} disabled={running || !candidateRequired}>
            {running ? "Queuing..." : "Queue candidate"}
          </Btn>
          <select
            value={candidateStatus}
            onChange={(event) => setCandidateStatus(event.target.value)}
            style={{ ...S.select, width: 150 }}
          >
            <option value="open">Open</option>
            <option value="promoted">Promoted</option>
            <option value="dismissed">Dismissed</option>
            <option value="all">All</option>
          </select>
          <Btn onClick={() => loadCandidates(activeRepo, candidateStatus)} disabled={candidatesLoading} style={{ padding: "4px 10px" }}>
            {candidatesLoading ? "Refreshing..." : "Refresh"}
          </Btn>
        </div>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: 16 }}>
        <div style={{ ...S.panel, display: "grid", gap: 12, alignContent: "start" }}>
          <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center" }}>
            <div style={{ fontSize: 16, fontWeight: 800 }}>Candidate queue</div>
            <Tag color="var(--blue)">{candidates.length} {candidateStatus}</Tag>
          </div>
          {candidates.length ? (
            <div style={{ display: "grid", gap: 10 }}>
              {candidates.map((candidate) => (
                <div
                  key={candidate.id}
                  style={{
                    display: "grid",
                    gap: 8,
                    border: "1px solid var(--border)",
                    borderRadius: 8,
                    padding: 10,
                    background: selectedCandidate?.id === candidate.id ? "var(--bg-input)" : "rgba(255,255,255,0.02)",
                  }}
                >
                  <div style={{ display: "flex", justifyContent: "space-between", gap: 8, flexWrap: "wrap" }}>
                    <strong>{candidate.title}</strong>
                    <Tag color={confidenceColor(candidate.confidence)}>{Math.round(candidate.confidence)}%</Tag>
                  </div>
                  <div style={{ color: "var(--text-dim)", lineHeight: 1.5 }}>{candidate.outcome}</div>
                  <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                    <Tag color="var(--accent)">{candidate.source_type}</Tag>
                    <Tag color={candidate.status === "open" ? "var(--gold)" : candidate.status === "promoted" ? "var(--green)" : "var(--text-dim)"}>
                      {candidate.status}
                    </Tag>
                    {candidate.memory_ref && <Tag color="var(--green)">memory linked</Tag>}
                  </div>
                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                    {candidate.status === "open" && (
                      <>
                        <Btn onClick={() => selectCandidate(candidate)} style={{ padding: "4px 10px" }}>Review</Btn>
                        <Btn onClick={() => dismissCandidate(candidate)} style={{ padding: "4px 10px" }}>Dismiss</Btn>
                      </>
                    )}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <EmptyState icon="FG" text="No FailGuard candidates in this queue." />
          )}
        </div>

        <div style={{ ...S.panel, display: "grid", gap: 12, alignContent: "start" }}>
          <div style={{ fontSize: 16, fontWeight: 800 }}>Review candidate</div>
          {selectedCandidate && reviewForm ? (
            <>
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                <Tag color="var(--accent)">{selectedCandidate.source_type}</Tag>
                {selectedCandidate.source_ref && <Tag color="var(--blue)">{selectedCandidate.source_ref}</Tag>}
              </div>
              <div style={S.field}>
                <div style={S.label}>Title</div>
                <Input value={reviewForm.title} onChange={(value) => updateReview("title", value)} />
              </div>
              <div style={S.field}>
                <div style={S.label}>Bad Outcome</div>
                <textarea
                  value={reviewForm.outcome}
                  onChange={(event) => updateReview("outcome", event.target.value)}
                  style={{ ...S.input, minHeight: 82, resize: "vertical" }}
                />
              </div>
              <div style={S.field}>
                <div style={S.label}>Durable Lesson</div>
                <textarea
                  value={reviewForm.lesson}
                  onChange={(event) => updateReview("lesson", event.target.value)}
                  style={{ ...S.input, minHeight: 82, resize: "vertical" }}
                />
              </div>
              <div style={S.field}>
                <div style={S.label}>Future Prevention</div>
                <textarea
                  value={reviewForm.prevention}
                  onChange={(event) => updateReview("prevention", event.target.value)}
                  style={{ ...S.input, minHeight: 82, resize: "vertical" }}
                />
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 12 }}>
                <div style={S.field}>
                  <div style={S.label}>Affected Paths</div>
                  <textarea
                    value={reviewForm.affected_paths}
                    onChange={(event) => updateReview("affected_paths", event.target.value)}
                    style={{ ...S.input, minHeight: 72, resize: "vertical" }}
                  />
                </div>
                <div style={S.field}>
                  <div style={S.label}>Evidence</div>
                  <textarea
                    value={reviewForm.evidence}
                    onChange={(event) => updateReview("evidence", event.target.value)}
                    style={{ ...S.input, minHeight: 72, resize: "vertical" }}
                  />
                </div>
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
                <div style={S.field}>
                  <div style={S.label}>Disposition</div>
                  <select
                    value={reviewForm.disposition}
                    onChange={(event) => updateReview("disposition", event.target.value)}
                    style={S.select}
                  >
                    <option value="policy">Policy</option>
                    <option value="signal">Signal</option>
                  </select>
                </div>
                <label style={{ display: "flex", alignItems: "end", gap: 8, color: "var(--text-dim)", paddingBottom: 9 }}>
                  <input
                    type="checkbox"
                    checked={reviewForm.pinned}
                    onChange={(event) => updateReview("pinned", event.target.checked)}
                  />
                  pin promoted lesson
                </label>
              </div>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                <Btn onClick={promoteCandidate} disabled={running || !reviewRequired} color="var(--green)">
                  {running ? "Promoting..." : "Promote to memory"}
                </Btn>
                <Btn onClick={() => dismissCandidate(selectedCandidate)} disabled={running}>
                  Dismiss
                </Btn>
              </div>
            </>
          ) : (
            <EmptyState icon="FG" text="Select an open candidate to review." />
          )}
        </div>
      </div>

      <div style={{ ...S.panel, display: "grid", gap: 14 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start", flexWrap: "wrap" }}>
          <div style={{ display: "grid", gap: 6 }}>
            <div style={{ fontSize: 18, fontWeight: 800 }}>Capture a FailGuard lesson</div>
            <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>
              Store an approved lesson directly as RepoMemory policy.
            </div>
          </div>
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
            <Tag color="var(--accent)">failure pattern</Tag>
            <Tag color="var(--green)">RepoMemory policy</Tag>
            <Tag color="var(--blue)">TrustGate context</Tag>
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Repo</div>
            <Input value={form.repo} onChange={(value) => update("repo", value)} placeholder="owner/repo" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Disposition</div>
            <select
              value={form.disposition}
              onChange={(event) => update("disposition", event.target.value)}
              style={S.select}
            >
              <option value="policy">Policy</option>
              <option value="signal">Signal</option>
            </select>
          </div>
          <label style={{ display: "flex", alignItems: "end", gap: 8, color: "var(--text-dim)", paddingBottom: 9 }}>
            <input
              type="checkbox"
              checked={form.pinned}
              onChange={(event) => update("pinned", event.target.checked)}
            />
            pin this lesson
          </label>
        </div>

        <div style={S.field}>
          <div style={S.label}>Title</div>
          <Input
            value={form.title}
            onChange={(value) => update("title", value)}
            placeholder="Webhook endpoints must fail closed without a secret"
          />
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Bad Outcome</div>
            <textarea
              value={form.outcome}
              onChange={(event) => update("outcome", event.target.value)}
              style={{ ...S.input, minHeight: 106, resize: "vertical" }}
              placeholder="Unsigned webhook payloads could trigger autonomous work."
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Future Prevention</div>
            <textarea
              value={form.prevention}
              onChange={(event) => update("prevention", event.target.value)}
              style={{ ...S.input, minHeight: 106, resize: "vertical" }}
              placeholder="Reject webhook delivery when the signing secret is missing."
            />
          </div>
        </div>

        <div style={S.field}>
          <div style={S.label}>Durable Lesson</div>
          <textarea
            value={form.lesson}
            onChange={(event) => update("lesson", event.target.value)}
            style={{ ...S.input, minHeight: 90, resize: "vertical" }}
            placeholder="Public webhook routes must not accept unsigned payloads or silently skip signature verification."
          />
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Affected Paths</div>
            <textarea
              value={form.affected_paths}
              onChange={(event) => update("affected_paths", event.target.value)}
              style={{ ...S.input, minHeight: 96, resize: "vertical" }}
              placeholder={"backend/src/routes/webhook.rs\ncrates/patchhive-product-core/src/auth.rs"}
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>Evidence</div>
            <textarea
              value={form.evidence}
              onChange={(event) => update("evidence", event.target.value)}
              style={{ ...S.input, minHeight: 96, resize: "vertical" }}
              placeholder={"Hermes review C2\nhttps://github.com/owner/repo/pull/123"}
            />
          </div>
        </div>

        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
          <Btn onClick={captureLesson} disabled={running || !lessonRequired} color="var(--green)">
            {running ? "Capturing..." : "Capture lesson"}
          </Btn>
          {result?.entry && (
            <Btn onClick={onShowMemory} style={{ padding: "4px 10px" }}>
              Open memory
            </Btn>
          )}
        </div>
      </div>

      {result?.entry ? (
        <div style={{ ...S.panel, display: "grid", gap: 10, borderColor: "var(--green)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
            <div>
              <div style={{ fontSize: 16, fontWeight: 800 }}>{result.entry.title}</div>
              <div style={{ color: "var(--text-dim)", fontSize: 11 }}>{result.entry.memory_ref}</div>
            </div>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              <Tag color="var(--accent)">{result.entry.kind}</Tag>
              <Tag color="var(--green)">{result.entry.disposition}</Tag>
              {result.entry.pinned && <Tag color="var(--blue)">pinned</Tag>}
            </div>
          </div>
          <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>{result.entry.detail}</div>
          <div style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 10, background: "var(--bg-input)" }}>
            <div style={S.label}>Prompt line</div>
            <div style={{ marginTop: 6, lineHeight: 1.6 }}>{result.entry.prompt_line}</div>
          </div>
        </div>
      ) : (
        <EmptyState icon="FG" text="Promoted lessons will appear as policy-ready RepoMemory failure patterns." />
      )}
    </div>
  );
}
