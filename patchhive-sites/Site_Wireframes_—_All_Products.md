# Site Wireframes — All Products

Click any element in a wireframe to request changes to copy, layout, colors, or sections.

## PatchHive — Umbrella Site (`patchhive.dev`)

```wireframe

<html>
<head>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; font-family: system-ui, sans-serif; }
  body { background: #0f0f0f; color: #e8e8e8; }
  nav { display: flex; align-items: center; justify-content: space-between; padding: 16px 40px; border-bottom: 1px solid #222; background: #0f0f0f; }
  .logo { font-weight: 800; font-size: 16px; color: #eab308; }
  .nav-right { display: flex; align-items: center; gap: 16px; }
  .nav-link { font-size: 13px; color: #888; cursor: pointer; }
  .cta-btn { background: #eab308; color: #0f0f0f; border: none; border-radius: 6px; padding: 8px 18px; font-size: 13px; font-weight: 700; cursor: pointer; }
  .hero { text-align: center; padding: 100px 40px 60px; max-width: 860px; margin: 0 auto; }
  .hero h1 { font-size: 44px; font-weight: 800; line-height: 1.12; margin-bottom: 20px; }
  .hero h1 span { color: #eab308; }
  .hero p { font-size: 18px; color: #aaa; margin-bottom: 32px; max-width: 600px; margin-left: auto; margin-right: auto; }
  .hero-actions { display: flex; gap: 12px; justify-content: center; align-items: center; margin-bottom: 32px; }
  .hero-secondary { font-size: 13px; color: #666; cursor: pointer; }
  .section { padding: 64px 40px; max-width: 900px; margin: 0 auto; }
  .section-label { font-size: 11px; text-transform: uppercase; letter-spacing: 2px; color: #666; margin-bottom: 12px; }
  h2 { font-size: 26px; font-weight: 700; margin-bottom: 16px; }
  .divider { border: none; border-top: 1px solid #1a1a1a; margin: 0 40px; }
  .principles { display: flex; gap: 24px; margin-top: 24px; flex-wrap: wrap; }
  .principle { flex: 1; min-width: 200px; background: #111; border: 1px solid #1e1e1e; border-radius: 8px; padding: 20px; }
  .principle h3 { font-size: 14px; font-weight: 700; color: #eab308; margin-bottom: 8px; }
  .principle p { font-size: 13px; color: #888; line-height: 1.5; }
  .product-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 16px; margin-top: 24px; }
  .product-card { background: #111; border: 1px solid #1e1e1e; border-radius: 8px; padding: 18px; cursor: pointer; }
  .product-card:hover { border-color: #333; }
  .card-name { font-size: 13px; font-weight: 700; margin-bottom: 6px; }
  .card-pitch { font-size: 12px; color: #666; line-height: 1.4; }
  .card-role { font-size: 10px; text-transform: uppercase; letter-spacing: 1px; margin-top: 10px; }
  .hivecore-box { background: #111; border: 1px solid #a78bfa44; border-radius: 10px; padding: 32px; margin-top: 24px; }
  .hivecore-box h3 { font-size: 18px; font-weight: 700; color: #a78bfa; margin-bottom: 10px; }
  .hivecore-box p { font-size: 14px; color: #888; line-height: 1.6; }
  .trust-box { background: #111; border: 1px solid #1e1e1e; border-radius: 10px; padding: 32px; margin-top: 24px; }
  .trust-box h3 { font-size: 16px; font-weight: 700; margin-bottom: 10px; }
  .trust-box p { font-size: 13px; color: #888; line-height: 1.6; }
  .pr-footer-example { background: #0a0a0a; border: 1px solid #222; border-radius: 6px; padding: 14px 16px; margin-top: 14px; font-family: monospace; font-size: 12px; color: #666; }
  .cta-banner { background: #111; border: 1px solid #eab30833; border-radius: 12px; padding: 48px; text-align: center; margin: 0 40px; }
  .cta-banner h2 { font-size: 24px; margin-bottom: 12px; }
  .trust-line { font-size: 12px; color: #555; margin-top: 12px; }
  footer { border-top: 1px solid #1a1a1a; padding: 32px 40px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 16px; }
  .footer-brand { font-size: 13px; color: #555; }
  .footer-links { display: flex; gap: 14px; flex-wrap: wrap; }
  .footer-links a { font-size: 12px; color: #555; text-decoration: none; }
  .ph-badge { font-size: 11px; color: #eab308; border: 1px solid #eab30844; border-radius: 4px; padding: 2px 8px; text-decoration: none; }
</style>
</head>
<body>

<nav>
  <span class="logo" data-element-id="nav-logo">PatchHive</span>
  <div class="nav-right">
    <span class="nav-link" data-element-id="nav-products">All Products ▾</span>
    <button class="cta-btn" data-element-id="nav-cta">View GitHub</button>
  </div>
</nav>

<div class="hero">
  <h1 data-element-id="hero-headline">An autonomous open source contributor<br><span>that shows up in repos you've never touched.</span></h1>
  <p data-element-id="hero-subhead">Add a GitHub token. Pick your topics and languages. Run it. PatchHive finds the repos, finds the work, and files it under its own name.</p>
  <div class="hero-actions">
    <button class="cta-btn" data-element-id="hero-cta">View GitHub</button>
    <span class="hero-secondary" data-element-id="hero-secondary">See how it works ↓</span>
  </div>
</div>

<hr class="divider">

<div class="section" data-element-id="what-it-is">
  <div class="section-label">What PatchHive Is</div>
  <h2>Not an assistant. An agent with a GitHub identity.</h2>
  <div class="principles">
    <div class="principle" data-element-id="principle-setup"><h3>Three steps. Then nothing.</h3><p>Configure GitHub access. Pick your topics and languages. Run it. Everything after that is the agent.</p></div>
    <div class="principle" data-element-id="principle-1"><h3>It finds the repos itself.</h3><p>You don't pick repos. You don't pick issues. PatchHive searches GitHub for repos matching your interests and decides what's worth doing.</p></div>
    <div class="principle" data-element-id="principle-2"><h3>Signed contributions.</h3><p>Every PR, every report, every fix is attributed to the PatchHive GitHub account. Maintainers see its history and decide whether to trust it.</p></div>
    <div class="principle" data-element-id="principle-3"><h3>Earned reputation.</h3><p>The reputation builds through output, not marketing. Quality PRs across the ecosystem — that's how PatchHive becomes a known contributor.</p></div>
  </div>
</div>

<hr class="divider">

<div class="section" data-element-id="suite-section">
  <div class="section-label">The Suite</div>
  <h2>10 standalone products. One suite. FailGuard in the loop.</h2>
  <div class="product-grid">
    <div class="product-card" data-element-id="card-signalhive"><div class="card-name" style="color:#5b5bd6;">SignalHive</div><div class="card-pitch">Scans the ecosystem for signals before HiveCore acts.</div><div class="card-role" style="color:#5b5bd6;">The eyes</div></div>
    <div class="product-card" data-element-id="card-reviewbee"><div class="card-name" style="color:#f59e0b;">ReviewBee</div><div class="card-pitch">Turns reviewer comments into concrete checklists.</div><div class="card-role" style="color:#f59e0b;">PR clarity</div></div>
    <div class="product-card" data-element-id="card-trustgate"><div class="card-name" style="color:#ef4444;">TrustGate</div><div class="card-pitch">Reviews AI-generated diffs before they move forward.</div><div class="card-role" style="color:#ef4444;">The judgment</div></div>
    <div class="product-card" data-element-id="card-repomemory"><div class="card-name" style="color:#8b5cf6;">RepoMemory</div><div class="card-pitch">Builds a durable knowledge layer from repo history.</div><div class="card-role" style="color:#8b5cf6;">The memory</div></div>
    <div class="product-card" data-element-id="card-mergekeeper"><div class="card-name" style="color:#10b981;">MergeKeeper</div><div class="card-pitch">Catches stale branches and merge conflicts early.</div><div class="card-role" style="color:#10b981;">PR health</div></div>
    <div class="product-card" data-element-id="card-flakesting"><div class="card-name" style="color:#f97316;">FlakeSting</div><div class="card-pitch">Isolates flaky tests before they destroy CI trust.</div><div class="card-role" style="color:#f97316;">CI clarity</div></div>
    <div class="product-card" data-element-id="card-deptriage"><div class="card-name" style="color:#06b6d4;">DepTriage</div><div class="card-pitch">Ranks dependency updates by urgency and risk.</div><div class="card-role" style="color:#06b6d4;">Dep health</div></div>
    <div class="product-card" data-element-id="card-vulntriage"><div class="card-name" style="color:#dc2626;">VulnTriage</div><div class="card-pitch">Turns security alerts into ranked engineering tasks.</div><div class="card-role" style="color:#dc2626;">Security</div></div>
    <div class="product-card" data-element-id="card-refactorscout"><div class="card-name" style="color:#84cc16;">RefactorScout</div><div class="card-pitch">Surfaces bounded, low-risk refactor opportunities.</div><div class="card-role" style="color:#84cc16;">Code health</div></div>
    <div class="product-card" data-element-id="card-reporeaper"><div class="card-name" style="color:#6366f1;">RepoReaper</div><div class="card-pitch">Hunts bugs, patches them, files PRs. Under PatchHive's name.</div><div class="card-role" style="color:#6366f1;">The hands</div></div>
  </div>
  <div class="hivecore-box" data-element-id="hivecore-box">
    <h3>Powered by HiveCore</h3>
    <p>HiveCore is the control plane that brings standalone PatchHive products into one interface while each product keeps its own APIs and decisions.</p>
  </div>
</div>

<hr class="divider">

<div class="section" data-element-id="trust-section">
  <div class="section-label">Identity & Trust</div>
  <h2>Every contribution is signed. Every PR is reviewable.</h2>
  <div class="trust-box">
    <h3>The PatchHive GitHub account</h3>
    <p>PatchHive has its own GitHub identity. Every PR it files carries a standard attribution footer. No apology for being an agent — just clear attribution and confidence that the work stands on its own.</p>
    <div class="pr-footer-example" data-element-id="pr-footer">This PR was generated autonomously by RepoReaper, a PatchHive tool.<br>Review it like any other PR. · patchhive.dev</div>
  </div>
</div>

<div class="cta-banner" data-element-id="cta-banner">
  <h2>View GitHub.</h2>
  <button class="cta-btn" data-element-id="cta-banner-btn">View GitHub</button>
  <div class="trust-line">Every contribution is signed. Every PR is reviewable. No write access without your say.</div>
</div>

<footer>
  <div class="footer-brand">© PatchHive · patchhive.dev</div>
  <div class="footer-links">
    <a href="#">SignalHive</a><a href="#">ReviewBee</a><a href="#">TrustGate</a><a href="#">RepoMemory</a><a href="#">MergeKeeper</a>
    <a href="#">FlakeSting</a><a href="#">DepTriage</a><a href="#">VulnTriage</a><a href="#">RefactorScout</a><a href="#">RepoReaper</a>
  </div>
  <a href="https://patchhive.dev" class="ph-badge" data-element-id="ph-badge">Powered by PatchHive</a>
</footer>

</body>
</html>
```

## HiveCore (`hivecore.patchhive.dev`) — accent: lavender `#a78bfa`

