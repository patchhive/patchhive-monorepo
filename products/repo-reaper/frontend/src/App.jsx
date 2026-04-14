import {
  PHASE_LABEL,
  PHASE_ICON,
  DiffViewer,
} from "@patchhivehq/ui";
import {
  ProductAppFrame,
  ProductSessionGate,
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

export default function App() {
  const {
    apiKey,
    checked,
    needsAuth,
    login,
    logout,
    authError,
    bootstrapRequired,
    generateKey,
    tab,
    setTab,
    agents,
    logs,
    issues,
    running,
    phase,
    cooldowns,
    runCost,
    lifetimeCost,
    viewDiff,
    setViewDiff,
    runStats,
    params,
    setParams,
    existingCfg,
    watchMode,
    refreshConfig,
    addAgent,
    removeAgent,
    loadPreset,
    toggleWatchMode,
    startRun,
    hasCooldown,
  } = useRepoReaperApp();

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
        running={running}
        phase={phase}
        phaseLabel={PHASE_LABEL}
        phaseIcon={PHASE_ICON}
        headerChildren={renderHeaderBadges({ watchMode, hasCooldown, cooldowns, runCost, lifetimeCost })}
        tabs={REPO_REAPER_TABS}
        activeTab={tab}
        onTabChange={setTab}
        maxWidth={1400}
        contentStyle={{ gap: 0 }}
        onSignOut={logout}
        showSignOut={Boolean(apiKey)}
      >
        {tab === "team" && (
          <TeamPanel
            agents={agents}
            logs={logs}
            running={running}
            cooldowns={cooldowns}
            onAdd={addAgent}
            onRemove={removeAgent}
            apiKey={apiKey}
            existingConfig={existingCfg}
          />
        )}
        {tab === "run" && (
          <RunPanel
            running={running}
            onStart={startRun}
            params={params}
            setParams={setParams}
            issues={issues}
            logs={logs}
            agents={agents}
            runStats={runStats}
            runCost={runCost}
            onViewDiff={setViewDiff}
          />
        )}
        {tab === "dryrun" && <DryRunPanel agents={agents} apiKey={apiKey} onViewDiff={setViewDiff} />}
        {tab === "history" && <HistoryPanel apiKey={apiKey} onViewDiff={setViewDiff} />}
        {tab === "board" && <LeaderboardPanel apiKey={apiKey} />}
        {tab === "rejected" && <RejectedPanel apiKey={apiKey} onViewDiff={setViewDiff} />}
        {tab === "prs" && <PRTrackingPanel apiKey={apiKey} />}
        {tab === "presets" && (
          <PresetsPanel
            apiKey={apiKey}
            currentAgents={Object.values(agents)}
            onLoadPreset={loadPreset}
          />
        )}
        {tab === "repos" && <RepoListsPanel apiKey={apiKey} />}
        {tab === "sched" && <SchedulesPanel apiKey={apiKey} />}
        {tab === "webhook" && <WebhookPanel watchMode={watchMode} onToggleWatch={toggleWatchMode} />}
        {tab === "startup" && <StartupChecksPanel apiKey={apiKey} />}
        {tab === "cfg" && <ConfigPanel existingConfig={existingCfg} apiKey={apiKey} onSaved={refreshConfig} />}
      </ProductAppFrame>

      {viewDiff && <DiffViewer diff={viewDiff} onClose={() => setViewDiff(null)} />}
    </ProductSessionGate>
  );
}
