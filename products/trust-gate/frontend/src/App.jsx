import { useCallback, useEffect, useState } from "react";
import {
  applyTheme,
  Btn,
  LoginPage,
  PatchHiveFooter,
  PatchHiveHeader,
  TabBar,
} from "@patchhivehq/ui";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell";
import { API } from "./config.js";
import ReviewPanel from "./panels/ReviewPanel.jsx";
import HistoryPanel from "./panels/HistoryPanel.jsx";
import RulesPanel from "./panels/RulesPanel.jsx";
import ChecksPanel from "./panels/ChecksPanel.jsx";

const TABS = [
  { id: "review", label: "🛡 Review" },
  { id: "rules", label: "Rules" },
  { id: "history", label: "◎ History" },
  { id: "checks", label: "Checks" },
];

const DEFAULT_FORM = {
  repo: "",
  ai_source: "Codex",
  diff: "",
  pr_number: "",
  publish_status: true,
};

function recommendationColor(recommendation) {
  if (recommendation === "safe") {
    return "var(--green)";
  }
  if (recommendation === "warn") {
    return "var(--gold)";
  }
  return "var(--accent)";
}

export default function App() {
  const { apiKey, checked, needsAuth, login, logout } = useApiKeyAuth({
    apiBase: API,
    storageKey: "trust_api_key",
  });
  const [tab, setTab] = useState("review");
  const [running, setRunning] = useState(false);
  const [form, setForm] = useState(DEFAULT_FORM);
  const [review, setReview] = useState(null);
  const [error, setError] = useState("");
  const fetch_ = createApiFetcher(apiKey);

  useEffect(() => {
    applyTheme("trust-gate");
  }, []);

  const runReview = useCallback(async () => {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/review`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          repo: form.repo,
          ai_source: form.ai_source,
          diff: form.diff,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "TrustGate could not review this diff.");
      }
      setReview(data);
      setTab("review");
    } catch (err) {
      setError(err.message || "TrustGate could not review this diff.");
    } finally {
      setRunning(false);
    }
  }, [fetch_, form]);

  const runGitHubReview = useCallback(async () => {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/review/github/pr`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          repo: form.repo,
          pr_number: Number(form.pr_number) || 0,
          ai_source: form.ai_source,
          publish_status: !!form.publish_status,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "TrustGate could not fetch and review that PR.");
      }
      setReview(data);
      setForm((prev) => ({
        ...prev,
        diff: data.diff || prev.diff,
      }));
      setTab("review");
    } catch (err) {
      setError(err.message || "TrustGate could not fetch and review that PR.");
    } finally {
      setRunning(false);
    }
  }, [fetch_, form]);

  const loadHistoryReview = useCallback(
    async (id) => {
      setRunning(true);
      setError("");
      try {
        const res = await fetch_(`${API}/history/${id}`);
        const data = await res.json();
        if (!res.ok) {
          throw new Error(data.error || "TrustGate could not load that review.");
        }
        setReview(data);
        setForm({
          repo: data.repo || "",
          ai_source: data.ai_source || "unknown",
          diff: data.diff || "",
          pr_number: data.github?.pr_number ? String(data.github.pr_number) : "",
          publish_status: true,
        });
        setTab("review");
      } catch (err) {
        setError(err.message || "TrustGate could not load that review.");
      } finally {
        setRunning(false);
      }
    },
    [fetch_]
  );

  if (!checked) {
    return (
      <div
        style={{
          minHeight: "100vh",
          background: "#080810",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          color: "#7b2d8b",
          fontSize: 26,
        }}
      >
        🛡
      </div>
    );
  }

  if (needsAuth) {
    return (
      <LoginPage
        onLogin={login}
        icon="🛡"
        title="TrustGate"
        subtitle="by PatchHive"
        storageKey="trust_api_key"
        apiBase={API}
      />
    );
  }

  return (
    <div
      style={{
        minHeight: "100vh",
        background: "var(--bg)",
        color: "var(--text)",
        fontFamily: "'SF Mono','Fira Mono',monospace",
        fontSize: 12,
      }}
    >
      <PatchHiveHeader icon="🛡" title="TrustGate" version="v0.1.0" running={running}>
        <div style={{ fontSize: 10, color: "var(--text-dim)" }}>
          Review AI-generated diffs before they move forward
        </div>
        {review?.recommendation && (
          <div
            style={{
              fontSize: 10,
              color: recommendationColor(review.recommendation),
              fontWeight: 700,
            }}
          >
            {review.recommendation.toUpperCase()}
          </div>
        )}
        {apiKey && (
          <Btn onClick={logout} style={{ padding: "4px 10px" }}>
            Sign out
          </Btn>
        )}
      </PatchHiveHeader>

      <TabBar tabs={TABS} active={tab} onChange={setTab} />

      <div
        style={{
          padding: 24,
          maxWidth: 1320,
          margin: "0 auto",
          display: "grid",
          gap: 16,
        }}
      >
        {error && (
          <div
            style={{
              border: "1px solid var(--accent)44",
              background: "var(--accent)10",
              color: "var(--accent)",
              borderRadius: 8,
              padding: "12px 14px",
            }}
          >
            {error}
          </div>
        )}

        {tab === "review" && (
          <ReviewPanel
            form={form}
            setForm={setForm}
            running={running}
            onRun={runReview}
            onRunGitHub={runGitHubReview}
            review={review}
            setReview={setReview}
          />
        )}
        {tab === "rules" && <RulesPanel apiKey={apiKey} initialRepo={form.repo} />}
        {tab === "history" && <HistoryPanel apiKey={apiKey} onLoadReview={loadHistoryReview} />}
        {tab === "checks" && <ChecksPanel apiKey={apiKey} />}
      </div>

      <PatchHiveFooter product="TrustGate" />
    </div>
  );
}
