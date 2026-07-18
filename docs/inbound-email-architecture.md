# PatchHive Email And Agentic Webmail Architecture

**Status:** Architecture decision recorded; not implemented.

**Preferred working name:** HiveMail. Final confirmation remains open until
scaffolding.

## Decision

PatchHive will eventually own an inbound email capability that can monitor a
PatchHive mailbox, understand repository-related messages, reply when the
response is low risk, dispatch approved work to existing products, and escalate
uncertain messages to the operator with a concise summary.

PatchHive will also expose a focused agentic webmail surface for the actual suite
mailbox. The operator must be able to browse and search messages, read complete
threads, compose and reply, review AI summaries and drafts, and dispatch approved
work into the appropriate PatchHive product. This is a PatchHive operations
inbox, not an attempt to reproduce every consumer Gmail feature.

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
- operator mailbox browsing, search, thread, compose, and reply workflows;
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
| Agentic webmail surface | Let the operator browse, search, read, compose, reply, review agent proposals, and dispatch approved work without exposing provider credentials. |
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

## Agentic Webmail Surface

The mailbox UI is a first-class operator surface over the same trusted intake,
thread, policy, and delivery records used by automation. It must not become a
second browser-owned Gmail integration or a parallel source of truth.

### Mailbox access

The operator must eventually be able to:

- browse a paginated inbox and filtered views such as unread, escalated,
  awaiting approval, dispatched, and completed;
- search by sender, recipient, subject, content, date, thread, repository,
  classification, and workflow state;
- read complete normalized threads while retaining a safe path to original
  message details;
- compose new messages and create, edit, approve, and send replies or forwards;
- manage read state, labels, archive, and trash through audited backend actions;
- open the related PatchHive run, policy decision, finding, or repository from
  the message thread.

Permanent deletion must never be an autonomous agent action. Initial attachment
support remains disabled until both inbound extraction and outbound composition
have explicit type, size, malware, and privacy controls.

### Agentic assistance

The agent layer may:

- summarize a message or thread with references to the source messages;
- prioritize and classify mail with visible confidence, risk, and rationale;
- draft replies from approved PatchHive documentation, templates, and thread
  context;
- suggest follow-ups, verification steps, and the appropriate specialist
  product;
- answer operator questions across the mailbox while citing the messages used;
- produce a structured one-click dispatch proposal for an advertised PatchHive
  capability and attach the resulting run to the thread.

Email content remains untrusted data even when the operator asks the agent about
it. Quoted instructions, signatures, attachments, and linked pages may inform a
summary but may never become agent commands.

### Approval and action boundary

A draft is not a send, and a suggestion is not authorization. Externally visible
or destructive actions must produce an approval record before execution by
default, including agent-generated sends, forwards, trash or delete operations,
opt-out policy changes, and PatchHive product dispatches. Later bounded
auto-reply rules may bypass per-message approval only when an explicit policy
allows a fixed low-risk action.

Every provider mutation and product action must record the operator or policy
that authorized it, the exact proposed payload, the final payload, the execution
result, and any difference between the proposal and what was sent.

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
  content, received time, thread ID, provider labels, read/archive state,
  deduplication key, and processing state;
- **draft** — author, source message or thread, generation evidence, editable
  body, approval state, final sent body, and provider delivery ID;
- **classification** — intent, confidence, risk, evidence, classifier version,
  and policy version;
- **decision** — allowed action, denial or escalation reason, operator approval,
  and related product capability;
- **dispatch** — target product, action ID, request ID, run ID, status, and
  response reference;
- **delivery** — reply or forward recipient, template/version, provider delivery
  ID, attempt count, and final status;
- **mailbox mutation** — label, read-state, archive, trash, or delete proposal,
  authorization, provider result, and audit timestamp.

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

The agentic webmail decision makes the specialist-product option materially
stronger, but it does not settle the packaging or final name. The trusted mail
engine may still live in shared backend infrastructure while a distinct product
owns the operator experience.

The current decision is intentionally narrower: PatchHive owns the capability,
and Hermes is not part of its production runtime.

## Rollout

### Phase 1 — Read-only intake

- Connect one PatchHive Gmail account.
- Persist and deduplicate messages.
- Normalize safe plain text.
- Provide authenticated inbox, search, and complete thread views without
  exposing Gmail credentials to the browser.
- Forward every valid message to Jeremy with a deterministic summary until the
  operator inbox is the preferred review surface.
- Provide visibility without automatic replies or product dispatch.

### Phase 2 — Classification and suggested replies

- Add the typed classifier and confidence thresholds.
- Add agentic mailbox questions, thread summaries, triage, and cited answers.
- Draft responses from approved documentation and templates.
- Let the operator compose, edit, approve, and send replies through the trusted
  backend.
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
- the operator can browse, search, read threads, compose, and reply without Gmail
  OAuth credentials or provider keys reaching the browser;
- agent summaries and mailbox answers identify the source messages they used;
- every send, forward, mailbox mutation, and product dispatch is either approved
  or covered by a visible bounded automation policy;
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
- which mailbox mutations ship in the first operator UI and whether multi-mailbox
  support is ever needed;
- whether attachment support is needed at all in the first public release.

These remain open by design. The durable decision is that email becomes a
PatchHive-owned, auditable, policy-gated capability with a focused agentic
webmail surface rather than a dependency on Hermes or a generic Gmail clone.