```wireframe

<html>
<head>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; font-family: system-ui, sans-serif; }
  body { background: #0f0f0f; color: #e8e8e8; }
  nav { display: flex; align-items: center; justify-content: space-between; padding: 16px 40px; border-bottom: 1px solid #222; }
  .logo { font-weight: 700; font-size: 15px; color: #fff; }
  .badge { background: #1a1a2e; border: 1px solid #333; border-radius: 4px; padding: 2px 8px; font-size: 11px; color: #a78bfa; margin-left: 10px; }
  .cta-btn { background: #a78bfa; color: #0f0f0f; border: none; border-radius: 6px; padding: 8px 18px; font-size: 13px; font-weight: 700; cursor: pointer; }
  .nav-link { font-size: 13px; color: #888; cursor: pointer; margin-right: 16px; }
  .hero { text-align: center; padding: 100px 40px 60px; max-width: 860px; margin: 0 auto; }
  .hero h1 { font-size: 44px; font-weight: 800; line-height: 1.12; margin-bottom: 20px; }
  .hero h1 span { color: #a78bfa; }
  .hero p { font-size: 18px; color: #aaa; margin-bottom: 32px; }
  .hero-actions { display: flex; gap: 12px; justify-content: center; }
  .secondary { font-size: 13px; color: #666; cursor: pointer; }
  .section { padding: 64px 40px; max-width: 900px; margin: 0 auto; }
  .section-label { font-size: 11px; text-transform: uppercase; letter-spacing: 2px; color: #666; margin-bottom: 12px; }
  h2 { font-size: 26px; font-weight: 700; margin-bottom: 16px; }
  .divider { border: none; border-top: 1px solid #1a1a1a; margin: 0 40px; }
  .loop-steps { display: flex; flex-direction: column; gap: 0; margin-top: 24px; position: relative; }
  .loop-step { display: flex; gap: 20px; align-items: flex-start; padding: 20px 0; border-left: 2px solid #1e1e1e; padding-left: 24px; margin-left: 15px; }
  .loop-dot { width: 12px; height: 12px; background: #a78bfa; border-radius: 50%; position: absolute; left: 10px; margin-top: 4px; }
  .loop-step-inner { position: relative; }
  .loop-step h3 { font-size: 14px; font-weight: 700; margin-bottom: 4px; }
  .loop-step p { font-size: 13px; color: #888; }
  .specialist-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 12px; margin-top: 20px; }
  .specialist { background: #111; border: 1px solid #1e1e1e; border-radius: 8px; padding: 16px; }
  .specialist h3 { font-size: 13px; font-weight: 700; margin-bottom: 4px; }
  .specialist p { font-size: 12px; color: #666; }
  .cta-banner { background: #111; border: 1px solid #a78bfa33; border-radius: 12px; padding: 48px; text-align: center; margin: 0 40px; }
  .cta-banner h2 { font-size: 24px; margin-bottom: 12px; }
  .trust-line { font-size: 12px; color: #555; margin-top: 12px; }
  footer { border-top: 1px solid #1a1a1a; padding: 32px 40px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 16px; }
  .footer-links { display: flex; gap: 14px; flex-wrap: wrap; }
  .footer-links a { font-size: 12px; color: #555; text-decoration: none; }
  .ph-badge { font-size: 11px; color: #eab308; border: 1px solid #eab30844; border-radius: 4px; padding: 2px 8px; text-decoration: none; }
</style>
</head>
<body>

<nav>
  <div style="display:flex;align-items:center;">
    <span class="logo">PatchHive</span>
    <span class="badge">HiveCore</span>
  </div>
  <div style="display:flex;align-items:center;">
    <span class="nav-link" data-element-id="nav-products">All Products ▾</span>
    <button class="cta-btn" data-element-id="nav-cta">View HiveCore</button>
  </div>
</nav>

<div class="hero">
  <h1 data-element-id="hero-headline">The control plane for<br><span>every PatchHive product.</span></h1>
  <p data-element-id="hero-subhead">HiveCore brings the PatchHive product suite into one interface for health, shared defaults, run history, and explicit product handoffs.</p>
  <div class="hero-actions">
    <button class="cta-btn" data-element-id="hero-cta">View HiveCore</button>
    <span class="secondary" data-element-id="hero-secondary">See how it works ↓</span>
  </div>
</div>

<hr class="divider">

<div class="section" data-element-id="how-it-works">
  <div class="section-label">How It Works</div>
  <h2>The autonomous loop — closed without human intervention.</h2>
  <div style="position:relative;margin-top:24px;">
    <div style="position:absolute;left:10px;top:0;bottom:0;width:2px;background:#1e1e1e;"></div>
    <div style="display:flex;flex-direction:column;gap:0;">
      <div style="display:flex;gap:20px;padding:0 0 24px 36px;position:relative;" data-element-id="step-1"><div style="width:12px;height:12px;background:#a78bfa;border-radius:50%;position:absolute;left:5px;top:4px;"></div><div><h3 style="font-size:14px;font-weight:700;margin-bottom:4px;">Query SignalHive</h3><p style="font-size:13px;color:#888;">HiveCore wakes up on schedule and asks SignalHive what signals are worth acting on across the ecosystem.</p></div></div>
      <div style="display:flex;gap:20px;padding:0 0 24px 36px;position:relative;" data-element-id="step-2"><div style="width:12px;height:12px;background:#a78bfa;border-radius:50%;position:absolute;left:5px;top:4px;"></div><div><h3 style="font-size:14px;font-weight:700;margin-bottom:4px;">Score for impact and urgency</h3><p style="font-size:13px;color:#888;">Signals are ranked. HiveCore decides which ones are worth acting on now versus later.</p></div></div>
      <div style="display:flex;gap:20px;padding:0 0 24px 36px;position:relative;" data-element-id="step-3"><div style="width:12px;height:12px;background:#a78bfa;border-radius:50%;position:absolute;left:5px;top:4px;"></div><div><h3 style="font-size:14px;font-weight:700;margin-bottom:4px;">Dispatch the right specialist</h3><p style="font-size:13px;color:#888;">HiveCore routes each signal to the appropriate tool — RepoReaper for fixes, TrustGate for policy checks, and so on.</p></div></div>
      <div style="display:flex;gap:20px;padding:0 0 24px 36px;position:relative;" data-element-id="step-4"><div style="width:12px;height:12px;background:#a78bfa;border-radius:50%;position:absolute;left:5px;top:4px;"></div><div><h3 style="font-size:14px;font-weight:700;margin-bottom:4px;">Act and attribute</h3><p style="font-size:13px;color:#888;">The specialist does the work. Every output — PR, report, check — is signed by the PatchHive GitHub account.</p></div></div>
      <div style="display:flex;gap:20px;padding:0 0 0 36px;position:relative;" data-element-id="step-5"><div style="width:12px;height:12px;background:#a78bfa;border-radius:50%;position:absolute;left:5px;top:4px;"></div><div><h3 style="font-size:14px;font-weight:700;margin-bottom:4px;">Track outcomes</h3><p style="font-size:13px;color:#888;">HiveCore records what was filed, merged, or closed. The loop informs the next cycle.</p></div></div>
    </div>
  </div>
</div>

<hr class="divider">

<div class="section" data-element-id="specialists">
  <div class="section-label">The Specialists</div>
  <h2>HiveCore's capabilities.</h2>
  <div class="specialist-grid">
    <div class="specialist" data-element-id="sp-signalhive"><h3 style="color:#5b5bd6;">SignalHive gives HiveCore eyes.</h3><p>Ecosystem-wide reconnaissance before any action is taken.</p></div>
    <div class="specialist" data-element-id="sp-reporeaper"><h3 style="color:#6366f1;">RepoReaper gives HiveCore hands.</h3><p>Generates fixes and files PRs under PatchHive's identity.</p></div>
    <div class="specialist" data-element-id="sp-trustgate"><h3 style="color:#ef4444;">TrustGate gives HiveCore judgment.</h3><p>Policy checks before any diff moves forward.</p></div>
    <div class="specialist" data-element-id="sp-repomemory"><h3 style="color:#8b5cf6;">RepoMemory gives HiveCore memory.</h3><p>Repo-specific knowledge so the agent doesn't repeat mistakes.</p></div>
  </div>
</div>

<div class="cta-banner" data-element-id="cta-banner">
  <h2>View HiveCore.</h2>
  <button class="cta-btn" data-element-id="cta-banner-btn">View HiveCore</button>
  <div class="trust-line">Every action is signed. Every PR is reviewable.</div>
</div>

<footer>
  <span style="font-size:13px;color:#555;">© PatchHive · patchhive.dev</span>
  <div class="footer-links">
    <a href="#">SignalHive</a><a href="#">ReviewBee</a><a href="#">TrustGate</a><a href="#">RepoMemory</a><a href="#">MergeKeeper</a><a href="#">FlakeSting</a><a href="#">DepTriage</a><a href="#">VulnTriage</a><a href="#">RefactorScout</a><a href="#">RepoReaper</a>
  </div>
  <a href="https://patchhive.dev" class="ph-badge" data-element-id="ph-badge">Powered by PatchHive</a>
</footer>

</body>
</html>
```

## Setup Flow (shared across all product sites)

This wireframe shows the first-run experience — the topic/language picker that appears before any product runs. It is embedded as a section on every product site.

```wireframe
<!DOCTYPE html>
<html>
<head>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; font-family: system-ui, sans-serif; }
  body { background: #0f0f0f; color: #e8e8e8; padding: 48px 40px; }
  .setup-card { max-width: 640px; margin: 0 auto; background: #111; border: 1px solid #222; border-radius: 12px; padding: 40px; }
  .setup-label { font-size: 11px; text-transform: uppercase; letter-spacing: 2px; color: #666; margin-bottom: 8px; }
  .setup-title { font-size: 22px; font-weight: 800; margin-bottom: 6px; }
  .setup-sub { font-size: 14px; color: #666; margin-bottom: 32px; }
  .step-row { display: flex; flex-direction: column; gap: 20px; }
  .step-block { display: flex; flex-direction: column; gap: 8px; }
  .step-header { display: flex; align-items: center; gap: 10px; }
  .step-num { width: 24px; height: 24px; border-radius: 50%; background: #1a1a1a; border: 1px solid #333; display: flex; align-items: center; justify-content: center; font-size: 11px; color: #888; flex-shrink: 0; }
  .step-label { font-size: 13px; font-weight: 600; }
  .token-input { background: #0a0a0a; border: 1px solid #2a2a2a; border-radius: 6px; padding: 10px 14px; font-size: 13px; color: #555; font-family: monospace; width: 100%; display: flex; justify-content: space-between; align-items: center; }
  .token-input span { color: #333; }
  .token-link { font-size: 11px; color: #5b5bd6; cursor: pointer; }
  .topics-row { display: flex; flex-wrap: wrap; gap: 8px; }
  .topic-chip { background: #1a1a1a; border: 1px solid #2a2a2a; border-radius: 20px; padding: 5px 12px; font-size: 12px; color: #888; cursor: pointer; }
  .topic-chip.active { background: #1a1a2e; border-color: #5b5bd6; color: #a0a0ff; }
  .topic-chip.active-rust { background: #1a0800; border-color: #f97316; color: #f97316; }
  .topic-chip.active-sec { background: #1a0000; border-color: #ef4444; color: #ef4444; }
  .topic-chip.active-ts { background: #001a1a; border-color: #06b6d4; color: #06b6d4; }
  .lang-row { display: flex; flex-wrap: wrap; gap: 8px; }
  .lang-chip { background: #1a1a1a; border: 1px solid #2a2a2a; border-radius: 6px; padding: 5px 12px; font-size: 12px; color: #888; cursor: pointer; display: flex; align-items: center; gap: 6px; }
  .lang-chip.active { background: #0d1a0d; border-color: #10b981; color: #10b981; }
  .lang-dot { width: 8px; height: 8px; border-radius: 50%; }
  .divider { border: none; border-top: 1px solid #1a1a1a; margin: 24px 0; }
  .run-btn { width: 100%; background: #5b5bd6; color: #fff; border: none; border-radius: 8px; padding: 14px; font-size: 15px; font-weight: 700; cursor: pointer; margin-top: 8px; }
  .run-note { font-size: 11px; color: #444; text-align: center; margin-top: 10px; }
  .preview-box { background: #0a0a0a; border: 1px solid #1a1a1a; border-radius: 8px; padding: 14px 16px; margin-top: 16px; font-family: monospace; font-size: 12px; }
  .preview-line { color: #555; margin-bottom: 4px; }
  .preview-line span { color: #a0a0ff; }
</style>
</head>
<body>

<div class="setup-card" data-element-id="setup-card">
  <div class="setup-label">Get started</div>
  <div class="setup-title" data-element-id="setup-title">Three steps. Then PatchHive runs.</div>
  <div class="setup-sub" data-element-id="setup-sub">You pick your interests. The agent finds the repos, finds the work, and does it.</div>

  <div class="step-row">

    <div class="step-block" data-element-id="step-token">
      <div class="step-header">
        <div class="step-num">1</div>
        <span class="step-label">Connect GitHub access</span>
        <span class="token-link" data-element-id="token-link">Generate one ↗</span>
      </div>
      <div class="token-input" data-element-id="token-input">
        <span>ghp_••••••••••••••••••••••••••••••••••••</span>
        <span style="color:#5b5bd6;font-size:11px;cursor:pointer;">Paste</span>
      </div>
    </div>

    <hr class="divider">

    <div class="step-block" data-element-id="step-topics">
      <div class="step-header">
        <div class="step-num">2</div>
        <span class="step-label">Pick topics you care about</span>
      </div>
      <div class="topics-row" data-element-id="topics-row">
        <span class="topic-chip active-rust" data-element-id="topic-rust">rust</span>
        <span class="topic-chip active-sec" data-element-id="topic-security">web-security</span>
        <span class="topic-chip active-ts" data-element-id="topic-typescript">typescript</span>
        <span class="topic-chip" data-element-id="topic-cli">cli-tools</span>
        <span class="topic-chip" data-element-id="topic-wasm">webassembly</span>
        <span class="topic-chip" data-element-id="topic-db">databases</span>
        <span class="topic-chip" data-element-id="topic-infra">infrastructure</span>
        <span class="topic-chip" data-element-id="topic-ml">machine-learning</span>
        <span class="topic-chip" data-element-id="topic-add">+ add topic</span>
      </div>
    </div>

    <div class="step-block" data-element-id="step-langs">
      <div class="step-header">
        <div class="step-num" style="opacity:0;"></div>
        <span class="step-label" style="font-size:12px;color:#666;">And languages</span>
      </div>
      <div class="lang-row" data-element-id="lang-row">
        <span class="lang-chip active" data-element-id="lang-rust"><span class="lang-dot" style="background:#f97316;"></span>Rust</span>
        <span class="lang-chip active" data-element-id="lang-ts"><span class="lang-dot" style="background:#06b6d4;"></span>TypeScript</span>
        <span class="lang-chip" data-element-id="lang-go"><span class="lang-dot" style="background:#06b6d4;"></span>Go</span>
        <span class="lang-chip" data-element-id="lang-py"><span class="lang-dot" style="background:#eab308;"></span>Python</span>
        <span class="lang-chip" data-element-id="lang-rb"><span class="lang-dot" style="background:#ef4444;"></span>Ruby</span>
        <span class="lang-chip" data-element-id="lang-add">+ add</span>
      </div>
    </div>

    <hr class="divider">

    <div class="step-block" data-element-id="step-run">
      <div class="step-header">
        <div class="step-num">3</div>
        <span class="step-label">Run</span>
      </div>
      <div class="preview-box" data-element-id="run-preview">
        <div class="preview-line">Searching GitHub for <span>rust · web-security · typescript</span> repos…</div>
        <div class="preview-line">Languages: <span>Rust · TypeScript</span></div>
        <div class="preview-line" style="color:#444;">Found 847 candidate repos · scanning…</div>
      </div>
      <button class="run-btn" data-element-id="run-btn">View GitHub</button>
      <div class="run-note" data-element-id="run-note">PatchHive finds the repos. PatchHive finds the work. You don't pick either.</div>
    </div>

  </div>
</div>

</body>
</html>
```

