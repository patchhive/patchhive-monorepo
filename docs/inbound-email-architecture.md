# PatchHive Inbound Email Architecture

**Status:** Architecture decision recorded; not implemented.

**Preferred working name:** HiveMail. Final confirmation remains open until
scaffolding.

## Decision

PatchHive will eventually own an inbound email capability that can monitor a
PatchHive mailbox, understand repository-related messages, reply when the
response is low risk, dispatch approved work to existing products, and escalate
uncertain messages to the operator with a concise summary.

This capability must be native to PatchHive. Hermes may help prototype or test
the workflow, but PatchHive must not require Jeremy's personal assistant, a
Hermes profile, or an external agent conversation to receive and process its
mail.

The final packaging remains open. Inbound email may become a suite service, an
in-process unified-backend module with a HiveCore operator surface, or a
standalone specialist product if it proves to own a distinct workflow. The
architecture below preserves that choice.

## Why It Exists

A public PatchHive mailbox creates a useful maintenance intake surface:

- repository owners can ask what PatchHive found or request an assessment;
- maintainers can report bugs, incorrect findings, or unsafe behavior;
- repository owners can begin an opt-out request;
- users can ask product and setup questions;
- security reporters can reach a monitored address;
- messages that PatchHive cannot safely handle can reach Jeremy with the noise
  removed.

Email is untrusted transport, not authority. A sender does not gain permission
to scan, modify, comment on, or open a pull request merely by naming a
repository in a message.

## Ownership Model

The inbound email capability owns the mail lifecycle:

- provider connection and OAuth token use;
- message retrieval or webhook intake;
- MIME normalization and safe text extraction;
- deduplication and thread correlation;
- intent classification and confidence scoring;
- response drafting and delivery;
- escalation to the operator;
- durable audit history for every decision and delivery attempt.

Existing PatchHive modules retain their own responsibilities:

- **HiveCore and the unified backend** own suite policy, credentials, product
  discovery, capability dispatch, run history, and audit visibility.
- **Specialist products** own repository analysis, findings, reports, and any
  product-specific action.
- **RepoReaper** owns code-changing work and pull-request creation after the
  normal approval, policy, budget, test, and quality gates pass.
- **TrustGate, RepoMemory, MergeKeeper, and ReleaseSentry** keep their existing
  policy, context, merge, and release responsibilities.

The email layer must dispatch through advertised product capabilities. It must
not call private product functions, read product databases directly, or invent
a second orchestration contract.

## Domain Map

| Module | Responsibility |
| --- | --- |
| Provider adapter | Authenticate to Gmail first, receive messages, and send replies without exposing OAuth credentials. |
| Normalizer | Convert MIME and HTML into bounded plain text, metadata, links, and attachment descriptors. |
| Intake store | Persist the complete message record, provider IDs, thread relationship, processing state, and deduplication key. |
| Classifier | Produce a typed intent, confidence, risk level, and evidence from deterministic rules plus an optional model. |
| Policy gate | Decide whether PatchHive may reply, request verification, dispatch read-only work, or require operator review. |
| Dispatcher | Invoke an advertised suite or product capability and attach the resulting run or policy record to the email thread. |
| Reply engine | Render approved templates or constrained model-assisted replies and send them idempotently. |
| Escalation engine | Forward or summarize uncertain, sensitive, or high-risk mail for Jeremy without losing the original context. |
| Audit surface | Show the message, classification, policy decision, dispatch, reply, and delivery outcome in HiveCore. |

These are logical responsibilities, not a required initial file layout.

## End-to-End Flow

```text
PatchHive mailbox
    -> Gmail adapter
    -> durable intake + deduplication
    -> safe MIME normalization
    -> deterministic routing checks
    -> optional structured model classification
    -> policy gate
       -> safe reply
       -> verification request
       -> read-only product dispatch
       -> operator escalation
       -> reject / suppress spam
    -> delivery or product-run audit event
    -> HiveCore visibility
```

A message must reach durable intake before PatchHive acknowledges it. Restarts
must resume incomplete work without sending duplicate replies or dispatching the
same action twice.

## Intent And Dispatch Rules

