import { Component } from "react";

const fallbackStyle = {
  display: "grid",
  gap: 12,
  padding: 18,
  border: "1px solid color-mix(in srgb, var(--red, #ff6b6b) 30%, var(--border, rgba(255,255,255,0.14)))",
  borderRadius: 12,
  background:
    "linear-gradient(135deg, color-mix(in srgb, var(--red, #ff6b6b) 10%, var(--bg-panel, #111827)) 0%, var(--bg-panel, #111827) 100%)",
  color: "var(--text, #f8fafc)",
};

const kickerStyle = {
  color: "var(--red, #ff6b6b)",
  fontSize: 11,
  fontWeight: 800,
  letterSpacing: "0.08em",
  textTransform: "uppercase",
};

const titleStyle = {
  margin: 0,
  fontSize: 20,
  lineHeight: 1.15,
};

const bodyStyle = {
  margin: 0,
  color: "var(--text-dim, #94a3b8)",
  fontSize: 13,
  lineHeight: 1.55,
};

const detailStyle = {
  maxHeight: 180,
  overflow: "auto",
  padding: 12,
  borderRadius: 8,
  border: "1px solid var(--border, rgba(255,255,255,0.14))",
  background: "color-mix(in srgb, var(--bg, #020617) 70%, transparent)",
  color: "var(--text-dim, #94a3b8)",
  fontSize: 12,
  whiteSpace: "pre-wrap",
};

const buttonStyle = {
  justifySelf: "start",
  border: "1px solid color-mix(in srgb, var(--red, #ff6b6b) 40%, var(--border, rgba(255,255,255,0.14)))",
  borderRadius: 8,
  padding: "8px 12px",
  background: "color-mix(in srgb, var(--red, #ff6b6b) 12%, var(--bg-panel, #111827))",
  color: "var(--text, #f8fafc)",
  cursor: "pointer",
  fontWeight: 800,
};

export default class PanelErrorBoundary extends Component {
  constructor(props) {
    super(props);
    this.state = { error: null, errorInfo: null };
  }

  static getDerivedStateFromError(error) {
    return { error };
  }

  componentDidCatch(error, errorInfo) {
    this.setState({ errorInfo });
    this.props.onError?.(error, errorInfo);
  }

  reset = () => {
    this.setState({ error: null, errorInfo: null });
  };

  render() {
    const { children, label = "panel" } = this.props;
    const { error, errorInfo } = this.state;

    if (!error) {
      return children;
    }

    return (
      <section role="alert" style={fallbackStyle}>
        <div style={kickerStyle}>Panel fault isolated</div>
        <h2 style={titleStyle}>{label} hit a render error.</h2>
        <p style={bodyStyle}>
          The rest of the app is still running. You can retry this panel, switch
          tabs, or refresh telemetry without losing the whole screen.
        </p>
        <pre style={detailStyle}>
          {error?.message || String(error)}
          {errorInfo?.componentStack ? `\n${errorInfo.componentStack}` : ""}
        </pre>
        <button type="button" style={buttonStyle} onClick={this.reset}>
          Retry panel
        </button>
      </section>
    );
  }
}