## ReviewBee (`reviewbee.patchhive.dev`) — accent: amber `#f59e0b`

```wireframe

<html>
<head>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; font-family: system-ui, sans-serif; }
  body { background: #0f0f0f; color: #e8e8e8; }
  nav { display: flex; align-items: center; justify-content: space-between; padding: 16px 40px; border-bottom: 1px solid #222; }
  .logo { font-weight: 700; font-size: 15px; color: #fff; }
  .badge { background: #1a1200; border: 1px solid #333; border-radius: 4px; padding: 2px 8px; font-size: 11px; color: #f59e0b; margin-left: 10px; }
  .cta-btn { background: #f59e0b; color: #0f0f0f; border: none; border-radius: 6px; padding: 8px 18px; font-size: 13px; font-weight: 700; cursor: pointer; }
  .nav-link { font-size: 13px; color: #888; cursor: pointer; margin-right: 16px; }
  .hero { text-align: center; padding: 100px 40px 60px; max-width: 860px; margin: 0 auto; }
  .hero h1 { font-size: 44px; font-weight: 800; line-height: 1.12; margin-bottom: 20px; }
  .hero p { font-size: 18px; color: #aaa; margin-bottom: 32px; }
  .hero-actions { display: flex; gap: 12px; justify-content: center; }
  .secondary { font-size: 13px; color: #666; cursor: pointer; }
  .section { padding: 64px 40px; max-width: 900px; margin: 0 auto; }
  .section-label { font-size: 11px; text-transform: uppercase; letter-spacing: 2px; color: #666; margin-bottom: 12px; }
  h2 { font-size: 26px; font-weight: 700; margin-bottom: 16px; }
  .divider { border: none; border-top: 1px solid #1a1a1a; margin: 0 40px; }
  .bullets { list-style: none; display: flex; flex-direction: column; gap: 10px; margin-top: 16px; }
  .bullets li::before { content: "→ "; color: #f59e0b; }
  .bullets li { font-size: 15px; color: #bbb; }
  .checklist-mockup { background: #111; border: 1px solid #222; border-radius: 8px; padding: 20px; margin-top: 32px; }
  .checklist-mockup .title { font-size: 12px; color: #666; margin-bottom: 12px; font-family: monospace; }
  .check-item { display: flex; gap: 10px; align-items: flex-start; padding: 8px 0; border-bottom: 1px solid #1a1a1a; font-size: 13px; }
  .check-item:last-child { border-bottom: none; }
  .check { width: 14px; height: 14px; border: 1px solid #333; border-radius: 3px; flex-shrink: 0; margin-top: 1px; display: flex; align-items: center; justify-content: center; font-size: 10px; }
  .check.done { background: #f59e0b22; border-color: #f59e0b; color: #f59e0b; }
  .steps { display: flex; flex-direction: column; gap: 16px; margin-top: 20px; }
  .step { display: flex; gap: 16px; }
  .step-num { background: #1a1200; border: 1px solid #333; border-radius: 50%; width: 30px; height: 30px; display: flex; align-items: center; justify-content: center; font-size: 12px; color: #f59e0b; flex-shrink: 0; }
  .step-body h3 { font-size: 14px; font-weight: 600; margin-bottom: 3px; }
  .step-body p { font-size: 13px; color: #888; }
  .cta-banner { background: #111; border: 1px solid #f59e0b33; border-radius: 12px; padding: 48px; text-align: center; margin: 0 40px; }
  .cta-banner h2 { font-size: 24px; margin-bottom: 12px; }
  .trust-line { font-size: 12px; color: #555; margin-top: 12px; }
  footer { border-top: 1px solid #1a1a1a; padding: 32px 40px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 16px; }
  .footer-links { display: flex; gap: 14px; flex-wrap: wrap; }
  .footer-links a { font-size: 12px; color: #555; text-decoration: none; }
  .ph-badge { font-size: 11px; color: #eab308; border: 1px solid #eab30844; border-radius: 4px; padding: 2px 8px; text-decoration: none; }
</style>
</head>
<body>

<nav>
  <div style="display:flex;align-items:center;">
    <span class="logo">PatchHive</span>
    <span class="badge">ReviewBee</span>
  </div>
  <div style="display:flex;align-items:center;">
    <span class="nav-link" data-element-id="nav-products">All Products ▾</span>
    <button class="cta-btn" data-element-id="nav-cta">Find the PRs slowing your team</button>
  </div>
</nav>

<div class="hero">
  <h1 data-element-id="hero-headline">Close PR review threads faster.</h1>
  <p data-element-id="hero-subhead">ReviewBee turns reviewer comments into a concrete checklist so authors know exactly what to fix before merge.</p>
  <div class="hero-actions">
    <button class="cta-btn" data-element-id="hero-cta">Find the PRs slowing your team</button>
    <span class="secondary" data-element-id="hero-secondary">See how it works ↓</span>
  </div>
  <div class="checklist-mockup" data-element-id="hero-mockup">
    <div class="title">ReviewBee · PR #482 · "Add rate limiting middleware"</div>
    <div class="check-item"><div class="check done">✓</div><span style="color:#888;">Add unit tests for the rate limiter logic</span></div>
    <div class="check-item"><div class="check done">✓</div><span style="color:#888;">Extract config values to environment variables</span></div>
    <div class="check-item"><div class="check"></div><span>Handle the 429 response in the client SDK</span></div>
    <div class="check-item"><div class="check"></div><span>Update the API docs with new headers</span></div>
  </div>
</div>

<hr class="divider">

<div class="section" data-element-id="problem">
  <div class="section-label">The Problem</div>
  <h2>Review threads are long. What's actually blocking merge isn't clear.</h2>
  <ul class="bullets">
    <li>Long review threads with mixed feedback, questions, and nitpicks</li>
    <li>Authors reread the whole thread to find what still needs fixing</li>
    <li>Reviewers re-review changes they already approved</li>
    <li>PRs stall because nobody is sure what's left to do</li>
  </ul>
</div>

<hr class="divider">

  <div class="section" data-element-id="how-it-works">
  <div class="section-label">How It Works</div>
  <h2>Token. Topics. Run. ReviewBee does the rest.</h2>
  <div class="steps">
    <div class="step" data-element-id="step-1"><div class="step-num">1</div><div class="step-body"><h3>Connect GitHub access</h3><p>ReviewBee uses only the GitHub access you configure.</p></div></div>
    <div class="step" data-element-id="step-2"><div class="step-num">2</div><div class="step-body"><h3>Pick topics and languages</h3><p>e.g. <code>rust</code>, <code>typescript</code>. ReviewBee searches GitHub for open PRs in repos matching your interests.</p></div></div>
    <div class="step" data-element-id="step-3"><div class="step-num">3</div><div class="step-body"><h3>Run</h3><p>ReviewBee autonomously finds PRs with long review threads and parses them.</p></div></div>
    <div class="step" data-element-id="step-4"><div class="step-num">4</div><div class="step-body"><h3>Cluster and generate checklists</h3><p>Actionable feedback is grouped and turned into a clean, ordered checklist per PR.</p></div></div>
    <div class="step" data-element-id="step-5"><div class="step-num">5</div><div class="step-body"><h3>Mark resolved items as commits land</h3><p>ReviewBee tracks which checklist items are addressed — no manual follow-up needed.</p></div></div>
  </div>
</div>

<div class="cta-banner" data-element-id="cta-banner">
  <h2>Find the PRs slowing your team.</h2>
  <button class="cta-btn" data-element-id="cta-banner-btn">Find the PRs slowing your team</button>
  <div class="trust-line">Focuses on getting existing PRs merged — not writing new code.</div>
</div>

<footer>
  <span style="font-size:13px;color:#555;">© PatchHive · patchhive.dev</span>
  <div class="footer-links">
    <a href="#">SignalHive</a><a href="#">TrustGate</a><a href="#">RepoMemory</a><a href="#">MergeKeeper</a><a href="#">FlakeSting</a><a href="#">DepTriage</a><a href="#">VulnTriage</a><a href="#">RefactorScout</a><a href="#">RepoReaper</a>
  </div>
  <a href="https://patchhive.dev" class="ph-badge" data-element-id="ph-badge">Powered by PatchHive</a>
</footer>

</body>
</html>
```

## TrustGate (`trustgate.patchhive.dev`) — accent: red `#ef4444`

