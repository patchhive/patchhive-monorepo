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
import KeeperPanel from "./panels/KeeperPanel.jsx";
import HistoryPanel from "./panels/HistoryPanel.jsx";
import ChecksPanel from "./panels/ChecksPanel.jsx";

const TABS = [
  { id: "keeper", label: "🪢 Keeper" },
  { id: "setup", label: "Setup" },
  { id: "history", label: "◎ History" },
  { id: "checks", label: "Checks" },
];

const SETUP_STEPS = [
  {
    title: "Connect GitHub for PR state and review pressure",
    detail: "MergeKeeper depends on GitHub token access first, then optionally on webhook inputs if you want near-real-time merge readiness updates.",
    tab: "checks",
    actionLabel: "Review Checks",
  },
  {
    title: "Start with one known pull request",
    detail: "Run the readiness call on a PR you already understand so the output format and publish behavior feel trustworthy before wider use.",
    tab: "keeper",
    actionLabel: "Open Keeper",
  },
];

export default function App() {
  const { apiKey, checked, needsAuth, login, logout, authError, bootstrapRequired, generateKey } = useApiKeyAuth({
    apiBase: API,
    storageKey: "merge-keeper_api_key",
  });
  const [tab, setTab] = useState("keeper");
  const [form, setForm] = useState({
    repo: "",
    pr_number: "",
    publish_report: true,
  });
  const [assessment, setAssessment] = useState(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState("");
  const [pendingRunId, setPendingRunId] = useState(() => {
    if (typeof window === "undefined") {
      return "";
    }
    return new URLSearchParams(window.location.search).get("run") || "";
  });
  const fetch_ = useApiFetcher(apiKey);

  useEffect(() => {
    applyTheme("merge-keeper");
  }, []);

  async function runAssessment() {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/assess/github/pr`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          repo: form.repo,
          pr_number: Number(form.pr_number) || 0,
          publish_report: !!form.publish_report,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "MergeKeeper could not assess that pull request.");
      }
      setAssessment(data);
      setTab("keeper");
    } catch (err) {
      setError(err.message || "MergeKeeper could not assess that pull request.");
    } finally {
      setRunning(false);
    }
  }

  async function loadHistoryAssessment(id) {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/history/${id}`);
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "MergeKeeper could not load that run.");
      }
      setAssessment(data);
      setForm({
        repo: data.repo || "",
        pr_number: data.pr_number ? String(data.pr_number) : "",
        publish_report: form.publish_report,
      });
      setTab("keeper");
    } catch (err) {
      setError(err.message || "MergeKeeper could not load that run.");
    } finally {
      setRunning(false);
    }
  }

  useEffect(() => {
    if (!checked || needsAuth || !pendingRunId || running) {
      return;
    }
    loadHistoryAssessment(pendingRunId);
    setPendingRunId("");
  }, [checked, needsAuth, pendingRunId, running]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }

    const url = new URL(window.location.href);
    if (assessment?.id) {
      url.searchParams.set("run", assessment.id);
    } else {
      url.searchParams.delete("run");
    }
    window.history.replaceState({}, "", url.toString());
  }, [assessment?.id]);

  return (
    <ProductSessionGate
      checked={checked}
      needsAuth={needsAuth}
      onLogin={login}
      icon="🪢"
      title="MergeKeeper"
      storageKey="merge-keeper_api_key"
      apiBase={API}
      authError={authError}
      bootstrapRequired={bootstrapRequired}
      onGenerateKey={generateKey}
    >
      <ProductAppFrame
        icon="🪢"
        title="MergeKeeper"
        product="MergeKeeper"
        running={running}
        headerChildren={
          <>
            <div style={{ fontSize: 10, color: "var(--text-dim)" }}>
              Keep pull requests mergeable by turning GitHub merge pressure into a clear readiness call.
            </div>
            {assessment?.readiness && (
              <div
                style={{
                  fontSize: 10,
                  color:
                    assessment.readiness === "ready"
                      ? "var(--green)"
                      : assessment.readiness === "blocked"
                        ? "var(--accent)"
                        : "var(--gold)",
                  fontWeight: 700,
                }}
              >
                {assessment.readiness.toUpperCase()}
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
            product="MergeKeeper"
            icon="🪢"
            description="MergeKeeper should earn trust by making a few clear readiness calls first. This shared wizard keeps the boot path consistent with the rest of the suite."
            steps={SETUP_STEPS}
            onOpenTab={setTab}
          />
        )}
        {tab === "keeper" && (
          <KeeperPanel
            apiKey={apiKey}
            form={form}
            setForm={setForm}
            running={running}
            onRun={runAssessment}
            assessment={assessment}
            onLoadAssessment={loadHistoryAssessment}
          />
        )}
        {tab === "history" && (
          <HistoryPanel
            apiKey={apiKey}
            onLoadAssessment={loadHistoryAssessment}
            activeAssessmentId={assessment?.id || ""}
          />
        )}
        {tab === "checks" && <ChecksPanel apiKey={apiKey} />}
      </ProductAppFrame>
    </ProductSessionGate>
  );
}
