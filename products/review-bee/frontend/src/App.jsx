import { useEffect, useState } from "react";
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
import ChecksPanel from "./panels/ChecksPanel.jsx";

const TABS = [
  { id: "review", label: "🐝 Review" },
  { id: "history", label: "◎ History" },
  { id: "checks", label: "Checks" },
];

export default function App() {
  const { apiKey, checked, needsAuth, login, logout } = useApiKeyAuth({
    apiBase: API,
    storageKey: "review-bee_api_key",
  });
  const [tab, setTab] = useState("review");
  const [form, setForm] = useState({
    repo: "",
    pr_number: "",
  });
  const [review, setReview] = useState(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState("");
  const fetch_ = createApiFetcher(apiKey);

  useEffect(() => {
    applyTheme("review-bee");
  }, []);

  async function runReview() {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/review/github/pr`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          repo: form.repo,
          pr_number: Number(form.pr_number) || 0,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "ReviewBee could not review that pull request.");
      }
      setReview(data);
      setTab("review");
    } catch (err) {
      setError(err.message || "ReviewBee could not review that pull request.");
    } finally {
      setRunning(false);
    }
  }

  async function loadHistoryReview(id) {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/history/${id}`);
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "ReviewBee could not load that review.");
      }
      setReview(data);
      setForm({
        repo: data.repo || "",
        pr_number: data.pr_number ? String(data.pr_number) : "",
      });
      setTab("review");
    } catch (err) {
      setError(err.message || "ReviewBee could not load that review.");
    } finally {
      setRunning(false);
    }
  }

  if (!checked) {
    return (
      <div style={{ minHeight: "100vh", background: "#080810", display: "flex", alignItems: "center", justifyContent: "center", color: "var(--accent)", fontSize: 26 }}>
        🐝
      </div>
    );
  }

  if (needsAuth) {
    return (
      <LoginPage
        onLogin={login}
        icon="🐝"
        title="ReviewBee"
        subtitle="by PatchHive"
        storageKey="review-bee_api_key"
        apiBase={API}
      />
    );
  }

  return (
    <div style={{ minHeight: "100vh", background: "var(--bg)", color: "var(--text)", fontFamily: "'SF Mono','Fira Mono',monospace", fontSize: 12 }}>
      <PatchHiveHeader icon="🐝" title="ReviewBee" version="v0.1.0" running={running}>
        <div style={{ fontSize: 10, color: "var(--text-dim)" }}>Turn reviewer churn into a concrete merge checklist.</div>
        {review?.status && (
          <div style={{ fontSize: 10, color: review.status === "clear" ? "var(--green)" : review.status === "attention" ? "var(--accent)" : "var(--gold)", fontWeight: 700 }}>
            {review.status.toUpperCase()}
          </div>
        )}
        {apiKey && (
          <Btn onClick={logout} style={{ padding: "4px 10px" }}>
            Sign out
          </Btn>
        )}
      </PatchHiveHeader>

      <TabBar tabs={TABS} active={tab} onChange={setTab} />

      <div style={{ padding: 24, maxWidth: 1200, margin: "0 auto", display: "grid", gap: 16 }}>
        {error && (
          <div style={{ border: "1px solid var(--accent)44", background: "var(--accent)10", color: "var(--accent)", borderRadius: 8, padding: "12px 14px" }}>
            {error}
          </div>
        )}
        {tab === "review" && (
          <ReviewPanel
            apiKey={apiKey}
            form={form}
            setForm={setForm}
            running={running}
            onRun={runReview}
            review={review}
            onLoadReview={loadHistoryReview}
          />
        )}
        {tab === "history" && (
          <HistoryPanel
            apiKey={apiKey}
            onLoadReview={loadHistoryReview}
            activeReviewId={review?.id || ""}
          />
        )}
        {tab === "checks" && <ChecksPanel apiKey={apiKey} />}
      </div>

      <PatchHiveFooter product="ReviewBee" />
    </div>
  );
}