```wireframe

<html>
<head>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; font-family: system-ui, sans-serif; }
  body { background: #0f0f0f; color: #e8e8e8; }
  nav { display: flex; align-items: center; justify-content: space-between; padding: 16px 40px; border-bottom: 1px solid #222; }
  .logo { font-weight: 700; font-size: 15px; color: #fff; }
  .badge { background: #1a0000; border: 1px solid #333; border-radius: 4px; padding: 2px 8px; font-size: 11px; color: #ef4444; margin-left: 10px; }
  .cta-btn { background: #ef4444; color: #fff; border: none; border-radius: 6px; padding: 8px 18px; font-size: 13px; font-weight: 700; cursor: pointer; }
  .nav-link { font-size: 13px; color: #888; cursor: pointer; margin-right: 16px; }
  .hero { text-align: center; padding: 100px 40px 60px; max-width: 860px; margin: 0 auto; }
  .hero h1 { font-size: 40px; font-weight: 800; line-height: 1.12; margin-bottom: 20px; }
  .hero p { font-size: 18px; color: #aaa; margin-bottom: 32px; }
  .hero-actions { display: flex; gap: 12px; justify-content: center; }
  .secondary { font-size: 13px; color: #666; cursor: pointer; }
  .verdict-mockup { display: flex; gap: 12px; justify-content: center; margin-top: 40px; flex-wrap: wrap; }
  .verdict { border-radius: 8px; padding: 16px 24px; font-size: 13px; font-weight: 700; text-align: center; min-width: 120px; }
  .verdict .label { font-size: 10px; text-transform: uppercase; letter-spacing: 1px; margin-bottom: 6px; opacity: 0.7; }
  .verdict .value { font-size: 20px; font-weight: 800; }
  .section { padding: 64px 40px; max-width: 900px; margin: 0 auto; }
  .section-label { font-size: 11px; text-transform: uppercase; letter-spacing: 2px; color: #666; margin-bottom: 12px; }
  h2 { font-size: 26px; font-weight: 700; margin-bottom: 16px; }
  .divider { border: none; border-top: 1px solid #1a1a1a; margin: 0 40px; }
  .bullets { list-style: none; display: flex; flex-direction: column; gap: 10px; margin-top: 16px; }
  .bullets li::before { content: "→ "; color: #ef4444; }
  .bullets li { font-size: 15px; color: #bbb; }
  .steps { display: flex; flex-direction: column; gap: 16px; margin-top: 20px; }
  .step { display: flex; gap: 16px; }
  .step-num { background: #1a0000; border: 1px solid #333; border-radius: 50%; width: 30px; height: 30px; display: flex; align-items: center; justify-content: center; font-size: 12px; color: #ef4444; flex-shrink: 0; }
  .step-body h3 { font-size: 14px; font-weight: 600; margin-bottom: 3px; }
  .step-body p { font-size: 13px; color: #888; }
  .incident-callout { background: #1a0000; border: 1px solid #ef444433; border-radius: 8px; padding: 20px; margin-top: 24px; }
  .incident-callout h3 { font-size: 13px; font-weight: 700; color: #ef4444; margin-bottom: 6px; }
  .incident-callout p { font-size: 13px; color: #888; }
  .cta-banner { background: #111; border: 1px solid #ef444433; border-radius: 12px; padding: 48px; text-align: center; margin: 0 40px; }
  .cta-banner h2 { font-size: 24px; margin-bottom: 12px; }
  .trust-line { font-size: 12px; color: #555; margin-top: 12px; }
  footer { border-top: 1px solid #1a1a1a; padding: 32px 40px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 16px; }
  .footer-links { display: flex; gap: 14px; flex-wrap: wrap; }
  .footer-links a { font-size: 12px; color: #555; text-decoration: none; }
  .ph-badge { font-size: 11px; color: #eab308; border: 1px solid #eab30844; border-radius: 4px; padding: 2px 8px; text-decoration: none; }
</style>
</head>
<body>

<nav>
  <div style="display:flex;align-items:center;">
    <span class="logo">PatchHive</span>
    <span class="badge">TrustGate</span>
  </div>
  <div style="display:flex;align-items:center;">
    <span class="nav-link" data-element-id="nav-products">All Products ▾</span>
    <button class="cta-btn" data-element-id="nav-cta">Review your first AI diff</button>
  </div>
</nav>

<div class="hero">
  <h1 data-element-id="hero-headline">Decide whether AI-generated code changes should be blocked, warned on, or approved.</h1>
  <p data-element-id="hero-subhead">TrustGate reviews AI-generated diffs against your repo's risk rules before they move forward.</p>
  <div class="hero-actions">
    <button class="cta-btn" data-element-id="hero-cta">Review your first AI diff</button>
    <span class="secondary" data-element-id="hero-secondary">See how it works ↓</span>
  </div>
  <div class="verdict-mockup" data-element-id="hero-mockup">
    <div class="verdict" style="background:#0a1a0a;border:1px solid #22c55e44;" data-element-id="verdict-safe"><div class="label" style="color:#22c55e;">Verdict</div><div class="value" style="color:#22c55e;">SAFE</div></div>
    <div class="verdict" style="background:#1a1200;border:1px solid #f59e0b44;" data-element-id="verdict-warn"><div class="label" style="color:#f59e0b;">Verdict</div><div class="value" style="color:#f59e0b;">WARN</div></div>
    <div class="verdict" style="background:#1a0000;border:1px solid #ef444444;" data-element-id="verdict-block"><div class="label" style="color:#ef4444;">Verdict</div><div class="value" style="color:#ef4444;">BLOCK</div></div>
  </div>
</div>

<hr class="divider">

<div class="section" data-element-id="problem">
  <div class="section-label">The Problem</div>
  <h2>AI-generated diffs have no review layer by default.</h2>
  <ul class="bullets">
    <li>Risky file changes with no policy check</li>
    <li>Missing tests on AI-generated logic</li>
    <li>Suspicious patterns that slip past reviewers</li>
    <li>No consistent enforcement of repo-specific rules</li>
  </ul>
</div>

<hr class="divider">

  <div class="section" data-element-id="how-it-works">
  <div class="section-label">How It Works</div>
  <h2>Token. Topics. Run. TrustGate judges every AI diff it finds.</h2>
  <div class="steps">
    <div class="step" data-element-id="step-1"><div class="step-num">1</div><div class="step-body"><h3>Connect GitHub access</h3><p>TrustGate uses only the GitHub access you configure.</p></div></div>
    <div class="step" data-element-id="step-2"><div class="step-num">2</div><div class="step-body"><h3>Pick topics and languages</h3><p>TrustGate searches GitHub for AI-generated PRs in repos matching your interests.</p></div></div>
    <div class="step" data-element-id="step-3"><div class="step-num">3</div><div class="step-body"><h3>Run</h3><p>TrustGate autonomously ingests diffs, applies risk scoring, and checks policy rules.</p></div></div>
    <div class="step" data-element-id="step-4"><div class="step-num">4</div><div class="step-body"><h3>Return safe / warn / block</h3><p>A clear, actionable verdict — not a wall of findings. You never read the diff.</p></div></div>
    <div class="step" data-element-id="step-5"><div class="step-num">5</div><div class="step-body"><h3>Post GitHub status check</h3><p>The verdict appears on the PR. Reviewers see it immediately. TrustGate moves on to the next one.</p></div></div>
  </div>
  <div class="incident-callout" data-element-id="incident-echo">
    <h3>FailGuard capability</h3>
    <p>Past failures, outages, and bad PR outcomes become future policy checks. TrustGate learns from what went wrong and turns it into guardrails.</p>
  </div>
</div>

<div class="cta-banner" data-element-id="cta-banner">
  <h2>Review your first AI diff.</h2>
  <button class="cta-btn" data-element-id="cta-banner-btn">Review your first AI diff</button>
  <div class="trust-line">Complements existing agents. Solves trust, not code generation.</div>
</div>

<footer>
  <span style="font-size:13px;color:#555;">© PatchHive · patchhive.dev</span>
  <div class="footer-links">
    <a href="#">SignalHive</a><a href="#">ReviewBee</a><a href="#">RepoMemory</a><a href="#">MergeKeeper</a><a href="#">FlakeSting</a><a href="#">DepTriage</a><a href="#">VulnTriage</a><a href="#">RefactorScout</a><a href="#">RepoReaper</a>
  </div>
  <a href="https://patchhive.dev" class="ph-badge" data-element-id="ph-badge">Powered by PatchHive</a>
</footer>

</body>
</html>
```

## RepoMemory (`repomemory.patchhive.dev`) — accent: violet `#8b5cf6`

```wireframe

<html>
<head>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; font-family: system-ui, sans-serif; }
  body { background: #0f0f0f; color: #e8e8e8; }
  nav { display: flex; align-items: center; justify-content: space-between; padding: 16px 40px; border-bottom: 1px solid #222; }
  .logo { font-weight: 700; font-size: 15px; color: #fff; }
  .badge { background: #0d0a1a; border: 1px solid #333; border-radius: 4px; padding: 2px 8px; font-size: 11px; color: #8b5cf6; margin-left: 10px; }
  .cta-btn { background: #8b5cf6; color: #fff; border: none; border-radius: 6px; padding: 8px 18px; font-size: 13px; font-weight: 700; cursor: pointer; }
  .nav-link { font-size: 13px; color: #888; cursor: pointer; margin-right: 16px; }
  .hero { text-align: center; padding: 100px 40px 60px; max-width: 860px; margin: 0 auto; }
  .hero h1 { font-size: 44px; font-weight: 800; line-height: 1.12; margin-bottom: 20px; }
  .hero p { font-size: 18px; color: #aaa; margin-bottom: 32px; }
  .hero-actions { display: flex; gap: 12px; justify-content: center; }
  .secondary { font-size: 13px; color: #666; cursor: pointer; }
  .memory-mockup { background: #111; border: 1px solid #222; border-radius: 8px; padding: 20px; margin-top: 32px; text-align: left; }
  .memory-entry { padding: 10px 0; border-bottom: 1px solid #1a1a1a; font-size: 13px; }
  .memory-entry:last-child { border-bottom: none; }
  .mem-tag { font-size: 10px; text-transform: uppercase; letter-spacing: 1px; color: #8b5cf6; margin-bottom: 4px; }
  .mem-text { color: #bbb; }
  .section { padding: 64px 40px; max-width: 900px; margin: 0 auto; }
  .section-label { font-size: 11px; text-transform: uppercase; letter-spacing: 2px; color: #666; margin-bottom: 12px; }
  h2 { font-size: 26px; font-weight: 700; margin-bottom: 16px; }
  .divider { border: none; border-top: 1px solid #1a1a1a; margin: 0 40px; }
  .bullets { list-style: none; display: flex; flex-direction: column; gap: 10px; margin-top: 16px; }
  .bullets li::before { content: "→ "; color: #8b5cf6; }
  .bullets li { font-size: 15px; color: #bbb; }
  .steps { display: flex; flex-direction: column; gap: 16px; margin-top: 20px; }
  .step { display: flex; gap: 16px; }
  .step-num { background: #0d0a1a; border: 1px solid #333; border-radius: 50%; width: 30px; height: 30px; display: flex; align-items: center; justify-content: center; font-size: 12px; color: #8b5cf6; flex-shrink: 0; }
  .step-body h3 { font-size: 14px; font-weight: 600; margin-bottom: 3px; }
  .step-body p { font-size: 13px; color: #888; }
  .incident-callout { background: #0d0a1a; border: 1px solid #8b5cf633; border-radius: 8px; padding: 20px; margin-top: 24px; }
  .incident-callout h3 { font-size: 13px; font-weight: 700; color: #8b5cf6; margin-bottom: 6px; }
  .incident-callout p { font-size: 13px; color: #888; }
  .cta-banner { background: #111; border: 1px solid #8b5cf633; border-radius: 12px; padding: 48px; text-align: center; margin: 0 40px; }
  .cta-banner h2 { font-size: 24px; margin-bottom: 12px; }
  .trust-line { font-size: 12px; color: #555; margin-top: 12px; }
  footer { border-top: 1px solid #1a1a1a; padding: 32px 40px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 16px; }
  .footer-links { display: flex; gap: 14px; flex-wrap: wrap; }
  .footer-links a { font-size: 12px; color: #555; text-decoration: none; }
  .ph-badge { font-size: 11px; color: #eab308; border: 1px solid #eab30844; border-radius: 4px; padding: 2px 8px; text-decoration: none; }
</style>
</head>
<body>

<nav>
  <div style="display:flex;align-items:center;">
    <span class="logo">PatchHive</span>
    <span class="badge">RepoMemory</span>
  </div>
  <div style="display:flex;align-items:center;">
    <span class="nav-link" data-element-id="nav-products">All Products ▾</span>
    <button class="cta-btn" data-element-id="nav-cta">Build your repo's memory</button>
  </div>
</nav>

<div class="hero">
  <h1 data-element-id="hero-headline">Give coding agents memory of how your repo actually works.</h1>
  <p data-element-id="hero-subhead">RepoMemory builds a durable knowledge layer from merged PRs, past bugs, and reviewer feedback so your team stops rediscovering the same rules.</p>
  <div class="hero-actions">
    <button class="cta-btn" data-element-id="hero-cta">Build your repo's memory</button>
    <span class="secondary" data-element-id="hero-secondary">See how it works ↓</span>
  </div>
  <div class="memory-mockup" data-element-id="hero-mockup">
    <div class="memory-entry"><div class="mem-tag">Convention · from 14 merged PRs</div><div class="mem-text">All database migrations must include a rollback step. Reviewers have flagged this 8 times.</div></div>
    <div class="memory-entry"><div class="mem-tag">Past failure · incident 2024-03</div><div class="mem-text">Avoid direct writes to the cache layer from request handlers. Caused the March outage.</div></div>
    <div class="memory-entry"><div class="mem-tag">Reviewer preference · @lead-eng</div><div class="mem-text">Error messages must include the originating service name for traceability.</div></div>
  </div>
</div>

<hr class="divider">

<div class="section" data-element-id="problem">
  <div class="section-label">The Problem</div>
  <h2>Agents repeat the same mistakes. Conventions live nowhere.</h2>
  <ul class="bullets">
    <li>Coding agents ignore architectural rules they've never seen</li>
    <li>Conventions exist in people's heads, not in the repo</li>
    <li>Onboarding takes weeks because context isn't written down</li>
    <li>The same reviewer feedback appears on PR after PR</li>
  </ul>
</div>

<hr class="divider">

  <div class="section" data-element-id="how-it-works">
  <div class="section-label">How It Works</div>
  <h2>Token. Topics. Run. RepoMemory builds the knowledge layer.</h2>
  <div class="steps">
    <div class="step" data-element-id="step-1"><div class="step-num">1</div><div class="step-body"><h3>Connect GitHub access</h3><p>RepoMemory uses only the GitHub access you configure.</p></div></div>
    <div class="step" data-element-id="step-2"><div class="step-num">2</div><div class="step-body"><h3>Pick topics and languages</h3><p>RepoMemory searches GitHub for repos matching your interests and ingests their merged PR history.</p></div></div>
    <div class="step" data-element-id="step-3"><div class="step-num">3</div><div class="step-body"><h3>Run</h3><p>RepoMemory autonomously extracts conventions, past failures, and recurring reviewer feedback.</p></div></div>
    <div class="step" data-element-id="step-4"><div class="step-num">4</div><div class="step-body"><h3>Build a searchable memory store</h3><p>Everything indexed and queryable — by file, topic, contributor. Agents can query it directly.</p></div></div>
  </div>
  <div class="incident-callout" data-element-id="incident-echo">
    <h3>FailGuard capability</h3>
    <p>Captures lessons from bugs, incidents, and painful reviews so the repo keeps institutional memory — and agents don't repeat the mistakes that caused them.</p>
  </div>
</div>

<div class="cta-banner" data-element-id="cta-banner">
  <h2>Build your repo's memory.</h2>
  <button class="cta-btn" data-element-id="cta-banner-btn">Build your repo's memory</button>
  <div class="trust-line">Repo-specific memory. Most agents are shallow on project history.</div>
</div>

<footer>
  <span style="font-size:13px;color:#555;">© PatchHive · patchhive.dev</span>
  <div class="footer-links">
    <a href="#">SignalHive</a><a href="#">ReviewBee</a><a href="#">TrustGate</a><a href="#">MergeKeeper</a><a href="#">FlakeSting</a><a href="#">DepTriage</a><a href="#">VulnTriage</a><a href="#">RefactorScout</a><a href="#">RepoReaper</a>
  </div>
  <a href="https://patchhive.dev" class="ph-badge" data-element-id="ph-badge">Powered by PatchHive</a>
</footer>

</body>
</html>
```

