import {
  PHASE_LABEL,
  PHASE_ICON,
  DiffViewer,
} from "@patchhivehq/ui";
import {
  ProductAppFrame,
  ProductSessionGate,
  ProductSetupWizard,
  useApiFetcher,
} from "@patchhivehq/product-shell";
import { API } from "./config.js";
import TeamPanel from "./panels/TeamPanel.jsx";
import RunPanel from "./panels/RunPanel.jsx";
import DryRunPanel from "./panels/DryRunPanel.jsx";
import HistoryPanel from "./panels/HistoryPanel.jsx";
import LeaderboardPanel from "./panels/LeaderboardPanel.jsx";
import PresetsPanel from "./panels/PresetsPanel.jsx";
import RejectedPanel from "./panels/RejectedPanel.jsx";
import ConfigPanel from "./panels/ConfigPanel.jsx";
import {
  RepoListsPanel,
  SchedulesPanel,
  WebhookPanel,
  PRTrackingPanel,
  StartupChecksPanel,
} from "./panels/misc.jsx";
import {
  REPO_REAPER_TABS,
  useRepoReaperApp,
} from "./hooks/useRepoReaperApp.js";

const SETUP_STEPS = [
  {
    title: "Connect bot GitHub identity and AI access",
    detail: "RepoReaper is only as trustworthy as its execution environment. Confirm bot GitHub credentials, AI access, and startup checks before you let it touch a repo.",
    tab: "cfg",
    actionLabel: "Open Config",
  },
  {
    title: "Shape the team and safeguards first",
    detail: "Set your agents, confidence thresholds, repo lists, and watch-mode boundaries before you launch real hunts.",
    tab: "team",
    actionLabel: "Open Team",
  },
  {
    title: "Use dry run before real patching",
    detail: "Start with Dry Stalk so you can inspect output quality and confidence without opening live pull requests yet.",
    tab: "dryrun",
    actionLabel: "Open Dry Stalk",
  },
];

function renderHeaderBadges({ watchMode, hasCooldown, cooldowns, runCost, lifetimeCost }) {
  return (
    <>
      {watchMode && (
        <div style={{ fontSize: 9, color: "var(--green)", border: "1px solid var(--green)44", borderRadius: 3, padding: "2px 7px" }}>
          ● Watch Mode
        </div>
      )}
      {hasCooldown && (
        <div style={{ fontSize: 9, color: "var(--purple)", border: "1px solid var(--purple)33", borderRadius: 3, padding: "2px 7px" }}>
          ⏸ {Object.keys(cooldowns).join(",")} cooling
        </div>
      )}
      {runCost > 0 && <span style={{ fontSize: 10, color: "var(--gold)" }}>Run: ${runCost.toFixed(4)}</span>}
      {lifetimeCost > 0 && <span style={{ fontSize: 10, color: "var(--text-dim)" }}>Lifetime: ${lifetimeCost.toFixed(4)}</span>}
    </>
  );
}

function RepoReaperActivePanel({ app, apiKey, fetch_ }) {
  const { nav, team, run, config, watch, diff } = app;

  switch (nav.tab) {
    case "setup":
      return (
        <ProductSetupWizard
          apiBase={API}
          fetch_={fetch_}
          product="RepoReaper"
          icon="🔱"
          description="RepoReaper is the highest-autonomy tool in the suite, so its first-run path should stay disciplined: prove config, shape safeguards, then dry run before real hunts."
          steps={SETUP_STEPS}
          onOpenTab={nav.setTab}
          checksTabId="startup"
        />
      );
    case "team":
      return (
        <TeamPanel
          agents={team.agents}
          logs={run.logs}
          running={run.running}
          cooldowns={team.cooldowns}
          onAdd={team.addAgent}
          onRemove={team.removeAgent}
          apiKey={apiKey}
          existingConfig={config.existingCfg}
        />
      );
    case "run":
      return (
        <RunPanel
          running={run.running}
          onStart={run.startRun}
          params={run.params}
          setParams={run.setParams}
          issues={run.issues}
          logs={run.logs}
          agents={team.agents}
          runStats={run.runStats}
          runCost={run.runCost}
          onViewDiff={diff.setViewDiff}
        />
      );
    case "dryrun":
      return <DryRunPanel agents={team.agents} apiKey={apiKey} onViewDiff={diff.setViewDiff} />;
    case "history":
      return <HistoryPanel apiKey={apiKey} onViewDiff={diff.setViewDiff} />;
    case "board":
      return <LeaderboardPanel apiKey={apiKey} />;
    case "rejected":
      return <RejectedPanel apiKey={apiKey} onViewDiff={diff.setViewDiff} />;
    case "prs":
      return <PRTrackingPanel apiKey={apiKey} />;
    case "presets":
      return (
        <PresetsPanel
          apiKey={apiKey}
          currentAgents={Object.values(team.agents)}
          onLoadPreset={team.loadPreset}
        />
      );
    case "repos":
      return <RepoListsPanel apiKey={apiKey} />;
    case "sched":
      return <SchedulesPanel apiKey={apiKey} />;
    case "webhook":
      return (
        <WebhookPanel
          watchMode={watch.watchMode}
          onToggleWatch={watch.toggleWatchMode}
        />
      );
    case "startup":
      return <StartupChecksPanel apiKey={apiKey} />;
    case "cfg":
      return (
        <ConfigPanel
          existingConfig={config.existingCfg}
          apiKey={apiKey}
          onSaved={config.refreshConfig}
        />
      );
    default:
      return null;
  }
}

export default function App() {
  const app = useRepoReaperApp();
  const { auth, nav, run } = app;
  const {
    apiKey,
    checked,
    needsAuth,
    login,
    logout,
    authError,
    bootstrapRequired,
    generateKey,
  } = auth;
  const fetch_ = useApiFetcher(apiKey);

  return (
    <ProductSessionGate
      checked={checked}
      needsAuth={needsAuth}
      onLogin={login}
      icon="🔱"
      title="RepoReaper"
      storageKey="reaper_api_key"
      apiBase={API}
      authError={authError}
      bootstrapRequired={bootstrapRequired}
      onGenerateKey={generateKey}
      loadingColor="#1c1c30"
    >
      <ProductAppFrame
        icon="🔱"
        title="RepoReaper"
        product="RepoReaper"
        running={run.running}
        phase={run.phase}
        phaseLabel={PHASE_LABEL}
        phaseIcon={PHASE_ICON}
        headerChildren={renderHeaderBadges({
          watchMode: app.watch.watchMode,
          hasCooldown: app.team.hasCooldown,
          cooldowns: app.team.cooldowns,
          runCost: run.runCost,
          lifetimeCost: run.lifetimeCost,
        })}
        tabs={REPO_REAPER_TABS}
        activeTab={nav.tab}
        onTabChange={nav.setTab}
        maxWidth={1400}
        contentStyle={{ gap: 0 }}
        onSignOut={logout}
        showSignOut={Boolean(apiKey)}
      >
        <RepoReaperActivePanel app={app} apiKey={apiKey} fetch_={fetch_} />
      </ProductAppFrame>

      {app.diff.viewDiff && (
        <DiffViewer diff={app.diff.viewDiff} onClose={() => app.diff.setViewDiff(null)} />
      )}
    </ProductSessionGate>
  );
}