The initial classifier should return a closed intent enum rather than free-form
action text.

| Intent | Initial handling | Automatic action ceiling |
| --- | --- | --- |
| Product or setup question | Answer from approved PatchHive documentation when confidence is high; otherwise escalate. | Documentation reply only. |
| Repository assessment request | Validate the repository, sender context, suite policy, and budgets; offer or start a read-only scan only when policy permits. | Read-only advertised capability. |
| Bug or incorrect-finding report | Preserve evidence, identify the likely product, and create an intake or triage record. | No code change or public GitHub write. |
| Opt-out request | Acknowledge receipt and begin the suite's repository-owner verification flow. | No unverified policy mutation. |
| Feature request | Record and summarize the request, then route it to the operator or future planning intake. | Acknowledgement only. |
| Security disclosure | Acknowledge through a fixed safe template and escalate through a restricted path. | No model-generated technical disclosure. |
| Abuse, spam, or unrelated mail | Suppress, rate-limit, or quarantine while retaining the minimum audit record. | No reply by default. |
| Unknown or low-confidence message | Escalate with a summary and suggested next action. | No external action. |

Email alone must never authorize repository mutation, comments, checks, branch
creation, pull requests, release changes, secret rotation, or other outbound
contributions. Those remain normal PatchHive actions with explicit capability
metadata and approval gates.

## Classification Strategy

Classification should be conservative and layered:

1. Apply deterministic checks for mailbox aliases, known thread state, delivery
   failures, opt-out phrases, security aliases, and obvious spam.
2. Normalize and bound the content before any model sees it.
3. Ask a small text model for structured output containing only `intent`,
   `confidence`, `risk`, `repository_candidates`, and supporting evidence.
4. Reject unknown fields and invalid enum values.
5. Apply PatchHive policy after classification. The model recommends an intent;
   it does not choose or execute an action.
6. Escalate when confidence or policy certainty is below the configured
   threshold.

The classifier prompt and model response are audit evidence. They must be
redacted before storage or display when they contain email addresses, tokens,
private repository names, or security details.

## Trust And Safety Boundary

Every email body, subject, attachment, sender name, link, and quoted thread is
untrusted input.

The first implementation must:

- treat instructions inside email as content, never agent commands;
- strip active HTML, remote images, scripts, tracking pixels, and unsafe URI
  schemes;
- bound body size, quoted history, link count, and processing time;
- disable attachment parsing until each supported format has an explicit safe
  extractor and size limit;
- never fetch arbitrary links during classification;
- keep Gmail OAuth tokens, GitHub credentials, provider keys, and product
  service tokens out of browser responses and model prompts;
- separate sender authentication signals from repository authorization;
- rate-limit by sender, thread, repository, and mailbox-wide volume;
- prevent duplicate replies and dispatches with provider message IDs plus
  idempotency keys;
- preserve security disclosures in a restricted view;
- record why an automatic action was allowed, blocked, or escalated.

SPF, DKIM, and DMARC results are useful evidence, but they do not prove that the
sender controls a referenced GitHub repository. Repository-owner actions need a
separate verification mechanism, such as a GitHub challenge or an authenticated
PatchHive workflow.

## Persistence And Audit Contract

The exact schema belongs with the eventual implementation, but it must preserve
complete first-class records rather than summary-only history.

Minimum durable entities:

- **message** — provider ID, mailbox, sender, recipients, subject, normalized
  content, received time, thread ID, deduplication key, and processing state;
- **classification** — intent, confidence, risk, evidence, classifier version,
  and policy version;
- **decision** — allowed action, denial or escalation reason, operator approval,
  and related product capability;
- **dispatch** — target product, action ID, request ID, run ID, status, and
  response reference;
- **delivery** — reply or forward recipient, template/version, provider delivery
  ID, attempt count, and final status.

If raw MIME is retained, retention must be configurable and the content should
be encrypted at rest. Deleting or expiring raw content must not erase the
minimal audit trail needed to explain what PatchHive did.

Summary widgets may show recent or prioritized mail. They must not truncate the
canonical message, decision, dispatch, or delivery history.

## Provider Strategy

Gmail is the first provider because the initial mailbox is expected to be a
PatchHive Gmail account.