## MergeKeeper (`mergekeeper.patchhive.dev`) — accent: emerald `#10b981`

```wireframe

<html>
<head>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; font-family: system-ui, sans-serif; }
  body { background: #0f0f0f; color: #e8e8e8; }
  nav { display: flex; align-items: center; justify-content: space-between; padding: 16px 40px; border-bottom: 1px solid #222; }
  .logo { font-weight: 700; font-size: 15px; color: #fff; }
  .badge { background: #001a0f; border: 1px solid #333; border-radius: 4px; padding: 2px 8px; font-size: 11px; color: #10b981; margin-left: 10px; }
  .cta-btn { background: #10b981; color: #0f0f0f; border: none; border-radius: 6px; padding: 8px 18px; font-size: 13px; font-weight: 700; cursor: pointer; }
  .nav-link { font-size: 13px; color: #888; cursor: pointer; margin-right: 16px; }
  .hero { text-align: center; padding: 100px 40px 60px; max-width: 860px; margin: 0 auto; }
  .hero h1 { font-size: 48px; font-weight: 800; line-height: 1.12; margin-bottom: 20px; }
  .hero p { font-size: 18px; color: #aaa; margin-bottom: 32px; }
  .hero-actions { display: flex; gap: 12px; justify-content: center; }
  .secondary { font-size: 13px; color: #666; cursor: pointer; }
  .pr-list { background: #111; border: 1px solid #222; border-radius: 8px; padding: 16px; margin-top: 32px; text-align: left; }
  .pr-item { display: flex; justify-content: space-between; align-items: center; padding: 10px 0; border-bottom: 1px solid #1a1a1a; font-size: 13px; }
  .pr-item:last-child { border-bottom: none; }
  .pr-name { color: #bbb; }
  .pr-status { font-size: 11px; font-weight: 700; padding: 2px 8px; border-radius: 4px; }
  .section { padding: 64px 40px; max-width: 900px; margin: 0 auto; }
  .section-label { font-size: 11px; text-transform: uppercase; letter-spacing: 2px; color: #666; margin-bottom: 12px; }
  h2 { font-size: 26px; font-weight: 700; margin-bottom: 16px; }
  .divider { border: none; border-top: 1px solid #1a1a1a; margin: 0 40px; }
  .bullets { list-style: none; display: flex; flex-direction: column; gap: 10px; margin-top: 16px; }
  .bullets li::before { content: "→ "; color: #10b981; }
  .bullets li { font-size: 15px; color: #bbb; }
  .steps { display: flex; flex-direction: column; gap: 16px; margin-top: 20px; }
  .step { display: flex; gap: 16px; }
  .step-num { background: #001a0f; border: 1px solid #333; border-radius: 50%; width: 30px; height: 30px; display: flex; align-items: center; justify-content: center; font-size: 12px; color: #10b981; flex-shrink: 0; }
  .step-body h3 { font-size: 14px; font-weight: 600; margin-bottom: 3px; }
  .step-body p { font-size: 13px; color: #888; }
  .cta-banner { background: #111; border: 1px solid #10b98133; border-radius: 12px; padding: 48px; text-align: center; margin: 0 40px; }
  .cta-banner h2 { font-size: 24px; margin-bottom: 12px; }
  .trust-line { font-size: 12px; color: #555; margin-top: 12px; }
  footer { border-top: 1px solid #1a1a1a; padding: 32px 40px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 16px; }
  .footer-links { display: flex; gap: 14px; flex-wrap: wrap; }
  .footer-links a { font-size: 12px; color: #555; text-decoration: none; }
  .ph-badge { font-size: 11px; color: #eab308; border: 1px solid #eab30844; border-radius: 4px; padding: 2px 8px; text-decoration: none; }
</style>
</head>
<body>

<nav>
  <div style="display:flex;align-items:center;">
    <span class="logo">PatchHive</span>
    <span class="badge">MergeKeeper</span>
  </div>
  <div style="display:flex;align-items:center;">
    <span class="nav-link" data-element-id="nav-products">All Products ▾</span>
    <button class="cta-btn" data-element-id="nav-cta">Catch your first stale PR</button>
  </div>
</nav>

<div class="hero">
  <h1 data-element-id="hero-headline">Keep pull requests mergeable.</h1>
  <p data-element-id="hero-subhead">MergeKeeper watches open PRs and tells your team which ones are drifting toward merge trouble before it's too late.</p>
  <div class="hero-actions">
    <button class="cta-btn" data-element-id="hero-cta">Catch your first stale PR</button>
    <span class="secondary" data-element-id="hero-secondary">See how it works ↓</span>
  </div>
  <div class="pr-list" data-element-id="hero-mockup">
    <div class="pr-item"><span class="pr-name">feat/rate-limiting · 14 days old</span><span class="pr-status" style="background:#f59e0b22;color:#f59e0b;">STALE</span></div>
    <div class="pr-item"><span class="pr-name">fix/auth-timeout · CI failing</span><span class="pr-status" style="background:#ef444422;color:#ef4444;">FAILING CI</span></div>
    <div class="pr-item"><span class="pr-name">refactor/db-layer · needs rebase</span><span class="pr-status" style="background:#8b5cf622;color:#8b5cf6;">NEEDS REBASE</span></div>
    <div class="pr-item"><span class="pr-name">docs/api-update · ready to merge</span><span class="pr-status" style="background:#10b98122;color:#10b981;">CLEAN</span></div>
  </div>
</div>

<hr class="divider">

<div class="section" data-element-id="problem">
  <div class="section-label">The Problem</div>
  <h2>PRs drift. Nobody notices until it's a merge emergency.</h2>
  <ul class="bullets">
    <li>Stale branches that diverged weeks ago</li>
    <li>Failing CI checks nobody reruns</li>
    <li>Merge conflicts discovered at review time, not before</li>
    <li>Long-lived PRs that become impossible to merge cleanly</li>
  </ul>
</div>

<hr class="divider">

  <div class="section" data-element-id="how-it-works">
  <div class="section-label">How It Works</div>
  <h2>Token. Topics. Run. MergeKeeper watches every PR it finds.</h2>
  <div class="steps">
    <div class="step" data-element-id="step-1"><div class="step-num">1</div><div class="step-body"><h3>Connect GitHub access</h3><p>MergeKeeper uses only the GitHub access you configure.</p></div></div>
    <div class="step" data-element-id="step-2"><div class="step-num">2</div><div class="step-body"><h3>Pick topics and languages</h3><p>MergeKeeper searches GitHub for repos matching your interests and monitors their open PRs.</p></div></div>
    <div class="step" data-element-id="step-3"><div class="step-num">3</div><div class="step-body"><h3>Run</h3><p>MergeKeeper autonomously detects stale branches, failing CI, pending rebases, and merge conflicts.</p></div></div>
    <div class="step" data-element-id="step-4"><div class="step-num">4</div><div class="step-body"><h3>Post status directly on the PR</h3><p>Authors and reviewers see the condition and suggested action — no dashboard to check.</p></div></div>
  </div>
</div>

<div class="cta-banner" data-element-id="cta-banner">
  <h2>Catch your first stale PR.</h2>
  <button class="cta-btn" data-element-id="cta-banner-btn">Catch your first stale PR</button>
  <div class="trust-line">PR maintenance operations — painful, measurable, less crowded than coding agents.</div>
</div>

<footer>
  <span style="font-size:13px;color:#555;">© PatchHive · patchhive.dev</span>
  <div class="footer-links">
    <a href="#">SignalHive</a><a href="#">ReviewBee</a><a href="#">TrustGate</a><a href="#">RepoMemory</a><a href="#">FlakeSting</a><a href="#">DepTriage</a><a href="#">VulnTriage</a><a href="#">RefactorScout</a><a href="#">RepoReaper</a>
  </div>
  <a href="https://patchhive.dev" class="ph-badge" data-element-id="ph-badge">Powered by PatchHive</a>
</footer>

</body>
</html>
```

## FlakeSting (`flakesting.patchhive.dev`) — accent: orange `#f97316`

