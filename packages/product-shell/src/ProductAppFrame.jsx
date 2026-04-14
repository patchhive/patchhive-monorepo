import { Btn, LoginPage, PatchHiveFooter, PatchHiveHeader, TabBar } from "@patchhivehq/ui";

const loadingShellStyle = {
  minHeight: "100vh",
  background: "#080810",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  fontSize: 26,
};

const appShellStyle = {
  minHeight: "100vh",
  background: "var(--bg)",
  color: "var(--text)",
  fontFamily: "'SF Mono','Fira Mono',monospace",
  fontSize: 12,
};

const errorBannerStyle = {
  border: "1px solid var(--accent)44",
  background: "var(--accent)10",
  color: "var(--accent)",
  borderRadius: 8,
  padding: "12px 14px",
};

export function ProductSessionGate({
  checked,
  needsAuth,
  onLogin,
  icon,
  title,
  subtitle = "by PatchHive",
  storageKey,
  apiBase,
  authError,
  bootstrapRequired,
  onGenerateKey,
  loadingColor = "var(--accent)",
  children,
}) {
  if (!checked) {
    return (
      <div style={{ ...loadingShellStyle, color: loadingColor }}>
        {icon}
      </div>
    );
  }

  if (needsAuth) {
    return (
      <LoginPage
        onLogin={onLogin}
        icon={icon}
        title={title}
        subtitle={subtitle}
        storageKey={storageKey}
        apiBase={apiBase}
        authError={authError}
        bootstrapRequired={bootstrapRequired}
        onGenerateKey={onGenerateKey}
      />
    );
  }

  return children;
}

export function ProductAppFrame({
  icon,
  title,
  product = title,
  version = "v0.1.0",
  running = false,
  phase,
  phaseLabel,
  phaseIcon,
  headerChildren,
  tabs = [],
  activeTab = "",
  onTabChange,
  error = "",
  maxWidth = 1320,
  contentStyle = {},
  shellStyle = {},
  onSignOut,
  showSignOut = true,
  signOutLabel = "Sign out",
  children,
}) {
  const resolvedContentStyle = {
    padding: 24,
    maxWidth,
    margin: "0 auto",
    display: "grid",
    gap: 16,
    ...contentStyle,
  };

  return (
    <div style={{ ...appShellStyle, ...shellStyle }}>
      <PatchHiveHeader
        icon={icon}
        title={title}
        version={version}
        phase={phase}
        phaseLabel={phaseLabel}
        phaseIcon={phaseIcon}
        running={running}
      >
        {headerChildren}
        {showSignOut && onSignOut && (
          <Btn onClick={onSignOut} style={{ padding: "4px 10px" }}>
            {signOutLabel}
          </Btn>
        )}
      </PatchHiveHeader>

      {tabs.length > 0 && onTabChange && (
        <TabBar tabs={tabs} active={activeTab} onChange={onTabChange} />
      )}

      <div style={resolvedContentStyle}>
        {error && <div style={errorBannerStyle}>{error}</div>}
        {children}
      </div>

      <PatchHiveFooter product={product} />
    </div>
  );
}
