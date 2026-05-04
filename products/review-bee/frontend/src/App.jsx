import { useEffect, useState } from "react";
import { applyTheme } from "@patchhivehq/ui";
import {
  ProductAppFrame,
  ProductSessionGate,
  ProductSetupWizard,
  useApiFetcher,
  useApiKeyAuth,
} from "@patchhivehq/product-shell";
import { API } from "./config.js";
import ReviewPanel from "./panels/ReviewPanel.jsx";
import HistoryPanel from "./panels/HistoryPanel.jsx";
import ChecksPanel from "./panels/ChecksPanel.jsx";

const TABS = [
  { id: "review", label: "🐝 Review" },
  { id: "setup", label: "Setup" },
  { id: "history", label: "◎ History" },
  { id: "checks", label: "Checks" },
];

const SETUP_STEPS = [
  {
    title: "Connect GitHub and webhook plumbing",
    detail: "ReviewBee needs GitHub token access for PR reads, and a webhook secret only if you want automatic refresh from live PR events.",
    tab: "checks",
    actionLabel: "Review Checks",
  },
  {
    title: "Start with one known pull request",
    detail: "Run the checklist flow on a familiar PR before relying on comment publishing or wider review queue triage.",
    tab: "review",
    actionLabel: "Open Review",
  },
];

export default function App() {
  const { apiKey, checked, needsAuth, login, logout, authError, bootstrapRequired, generateKey } = useApiKeyAuth({
    apiBase: API,
    storageKey: "review-bee_api_key",
  });
  const [tab, setTab] = useState("review");
  const [form, setForm] = useState({
    repo: "",
    pr_number: "",
    publish_comment: true,
  });
  const [review, setReview] = useState(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState("");
  const fetch_ = useApiFetcher(apiKey);

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
          publish_comment: !!form.publish_comment,
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
        publish_comment: true,
      });
      setTab("review");
    } catch (err) {
      setError(err.message || "ReviewBee could not load that review.");
    } finally {
      setRunning(false);
    }
  }

  return (
    <ProductSessionGate
      checked={checked}
      needsAuth={needsAuth}
      onLogin={login}
      icon="🐝"
      title="ReviewBee"
      storageKey="review-bee_api_key"
      apiBase={API}
      authError={authError}
      bootstrapRequired={bootstrapRequired}
      onGenerateKey={generateKey}
    >
      <ProductAppFrame
        icon="🐝"
        title="ReviewBee"
        product="ReviewBee"
        running={running}
        headerChildren={
          <>
            <div style={{ fontSize: 10, color: "var(--text-dim)" }}>
              Turn reviewer churn into a concrete merge checklist.
            </div>
            {review?.status && (
              <div
                style={{
                  fontSize: 10,
                  color:
                    review.status === "clear"
                      ? "var(--green)"
                      : review.status === "attention"
                        ? "var(--accent)"
                        : "var(--gold)",
                  fontWeight: 700,
                }}
              >
                {review.status.toUpperCase()}
              </div>
            )}
          </>
        }
        tabs={TABS}
        activeTab={tab}
        onTabChange={setTab}
        error={error}
        maxWidth={1200}
        onSignOut={logout}
        showSignOut={Boolean(apiKey)}
      >
        {tab === "setup" && (
          <ProductSetupWizard
            apiBase={API}
            fetch_={fetch_}
            product="ReviewBee"
            icon="🐝"
            description="ReviewBee’s setup should stay lightweight: make GitHub access real, confirm startup checks, then validate the checklist experience on one pull request."
            steps={SETUP_STEPS}
            onOpenTab={setTab}
          />
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
      </ProductAppFrame>
    </ProductSessionGate>
  );
}