```wireframe

<html>
<head>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; font-family: system-ui, sans-serif; }
  body { background: #0f0f0f; color: #e8e8e8; }
  nav { display: flex; align-items: center; justify-content: space-between; padding: 16px 40px; border-bottom: 1px solid #222; }
  .logo { font-weight: 700; font-size: 15px; color: #fff; }
  .badge { background: #1a0800; border: 1px solid #333; border-radius: 4px; padding: 2px 8px; font-size: 11px; color: #f97316; margin-left: 10px; }
  .cta-btn { background: #f97316; color: #0f0f0f; border: none; border-radius: 6px; padding: 8px 18px; font-size: 13px; font-weight: 700; cursor: pointer; }
  .nav-link { font-size: 13px; color: #888; cursor: pointer; margin-right: 16px; }
  .hero { text-align: center; padding: 100px 40px 60px; max-width: 860px; margin: 0 auto; }
  .hero h1 { font-size: 38px; font-weight: 800; line-height: 1.12; margin-bottom: 20px; }
  .hero p { font-size: 18px; color: #aaa; margin-bottom: 32px; }
  .hero-actions { display: flex; gap: 12px; justify-content: center; }
  .secondary { font-size: 13px; color: #666; cursor: pointer; }
  .flake-mockup { background: #111; border: 1px solid #222; border-radius: 8px; padding: 16px; margin-top: 32px; text-align: left; font-family: monospace; font-size: 12px; }
  .flake-row { display: flex; justify-content: space-between; align-items: center; padding: 8px 0; border-bottom: 1px solid #1a1a1a; }
  .flake-row:last-child { border-bottom: none; }
  .flake-name { color: #bbb; }
  .flake-rate { font-weight: 700; }
  .section { padding: 64px 40px; max-width: 900px; margin: 0 auto; }
  .section-label { font-size: 11px; text-transform: uppercase; letter-spacing: 2px; color: #666; margin-bottom: 12px; }
  h2 { font-size: 26px; font-weight: 700; margin-bottom: 16px; }
  .divider { border: none; border-top: 1px solid #1a1a1a; margin: 0 40px; }
  .bullets { list-style: none; display: flex; flex-direction: column; gap: 10px; margin-top: 16px; }
  .bullets li::before { content: "→ "; color: #f97316; }
  .bullets li { font-size: 15px; color: #bbb; }
  .steps { display: flex; flex-direction: column; gap: 16px; margin-top: 20px; }
  .step { display: flex; gap: 16px; }
  .step-num { background: #1a0800; border: 1px solid #333; border-radius: 50%; width: 30px; height: 30px; display: flex; align-items: center; justify-content: center; font-size: 12px; color: #f97316; flex-shrink: 0; }
  .step-body h3 { font-size: 14px; font-weight: 600; margin-bottom: 3px; }
  .step-body p { font-size: 13px; color: #888; }
  .cta-banner { background: #111; border: 1px solid #f9731633; border-radius: 12px; padding: 48px; text-align: center; margin: 0 40px; }
  .cta-banner h2 { font-size: 24px; margin-bottom: 12px; }
  .trust-line { font-size: 12px; color: #555; margin-top: 12px; }
  footer { border-top: 1px solid #1a1a1a; padding: 32px 40px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 16px; }
  .footer-links { display: flex; gap: 14px; flex-wrap: wrap; }
  .footer-links a { font-size: 12px; color: #555; text-decoration: none; }
  .ph-badge { font-size: 11px; color: #eab308; border: 1px solid #eab30844; border-radius: 4px; padding: 2px 8px; text-decoration: none; }
</style>
</head>
<body>

<nav>
  <div style="display:flex;align-items:center;">
    <span class="logo">PatchHive</span>
    <span class="badge">FlakeSting</span>
  </div>
  <div style="display:flex;align-items:center;">
    <span class="nav-link" data-element-id="nav-products">All Products ▾</span>
    <button class="cta-btn" data-element-id="nav-cta">Find your flaky tests</button>
  </div>
</nav>

<div class="hero">
  <h1 data-element-id="hero-headline">Detect, isolate, and explain flaky tests before they destroy trust in CI.</h1>
  <p data-element-id="hero-subhead">FlakeSting analyzes test runs over time to separate real regressions from unreliable checks.</p>
  <div class="hero-actions">
    <button class="cta-btn" data-element-id="hero-cta">Find your flaky tests</button>
    <span class="secondary" data-element-id="hero-secondary">See how it works ↓</span>
  </div>
  <div class="flake-mockup" data-element-id="hero-mockup">
    <div style="color:#666;margin-bottom:8px;">FlakeSting · last 30 days · 847 runs analyzed</div>
    <div class="flake-row"><span class="flake-name">test_user_session_timeout</span><span class="flake-rate" style="color:#ef4444;">38% flake rate · SUSPECT</span></div>
    <div class="flake-row"><span class="flake-name">test_payment_webhook_retry</span><span class="flake-rate" style="color:#f97316;">21% flake rate · WATCH</span></div>
    <div class="flake-row"><span class="flake-name">test_cache_invalidation_race</span><span class="flake-rate" style="color:#f97316;">17% flake rate · WATCH</span></div>
    <div class="flake-row"><span class="flake-name">test_api_rate_limit_headers</span><span class="flake-rate" style="color:#10b981;">2% flake rate · STABLE</span></div>
  </div>
</div>

<hr class="divider">

<div class="section" data-element-id="problem">
  <div class="section-label">The Problem</div>
  <h2>Flaky tests make CI untrustworthy. Real failures hide in the noise.</h2>
  <ul class="bullets">
    <li>Tests that fail, then pass on rerun — with no explanation</li>
    <li>Engineers rerunnning CI instead of investigating</li>
    <li>Real regressions dismissed as "probably flaky"</li>
    <li>Team stops trusting CI results entirely</li>
  </ul>
</div>

<hr class="divider">

  <div class="section" data-element-id="how-it-works">
  <div class="section-label">How It Works</div>
  <h2>Token. Topics. Run. FlakeSting hunts flaky tests across the ecosystem.</h2>
  <div class="steps">
    <div class="step" data-element-id="step-1"><div class="step-num">1</div><div class="step-body"><h3>Connect GitHub access</h3><p>FlakeSting uses only the GitHub access you configure.</p></div></div>
    <div class="step" data-element-id="step-2"><div class="step-num">2</div><div class="step-body"><h3>Pick topics and languages</h3><p>FlakeSting searches GitHub for repos matching your interests and ingests their CI run history.</p></div></div>
    <div class="step" data-element-id="step-3"><div class="step-num">3</div><div class="step-body"><h3>Run</h3><p>FlakeSting autonomously identifies tests that fail inconsistently across runs with no code change.</p></div></div>
    <div class="step" data-element-id="step-4"><div class="step-num">4</div><div class="step-body"><h3>Emit suspect / quarantine report</h3><p>Ranked by flake rate and evidence — a clear list of what to investigate, quarantine, or fix.</p></div></div>
  </div>
</div>

<div class="cta-banner" data-element-id="cta-banner">
  <h2>Find your flaky tests.</h2>
  <button class="cta-btn" data-element-id="cta-banner-btn">Find your flaky tests</button>
  <div class="trust-line">Flaky test pain is constant and expensive. Poorly handled by general-purpose tools.</div>
</div>

<footer>
  <span style="font-size:13px;color:#555;">© PatchHive · patchhive.dev</span>
  <div class="footer-links">
    <a href="#">SignalHive</a><a href="#">ReviewBee</a><a href="#">TrustGate</a><a href="#">RepoMemory</a><a href="#">MergeKeeper</a><a href="#">DepTriage</a><a href="#">VulnTriage</a><a href="#">RefactorScout</a><a href="#">RepoReaper</a>
  </div>
  <a href="https://patchhive.dev" class="ph-badge" data-element-id="ph-badge">Powered by PatchHive</a>
</footer>

</body>
</html>
```

## DepTriage (`deptriage.patchhive.dev`) — accent: cyan `#06b6d4`

```wireframe

<html>
<head>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; font-family: system-ui, sans-serif; }
  body { background: #0f0f0f; color: #e8e8e8; }
  nav { display: flex; align-items: center; justify-content: space-between; padding: 16px 40px; border-bottom: 1px solid #222; }
  .logo { font-weight: 700; font-size: 15px; color: #fff; }
  .badge { background: #001a1a; border: 1px solid #333; border-radius: 4px; padding: 2px 8px; font-size: 11px; color: #06b6d4; margin-left: 10px; }
  .cta-btn { background: #06b6d4; color: #0f0f0f; border: none; border-radius: 6px; padding: 8px 18px; font-size: 13px; font-weight: 700; cursor: pointer; }
  .nav-link { font-size: 13px; color: #888; cursor: pointer; margin-right: 16px; }
  .hero { text-align: center; padding: 100px 40px 60px; max-width: 860px; margin: 0 auto; }
  .hero h1 { font-size: 40px; font-weight: 800; line-height: 1.12; margin-bottom: 20px; }
  .hero p { font-size: 18px; color: #aaa; margin-bottom: 32px; }
  .hero-actions { display: flex; gap: 12px; justify-content: center; }
  .secondary { font-size: 13px; color: #666; cursor: pointer; }
  .dep-list { background: #111; border: 1px solid #222; border-radius: 8px; padding: 16px; margin-top: 32px; text-align: left; }
  .dep-item { display: flex; justify-content: space-between; align-items: center; padding: 10px 0; border-bottom: 1px solid #1a1a1a; font-size: 13px; }
  .dep-item:last-child { border-bottom: none; }
  .dep-name { color: #bbb; font-family: monospace; }
  .dep-badge { font-size: 11px; font-weight: 700; padding: 2px 8px; border-radius: 4px; }
  .section { padding: 64px 40px; max-width: 900px; margin: 0 auto; }
  .section-label { font-size: 11px; text-transform: uppercase; letter-spacing: 2px; color: #666; margin-bottom: 12px; }
  h2 { font-size: 26px; font-weight: 700; margin-bottom: 16px; }
  .divider { border: none; border-top: 1px solid #1a1a1a; margin: 0 40px; }
  .bullets { list-style: none; display: flex; flex-direction: column; gap: 10px; margin-top: 16px; }
  .bullets li::before { content: "→ "; color: #06b6d4; }
  .bullets li { font-size: 15px; color: #bbb; }
  .steps { display: flex; flex-direction: column; gap: 16px; margin-top: 20px; }
  .step { display: flex; gap: 16px; }
  .step-num { background: #001a1a; border: 1px solid #333; border-radius: 50%; width: 30px; height: 30px; display: flex; align-items: center; justify-content: center; font-size: 12px; color: #06b6d4; flex-shrink: 0; }
  .step-body h3 { font-size: 14px; font-weight: 600; margin-bottom: 3px; }
  .step-body p { font-size: 13px; color: #888; }
  .cta-banner { background: #111; border: 1px solid #06b6d433; border-radius: 12px; padding: 48px; text-align: center; margin: 0 40px; }
  .cta-banner h2 { font-size: 24px; margin-bottom: 12px; }
  .trust-line { font-size: 12px; color: #555; margin-top: 12px; }
  footer { border-top: 1px solid #1a1a1a; padding: 32px 40px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 16px; }
  .footer-links { display: flex; gap: 14px; flex-wrap: wrap; }
  .footer-links a { font-size: 12px; color: #555; text-decoration: none; }
  .ph-badge { font-size: 11px; color: #eab308; border: 1px solid #eab30844; border-radius: 4px; padding: 2px 8px; text-decoration: none; }
</style>
</head>
<body>

<nav>
  <div style="display:flex;align-items:center;">
    <span class="logo">PatchHive</span>
    <span class="badge">DepTriage</span>
  </div>
  <div style="display:flex;align-items:center;">
    <span class="nav-link" data-element-id="nav-products">All Products ▾</span>
    <button class="cta-btn" data-element-id="nav-cta">Triage your dependencies</button>
  </div>
</nav>

<div class="hero">
  <h1 data-element-id="hero-headline">Tell your team which dependency updates matter now and which ones can wait.</h1>
  <p data-element-id="hero-subhead">DepTriage sorts the flood of Dependabot and Renovate PRs into something your team can actually act on.</p>
  <div class="hero-actions">
    <button class="cta-btn" data-element-id="hero-cta">Triage your dependencies</button>
    <span class="secondary" data-element-id="hero-secondary">See how it works ↓</span>
  </div>
  <div class="dep-list" data-element-id="hero-mockup">
    <div class="dep-item"><span class="dep-name">openssl 3.0.7 → 3.0.9</span><span class="dep-badge" style="background:#ef444422;color:#ef4444;">ACT NOW · CVE-2023-0286</span></div>
    <div class="dep-item"><span class="dep-name">express 4.18.1 → 4.18.2</span><span class="dep-badge" style="background:#f59e0b22;color:#f59e0b;">THIS WEEK · minor security</span></div>
    <div class="dep-item"><span class="dep-name">typescript 5.0.4 → 5.1.0</span><span class="dep-badge" style="background:#06b6d422;color:#06b6d4;">NEXT SPRINT · low risk</span></div>
    <div class="dep-item"><span class="dep-name">eslint-plugin-react 7.32 → 7.33</span><span class="dep-badge" style="background:#33333344;color:#666;">SKIP · cosmetic only</span></div>
  </div>
</div>

<hr class="divider">

<div class="section" data-element-id="problem">
  <div class="section-label">The Problem</div>
  <h2>Dependabot opens 40 PRs. Nobody knows which ones matter.</h2>
  <ul class="bullets">
    <li>Update bots generate noise faster than teams can review</li>
    <li>Security advisories buried in routine version bumps</li>
    <li>Compatibility risk unknown without manual investigation</li>
    <li>Teams ignore the queue entirely — including the critical ones</li>
  </ul>
</div>

<hr class="divider">

  <div class="section" data-element-id="how-it-works">
  <div class="section-label">How It Works</div>
  <h2>Token. Topics. Run. DepTriage ranks every dependency update it finds.</h2>
  <div class="steps">
    <div class="step" data-element-id="step-1"><div class="step-num">1</div><div class="step-body"><h3>Connect GitHub access</h3><p>DepTriage uses only the GitHub access you configure.</p></div></div>
    <div class="step" data-element-id="step-2"><div class="step-num">2</div><div class="step-body"><h3>Pick topics and languages</h3><p>DepTriage searches GitHub for repos matching your interests and ingests their Dependabot and Renovate PRs.</p></div></div>
    <div class="step" data-element-id="step-3"><div class="step-num">3</div><div class="step-body"><h3>Run</h3><p>DepTriage autonomously scores each update by urgency, compatibility risk, and likely value — then collapses duplicates.</p></div></div>
    <div class="step" data-element-id="step-4"><div class="step-num">4</div><div class="step-body"><h3>Weekly triage report</h3><p>Act now / this week / next sprint / skip. A clear queue across repos inside your allowed scope.</p></div></div>
  </div>
</div>

<div class="cta-banner" data-element-id="cta-banner">
  <h2>Triage your dependencies.</h2>
  <button class="cta-btn" data-element-id="cta-banner-btn">Triage your dependencies</button>
  <div class="trust-line">Filters and prioritizes update bots — doesn't replace them.</div>
</div>

<footer>
  <span style="font-size:13px;color:#555;">© PatchHive · patchhive.dev</span>
  <div class="footer-links">
    <a href="#">SignalHive</a><a href="#">ReviewBee</a><a href="#">TrustGate</a><a href="#">RepoMemory</a><a href="#">MergeKeeper</a><a href="#">FlakeSting</a><a href="#">VulnTriage</a><a href="#">RefactorScout</a><a href="#">RepoReaper</a>
  </div>
  <a href="https://patchhive.dev" class="ph-badge" data-element-id="ph-badge">Powered by PatchHive</a>
</footer>

</body>
</html>
```

