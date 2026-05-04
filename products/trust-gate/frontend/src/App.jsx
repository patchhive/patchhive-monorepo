import { useCallback, useEffect, useState } from "react";
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
import RulesPanel from "./panels/RulesPanel.jsx";
import ChecksPanel from "./panels/ChecksPanel.jsx";
import DecisionViewPage from "./components/DecisionViewPage.jsx";

const TABS = [
  { id: "review", label: "🛡 Review" },
  { id: "setup", label: "Setup" },
  { id: "rules", label: "Rules" },
  { id: "history", label: "◎ History" },
  { id: "checks", label: "Checks" },
];

const SETUP_STEPS = [
  {
    title: "Connect GitHub and optional webhook inputs",
    detail: "Enable GitHub token access first, then add webhook secret wiring if you want live PR-triggered review instead of pasted diffs only.",
    tab: "checks",
    actionLabel: "Review Checks",
  },
  {
    title: "Tune repo rules before trusting live reviews",
    detail: "Blocked paths, suspicious terms, test expectations, and scope budgets should match the repos you plan to review.",
    tab: "rules",
    actionLabel: "Open Rules",
  },
  {
    title: "Start with a pasted diff",
    detail: "Validate the recommendation style on a small manual diff first, then move up to GitHub PR review and report publishing.",
    tab: "review",
    actionLabel: "Open Review",
  },
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

function resolveRoute(pathname) {
  const path = pathname || "/";
  const historyMatch = path.match(/^\/history\/([^/]+)$/);
  if (historyMatch) {
    return { kind: "decision", reviewId: decodeURIComponent(historyMatch[1]), printMode: false };
  }
  const printMatch = path.match(/^\/print\/([^/]+)$/);
  if (printMatch) {
    return { kind: "decision", reviewId: decodeURIComponent(printMatch[1]), printMode: true };
  }
  return { kind: "app" };
}

export default function App() {
  const { apiKey, checked, needsAuth, login, logout, authError, bootstrapRequired, generateKey } = useApiKeyAuth({
    apiBase: API,
    storageKey: "trust_api_key",
  });
  const [tab, setTab] = useState("review");
  const [running, setRunning] = useState(false);
  const [form, setForm] = useState(DEFAULT_FORM);
  const [review, setReview] = useState(null);
  const [error, setError] = useState("");
  const [route, setRoute] = useState(() => resolveRoute(window.location.pathname));
  const fetch_ = useApiFetcher(apiKey);

  useEffect(() => {
    applyTheme("trust-gate");
  }, []);

  useEffect(() => {
    const onPopState = () => setRoute(resolveRoute(window.location.pathname));
    window.addEventListener("popstate", onPopState);
    return () => window.removeEventListener("popstate", onPopState);
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

  return (
    <ProductSessionGate
      checked={checked}
      needsAuth={needsAuth}
      onLogin={login}
      icon="🛡"
      title="TrustGate"
      storageKey="trust_api_key"
      apiBase={API}
      authError={authError}
      bootstrapRequired={bootstrapRequired}
      onGenerateKey={generateKey}
      loadingColor="#7b2d8b"
    >
      {route.kind === "decision" ? (
        <DecisionViewPage apiKey={apiKey} reviewId={route.reviewId} printMode={route.printMode} />
      ) : (
        <ProductAppFrame
          icon="🛡"
          title="TrustGate"
          product="TrustGate"
          running={running}
          headerChildren={
            <>
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
            </>
          }
          tabs={TABS}
          activeTab={tab}
          onTabChange={setTab}
          error={error}
          onSignOut={logout}
          showSignOut={Boolean(apiKey)}
        >
          {tab === "setup" && (
            <ProductSetupWizard
              apiBase={API}
              fetch_={fetch_}
              product="TrustGate"
              icon="🛡"
              description="TrustGate should feel predictable before it feels powerful. Use this wizard to clear backend readiness, shape repo rules, and validate one conservative review path."
              steps={SETUP_STEPS}
              onOpenTab={setTab}
            />
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
        </ProductAppFrame>
      )}
    </ProductSessionGate>
  );
}