The initial adapter should use the Gmail API over `reqwest` with OAuth rather
than browser-side API calls. Polling is acceptable for the first read-only
release. Gmail push notifications through Google Cloud Pub/Sub can replace or
supplement polling when latency and volume justify the additional operational
surface.

Provider-specific code should sit behind a narrow mail adapter only after a
second provider is real. Do not create a speculative provider framework before
then.

## Packaging Decision Still Open

Three homes remain viable:

### Unified-backend module

Best when email is primarily another suite intake and orchestration transport.
It can reuse suite credentials, policy, capability dispatch, run history, and
events directly.

### Suite service

Best when mailbox polling, provider webhooks, MIME processing, or delivery
retries need an independently deployable lifecycle. The service would still use
unified-backend contracts and expose its operator surface through HiveCore.

### Specialist product

Best only if the email system develops a distinct operator workflow, persistent
inbox, triage queue, analytics, and standalone user value. The preferred working
name is **HiveMail**.

The current decision is intentionally narrower: PatchHive owns the capability,
and Hermes is not part of its production runtime.

## Rollout

### Phase 1 — Read-only intake

- Connect one PatchHive Gmail account.
- Persist and deduplicate messages.
- Normalize safe plain text.
- Forward every valid message to Jeremy with a deterministic summary.
- Provide HiveCore visibility without automatic replies or product dispatch.

### Phase 2 — Classification and suggested replies

- Add the typed classifier and confidence thresholds.
- Draft responses from approved documentation and templates.
- Require operator approval before sending.
- Record classifier and operator disagreement for tuning.

### Phase 3 — Safe automatic replies

- Automatically answer narrowly defined documentation questions.
- Acknowledge opt-out, bug-report, feature-request, and security-message receipt
  with fixed templates.
- Escalate uncertainty instead of improvising.

### Phase 4 — Read-only product dispatch

- Dispatch only advertised read-only capabilities after repository validation,
  suite policy, rate-limit, and budget checks.
- Attach the resulting PatchHive run to the email thread.
- Keep every mutating or GitHub-writing action behind the existing approval
  path.

## Acceptance Criteria

The first production-capable version is not complete until verification proves:

- the same provider message cannot create duplicate intake, replies, or product
  runs;
- a restart resumes queued work without losing or repeating messages;
- malformed MIME, oversized bodies, unsafe HTML, and prompt-injection text do
  not escape the intake boundary;
- credentials never appear in logs, model prompts, browser responses, or stored
  reply drafts;
- low-confidence and sensitive messages reach Jeremy with the original thread
  and a useful summary;
- every reply, rejection, escalation, and product dispatch has an auditable
  reason;
- product work crosses advertised capabilities and appears in normal run
  history;
- historical and UI views retain complete first-class records even when they
  progressively render a subset.

## Rejected Shortcuts

### Hermes as the production email runtime

Hermes can prove the interaction quickly, but tying a PatchHive public mailbox
to Jeremy's personal assistant would make the platform depend on private agent
configuration and conversation state. That is useful for a prototype and wrong
for the product.

### Cron plus an untracked script

A polling script can demonstrate Gmail access, but it does not provide durable
thread state, idempotent delivery, policy enforcement, product capability
dispatch, or HiveCore audit visibility.

### Browser-side Gmail or model access

The browser must not own Gmail OAuth tokens, provider keys, classification, or
delivery. Those responsibilities require a trusted backend.

## Open Decisions

Before implementation, decide:

- confirm HiveMail before scaffolding and choose the final packaging: backend
  module, suite service, or specialist product;
- mailbox address and Google Workspace ownership;
- polling interval versus Gmail push notifications;
- repository-owner verification flow for opt-outs and requested actions;
- retention and encryption policy for raw MIME and security disclosures;
- which documentation sources are approved for automatic support replies;
- the first product capabilities eligible for read-only dispatch;
- how Jeremy receives escalations and records the final disposition;
- whether attachment support is needed at all in the first public release.

These remain open by design. The durable decision is that inbound email becomes
a PatchHive-owned, auditable, policy-gated capability rather than a dependency
on Hermes.