## VulnTriage (`vulntriage.patchhive.dev`) — accent: rose-red `#dc2626`

```wireframe

<html>
<head>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; font-family: system-ui, sans-serif; }
  body { background: #0f0f0f; color: #e8e8e8; }
  nav { display: flex; align-items: center; justify-content: space-between; padding: 16px 40px; border-bottom: 1px solid #222; }
  .logo { font-weight: 700; font-size: 15px; color: #fff; }
  .badge { background: #1a0000; border: 1px solid #333; border-radius: 4px; padding: 2px 8px; font-size: 11px; color: #dc2626; margin-left: 10px; }
  .cta-btn { background: #dc2626; color: #fff; border: none; border-radius: 6px; padding: 8px 18px; font-size: 13px; font-weight: 700; cursor: pointer; }
  .nav-link { font-size: 13px; color: #888; cursor: pointer; margin-right: 16px; }
  .hero { text-align: center; padding: 100px 40px 60px; max-width: 860px; margin: 0 auto; }
  .hero h1 { font-size: 42px; font-weight: 800; line-height: 1.12; margin-bottom: 20px; }
  .hero p { font-size: 18px; color: #aaa; margin-bottom: 32px; }
  .hero-actions { display: flex; gap: 12px; justify-content: center; }
  .secondary { font-size: 13px; color: #666; cursor: pointer; }
  .vuln-list { background: #111; border: 1px solid #222; border-radius: 8px; padding: 16px; margin-top: 32px; text-align: left; }
  .vuln-item { padding: 10px 0; border-bottom: 1px solid #1a1a1a; }
  .vuln-item:last-child { border-bottom: none; }
  .vuln-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px; }
  .vuln-name { font-size: 13px; color: #bbb; font-family: monospace; }
  .vuln-score { font-size: 11px; font-weight: 700; padding: 2px 8px; border-radius: 4px; }
  .vuln-meta { font-size: 12px; color: #555; }
  .section { padding: 64px 40px; max-width: 900px; margin: 0 auto; }
  .section-label { font-size: 11px; text-transform: uppercase; letter-spacing: 2px; color: #666; margin-bottom: 12px; }
  h2 { font-size: 26px; font-weight: 700; margin-bottom: 16px; }
  .divider { border: none; border-top: 1px solid #1a1a1a; margin: 0 40px; }
  .bullets { list-style: none; display: flex; flex-direction: column; gap: 10px; margin-top: 16px; }
  .bullets li::before { content: "→ "; color: #dc2626; }
  .bullets li { font-size: 15px; color: #bbb; }
  .steps { display: flex; flex-direction: column; gap: 16px; margin-top: 20px; }
  .step { display: flex; gap: 16px; }
  .step-num { background: #1a0000; border: 1px solid #333; border-radius: 50%; width: 30px; height: 30px; display: flex; align-items: center; justify-content: center; font-size: 12px; color: #dc2626; flex-shrink: 0; }
  .step-body h3 { font-size: 14px; font-weight: 600; margin-bottom: 3px; }
  .step-body p { font-size: 13px; color: #888; }
  .cta-banner { background: #111; border: 1px solid #dc262633; border-radius: 12px; padding: 48px; text-align: center; margin: 0 40px; }
  .cta-banner h2 { font-size: 24px; margin-bottom: 12px; }
  .trust-line { font-size: 12px; color: #555; margin-top: 12px; }
  footer { border-top: 1px solid #1a1a1a; padding: 32px 40px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 16px; }
  .footer-links { display: flex; gap: 14px; flex-wrap: wrap; }
  .footer-links a { font-size: 12px; color: #555; text-decoration: none; }
  .ph-badge { font-size: 11px; color: #eab308; border: 1px solid #eab30844; border-radius: 4px; padding: 2px 8px; text-decoration: none; }
</style>
</head>
<body>

<nav>
  <div style="display:flex;align-items:center;">
    <span class="logo">PatchHive</span>
    <span class="badge">VulnTriage</span>
  </div>
  <div style="display:flex;align-items:center;">
    <span class="nav-link" data-element-id="nav-products">All Products ▾</span>
    <button class="cta-btn" data-element-id="nav-cta">Rank your security findings</button>
  </div>
</nav>

<div class="hero">
  <h1 data-element-id="hero-headline">Turn security findings into ranked, actionable engineering work.</h1>
  <p data-element-id="hero-subhead">VulnTriage ingests security alerts and tells your team what matters most, where it lives, and what to do next.</p>
  <div class="hero-actions">
    <button class="cta-btn" data-element-id="hero-cta">Rank your security findings</button>
    <span class="secondary" data-element-id="hero-secondary">See how it works ↓</span>
  </div>
  <div class="vuln-list" data-element-id="hero-mockup">
    <div class="vuln-item"><div class="vuln-header"><span class="vuln-name">CVE-2023-44487 · http2-server</span><span class="vuln-score" style="background:#dc262622;color:#dc2626;">CRITICAL · fix now</span></div><div class="vuln-meta">Owner: @backend-team · src/server/http.rs · reachable from public endpoint</div></div>
    <div class="vuln-item"><div class="vuln-header"><span class="vuln-name">CVE-2023-38545 · libcurl</span><span class="vuln-score" style="background:#f59e0b22;color:#f59e0b;">HIGH · this sprint</span></div><div class="vuln-meta">Owner: @infra · deps/curl · not directly reachable</div></div>
    <div class="vuln-item"><div class="vuln-header"><span class="vuln-name">CVE-2023-29491 · ncurses</span><span class="vuln-score" style="background:#33333344;color:#666;">LOW · backlog</span></div><div class="vuln-meta">Owner: @tooling · dev dependency only</div></div>
  </div>
</div>

<hr class="divider">

<div class="section" data-element-id="problem">
  <div class="section-label">The Problem</div>
  <h2>Security tools find everything. Nobody knows what to fix first.</h2>
  <ul class="bullets">
    <li>Hundreds of raw alerts with no priority signal</li>
    <li>Unclear which findings are actually reachable in production</li>
    <li>No routing to the right owner or team</li>
    <li>Security work deprioritized because it's overwhelming</li>
  </ul>
</div>

<hr class="divider">

<div class="section" data-element-id="how-it-works">
  <div class="section-label">How It Works</div>
  <h2>Token. Topics. Run. VulnTriage ranks every security finding it finds.</h2>
  <div class="steps">
    <div class="step" data-element-id="step-1"><div class="step-num">1</div><div class="step-body"><h3>Connect GitHub access</h3><p>VulnTriage uses only the GitHub access you configure.</p></div></div>
    <div class="step" data-element-id="step-2"><div class="step-num">2</div><div class="step-body"><h3>Pick topics and languages</h3><p>VulnTriage searches GitHub for repos matching your interests and ingests their security alerts.</p></div></div>
    <div class="step" data-element-id="step-3"><div class="step-num">3</div><div class="step-body"><h3>Run</h3><p>VulnTriage autonomously maps findings to files and owners, then ranks by exploitability and reachability.</p></div></div>
    <div class="step" data-element-id="step-4"><div class="step-num">4</div><div class="step-body"><h3>Ranked engineering tasks, not raw CVEs</h3><p>A clear queue across repos inside your allowed scope — what matters most, where it lives, what to do next.</p></div></div>
  </div>
</div>

<div class="cta-banner" data-element-id="cta-banner">
  <h2>Rank your security findings.</h2>
  <button class="cta-btn" data-element-id="cta-banner-btn">Rank your security findings</button>
  <div class="trust-line">Explains what matters most in repo context. Routes it into normal engineering work.</div>
</div>

<footer>
  <span style="font-size:13px;color:#555;">© PatchHive · patchhive.dev</span>
  <div class="footer-links">
    <a href="#">SignalHive</a><a href="#">ReviewBee</a><a href="#">TrustGate</a><a href="#">RepoMemory</a><a href="#">MergeKeeper</a><a href="#">FlakeSting</a><a href="#">DepTriage</a><a href="#">RefactorScout</a><a href="#">RepoReaper</a>
  </div>
  <a href="https://patchhive.dev" class="ph-badge" data-element-id="ph-badge">Powered by PatchHive</a>
</footer>

</body>
</html>
```

## RefactorScout (`refactorscout.patchhive.dev`) — accent: lime `#84cc16`

```wireframe

<html>
<head>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; font-family: system-ui, sans-serif; }
  body { background: #0f0f0f; color: #e8e8e8; }
  nav { display: flex; align-items: center; justify-content: space-between; padding: 16px 40px; border-bottom: 1px solid #222; }
  .logo { font-weight: 700; font-size: 15px; color: #fff; }
  .badge { background: #0a1400; border: 1px solid #333; border-radius: 4px; padding: 2px 8px; font-size: 11px; color: #84cc16; margin-left: 10px; }
  .cta-btn { background: #84cc16; color: #0f0f0f; border: none; border-radius: 6px; padding: 8px 18px; font-size: 13px; font-weight: 700; cursor: pointer; }
  .nav-link { font-size: 13px; color: #888; cursor: pointer; margin-right: 16px; }
  .hero { text-align: center; padding: 100px 40px 60px; max-width: 860px; margin: 0 auto; }
  .hero h1 { font-size: 40px; font-weight: 800; line-height: 1.12; margin-bottom: 20px; }
  .hero p { font-size: 18px; color: #aaa; margin-bottom: 32px; }
  .hero-actions { display: flex; gap: 12px; justify-content: center; }
  .secondary { font-size: 13px; color: #666; cursor: pointer; }
  .refactor-list { background: #111; border: 1px solid #222; border-radius: 8px; padding: 16px; margin-top: 32px; text-align: left; }
  .refactor-item { padding: 10px 0; border-bottom: 1px solid #1a1a1a; }
  .refactor-item:last-child { border-bottom: none; }
  .refactor-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px; }
  .refactor-name { font-size: 13px; color: #bbb; font-family: monospace; }
  .refactor-tag { font-size: 11px; font-weight: 700; padding: 2px 8px; border-radius: 4px; }
  .refactor-meta { font-size: 12px; color: #555; }
  .section { padding: 64px 40px; max-width: 900px; margin: 0 auto; }
  .section-label { font-size: 11px; text-transform: uppercase; letter-spacing: 2px; color: #666; margin-bottom: 12px; }
  h2 { font-size: 26px; font-weight: 700; margin-bottom: 16px; }
  .divider { border: none; border-top: 1px solid #1a1a1a; margin: 0 40px; }
  .bullets { list-style: none; display: flex; flex-direction: column; gap: 10px; margin-top: 16px; }
  .bullets li::before { content: "→ "; color: #84cc16; }
  .bullets li { font-size: 15px; color: #bbb; }
  .steps { display: flex; flex-direction: column; gap: 16px; margin-top: 20px; }
  .step { display: flex; gap: 16px; }
  .step-num { background: #0a1400; border: 1px solid #333; border-radius: 50%; width: 30px; height: 30px; display: flex; align-items: center; justify-content: center; font-size: 12px; color: #84cc16; flex-shrink: 0; }
  .step-body h3 { font-size: 14px; font-weight: 600; margin-bottom: 3px; }
  .step-body p { font-size: 13px; color: #888; }
  .cta-banner { background: #111; border: 1px solid #84cc1633; border-radius: 12px; padding: 48px; text-align: center; margin: 0 40px; }
  .cta-banner h2 { font-size: 24px; margin-bottom: 12px; }
  .trust-line { font-size: 12px; color: #555; margin-top: 12px; }
  footer { border-top: 1px solid #1a1a1a; padding: 32px 40px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 16px; }
  .footer-links { display: flex; gap: 14px; flex-wrap: wrap; }
  .footer-links a { font-size: 12px; color: #555; text-decoration: none; }
  .ph-badge { font-size: 11px; color: #eab308; border: 1px solid #eab30844; border-radius: 4px; padding: 2px 8px; text-decoration: none; }
</style>
</head>
<body>

<nav>
  <div style="display:flex;align-items:center;">
    <span class="logo">PatchHive</span>
    <span class="badge">RefactorScout</span>
  </div>
  <div style="display:flex;align-items:center;">
    <span class="nav-link" data-element-id="nav-products">All Products ▾</span>
    <button class="cta-btn" data-element-id="nav-cta">Surface your first refactor</button>
  </div>
</nav>

<div class="hero">
  <h1 data-element-id="hero-headline">Surface the safest high-value refactors your team can make this week.</h1>
  <p data-element-id="hero-subhead">RefactorScout finds bounded, low-risk cleanup opportunities so your team can improve the codebase without a migration plan.</p>
  <div class="hero-actions">
    <button class="cta-btn" data-element-id="hero-cta">Surface your first refactor</button>
    <span class="secondary" data-element-id="hero-secondary">See how it works ↓</span>
  </div>
  <div class="refactor-list" data-element-id="hero-mockup">
    <div class="refactor-item"><div class="refactor-header"><span class="refactor-name">src/auth/session.rs · complexity 47</span><span class="refactor-tag" style="background:#84cc1622;color:#84cc16;">SAFE · extract function</span></div><div class="refactor-meta">3 callers · no external dependencies · estimated 2h</div></div>
    <div class="refactor-item"><div class="refactor-header"><span class="refactor-name">src/utils/format.ts · duplicated in 4 files</span><span class="refactor-tag" style="background:#84cc1622;color:#84cc16;">SAFE · consolidate</span></div><div class="refactor-meta">Identical logic in format.ts, helpers.ts, display.ts, render.ts</div></div>
    <div class="refactor-item"><div class="refactor-header"><span class="refactor-name">src/legacy/parser.py · 0 callers</span><span class="refactor-tag" style="background:#33333344;color:#666;">DEAD CODE · review before delete</span></div><div class="refactor-meta">Last touched 14 months ago · no test coverage</div></div>
  </div>
</div>

<hr class="divider">

<div class="section" data-element-id="problem">
  <div class="section-label">The Problem</div>
  <h2>The codebase grows. Cleanup never makes the sprint.</h2>
  <ul class="bullets">
    <li>Dead code accumulating in files nobody touches</li>
    <li>Duplicated logic spread across multiple modules</li>
    <li>Complexity hotspots that slow every PR in that area</li>
    <li>No time to plan a cleanup — so it never happens</li>
  </ul>
</div>

<hr class="divider">

<div class="section" data-element-id="how-it-works">
  <div class="section-label">How It Works</div>
  <h2>Token. Topics. Run. RefactorScout scouts repos inside your allowed scope.</h2>
  <div class="steps">
    <div class="step" data-element-id="step-1"><div class="step-num">1</div><div class="step-body"><h3>Connect GitHub access</h3><p>RefactorScout uses only the GitHub access you configure.</p></div></div>
    <div class="step" data-element-id="step-2"><div class="step-num">2</div><div class="step-body"><h3>Pick topics and languages</h3><p>RefactorScout searches GitHub for repos matching your interests and scans their codebases.</p></div></div>
    <div class="step" data-element-id="step-3"><div class="step-num">3</div><div class="step-body"><h3>Run</h3><p>RefactorScout autonomously finds complexity hotspots, dead code, and duplicated logic across repos inside your allowed scope.</p></div></div>
    <div class="step" data-element-id="step-4"><div class="step-num">4</div><div class="step-body"><h3>Bounded, safe refactor recommendations</h3><p>Each recommendation includes candidate files, estimated effort, and a risk assessment. No migration plans required.</p></div></div>
  </div>
</div>

<div class="cta-banner" data-element-id="cta-banner">
  <h2>Surface your first refactor.</h2>
  <button class="cta-btn" data-element-id="cta-banner-btn">Surface your first refactor</button>
  <div class="trust-line">Bounded, low-risk opportunities — not broad autonomous rewrites.</div>
</div>

<footer>
  <span style="font-size:13px;color:#555;">© PatchHive · patchhive.dev</span>
  <div class="footer-links">
    <a href="#">SignalHive</a><a href="#">ReviewBee</a><a href="#">TrustGate</a><a href="#">RepoMemory</a><a href="#">MergeKeeper</a><a href="#">FlakeSting</a><a href="#">DepTriage</a><a href="#">VulnTriage</a><a href="#">RepoReaper</a>
  </div>
  <a href="https://patchhive.dev" class="ph-badge" data-element-id="ph-badge">Powered by PatchHive</a>
</footer>

</body>
</html>
```

## RepoReaper (`reporeaper.patchhive.dev`) — accent: purple `#6366f1`

```wireframe

<html>
<head>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; font-family: system-ui, sans-serif; }
  body { background: #0f0f0f; color: #e8e8e8; }
  nav { display: flex; align-items: center; justify-content: space-between; padding: 16px 40px; border-bottom: 1px solid #222; }
  .logo { font-weight: 700; font-size: 15px; color: #fff; }
  .badge { background: #0d0d1a; border: 1px solid #333; border-radius: 4px; padding: 2px 8px; font-size: 11px; color: #6366f1; margin-left: 10px; }
  .cta-btn { background: #6366f1; color: #fff; border: none; border-radius: 6px; padding: 8px 18px; font-size: 13px; font-weight: 700; cursor: pointer; }
  .nav-link { font-size: 13px; color: #888; cursor: pointer; margin-right: 16px; }
  .hero { text-align: center; padding: 100px 40px 60px; max-width: 860px; margin: 0 auto; }
  .hero h1 { font-size: 44px; font-weight: 800; line-height: 1.12; margin-bottom: 20px; }
  .hero h1 span { color: #6366f1; }
  .hero p { font-size: 18px; color: #aaa; margin-bottom: 32px; }
  .hero-actions { display: flex; gap: 12px; justify-content: center; }
  .secondary { font-size: 13px; color: #666; cursor: pointer; }
  .pr-mockup { background: #111; border: 1px solid #222; border-radius: 8px; padding: 20px; margin-top: 32px; text-align: left; }
  .pr-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
  .pr-title { font-size: 14px; font-weight: 700; }
  .pr-open { font-size: 11px; background: #6366f122; color: #6366f1; border: 1px solid #6366f144; border-radius: 4px; padding: 2px 8px; }
  .pr-meta { font-size: 12px; color: #555; margin-bottom: 12px; }
  .pr-body { font-size: 13px; color: #888; line-height: 1.6; }
  .pr-footer-box { background: #0a0a0a; border: 1px solid #1e1e1e; border-radius: 6px; padding: 12px; margin-top: 12px; font-size: 12px; color: #555; font-family: monospace; }
  .section { padding: 64px 40px; max-width: 900px; margin: 0 auto; }
  .section-label { font-size: 11px; text-transform: uppercase; letter-spacing: 2px; color: #666; margin-bottom: 12px; }
  h2 { font-size: 26px; font-weight: 700; margin-bottom: 16px; }
  .divider { border: none; border-top: 1px solid #1a1a1a; margin: 0 40px; }
  .bullets { list-style: none; display: flex; flex-direction: column; gap: 10px; margin-top: 16px; }
  .bullets li::before { content: "→ "; color: #6366f1; }
  .bullets li { font-size: 15px; color: #bbb; }
  .steps { display: flex; flex-direction: column; gap: 16px; margin-top: 20px; }
  .step { display: flex; gap: 16px; }
  .step-num { background: #0d0d1a; border: 1px solid #333; border-radius: 50%; width: 30px; height: 30px; display: flex; align-items: center; justify-content: center; font-size: 12px; color: #6366f1; flex-shrink: 0; }
  .step-body h3 { font-size: 14px; font-weight: 600; margin-bottom: 3px; }
  .step-body p { font-size: 13px; color: #888; }
  .prereq-box { background: #0d0d1a; border: 1px solid #6366f133; border-radius: 8px; padding: 20px; margin-top: 24px; }
  .prereq-box h3 { font-size: 13px; font-weight: 700; color: #6366f1; margin-bottom: 8px; }
  .prereq-box p { font-size: 13px; color: #888; }
  .cta-banner { background: #111; border: 1px solid #6366f133; border-radius: 12px; padding: 48px; text-align: center; margin: 0 40px; }
  .cta-banner h2 { font-size: 24px; margin-bottom: 12px; }
  .trust-line { font-size: 12px; color: #555; margin-top: 12px; }
  footer { border-top: 1px solid #1a1a1a; padding: 32px 40px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 16px; }
  .footer-links { display: flex; gap: 14px; flex-wrap: wrap; }
  .footer-links a { font-size: 12px; color: #555; text-decoration: none; }
  .ph-badge { font-size: 11px; color: #eab308; border: 1px solid #eab30844; border-radius: 4px; padding: 2px 8px; text-decoration: none; }
</style>
</head>
<body>

<nav>
  <div style="display:flex;align-items:center;">
    <span class="logo">PatchHive</span>
    <span class="badge">RepoReaper</span>
  </div>
  <div style="display:flex;align-items:center;">
    <span class="nav-link" data-element-id="nav-products">All Products ▾</span>
    <button class="cta-btn" data-element-id="nav-cta">Deploy RepoReaper</button>
  </div>
</nav>

<div class="hero">
  <h1 data-element-id="hero-headline">PatchHive's hands.<br><span>It hunts bugs, patches them, and files PRs — under its own name.</span></h1>
  <p data-element-id="hero-subhead">RepoReaper takes narrowly scoped issues, generates a validated fix, and opens a pull request signed by PatchHive. You don't pick the repo. You don't pick the issue. You just run it.</p>
  <div class="hero-actions">
    <button class="cta-btn" data-element-id="hero-cta">Deploy RepoReaper</button>
    <span class="secondary" data-element-id="hero-secondary">See how it works ↓</span>
  </div>
  <div class="pr-mockup" data-element-id="hero-mockup">
    <div class="pr-header"><span class="pr-title">fix: handle nil pointer in session cleanup</span><span class="pr-open">Open</span></div>
    <div class="pr-meta">PatchHive opened this PR · rust-lang/cargo · 2 minutes ago</div>
    <div class="pr-body">Fixes #4821. The session cleanup handler did not guard against nil pointers when the session expired mid-request. Added a nil check before dereferencing. Tests pass. Rollback: revert this commit — no schema changes.</div>
    <div class="pr-footer-box" data-element-id="pr-attribution">This PR was generated autonomously by RepoReaper, a PatchHive tool.<br>Review it like any other PR. · patchhive.dev</div>
  </div>
</div>

<hr class="divider">

  <div class="section" data-element-id="how-it-works">
  <div class="section-label">How It Works</div>
  <h2>Token. Topics. Run. RepoReaper hunts, patches, and files PRs.</h2>
  <div class="steps">
    <div class="step" data-element-id="step-1"><div class="step-num">1</div><div class="step-body"><h3>Connect GitHub access</h3><p>RepoReaper uses only the GitHub access you configure.</p></div></div>
    <div class="step" data-element-id="step-2"><div class="step-num">2</div><div class="step-body"><h3>Pick topics and languages</h3><p>RepoReaper searches GitHub for repos matching your interests. You never pick a repo. You never pick an issue.</p></div></div>
    <div class="step" data-element-id="step-3"><div class="step-num">3</div><div class="step-body"><h3>Run</h3><p>HiveCore dispatches RepoReaper on worthy targets. It generates a bounded fix and runs tests.</p></div></div>
    <div class="step" data-element-id="step-4"><div class="step-num">4</div><div class="step-body"><h3>PR opened with evidence and rollback notes</h3><p>Fix rationale, test results, and rollback instructions — all in the PR body.</p></div></div>
    <div class="step" data-element-id="step-5"><div class="step-num">5</div><div class="step-body"><h3>Signed by PatchHive</h3><p>The PR is attributed to the PatchHive GitHub account. Maintainers review it like any other contributor's work.</p></div></div>
  </div>
  <div class="prereq-box" data-element-id="prereq-box">
    <h3>Works best with the full stack</h3>
    <p>RepoReaper is most effective when HiveCore has SignalHive's reconnaissance and TrustGate's policy layer active. Signal quality and policy checks make the fixes better and the PRs safer.</p>
  </div>
</div>

<div class="cta-banner" data-element-id="cta-banner">
  <h2>Deploy RepoReaper.</h2>
  <button class="cta-btn" data-element-id="cta-banner-btn">Deploy RepoReaper</button>
  <div class="trust-line">Not "AI writes code." PatchHive shows up in your PR queue as a known contributor with a track record.</div>
</div>

<footer>
  <span style="font-size:13px;color:#555;">© PatchHive · patchhive.dev</span>
  <div class="footer-links">
    <a href="#">SignalHive</a><a href="#">ReviewBee</a><a href="#">TrustGate</a><a href="#">RepoMemory</a><a href="#">MergeKeeper</a><a href="#">FlakeSting</a><a href="#">DepTriage</a><a href="#">VulnTriage</a><a href="#">RefactorScout</a>
  </div>
  <a href="https://patchhive.dev" class="ph-badge" data-element-id="ph-badge">Powered by PatchHive</a>
</footer>

</body>
</html>
```