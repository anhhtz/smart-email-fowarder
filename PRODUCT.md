# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

- **Primary**: General Office & Operations Staff who need to review forwarded email history, verify delivery statuses, search/filter email logs, and manage recipient forwarding lists without touching server terminals.
- **Secondary**: Enterprise Sysadmins and IT Operators who oversee service uptime, background IMAP poller/SMTP relay health, process resource usage, and network connectivity in air-gapped environments.

## Product Purpose

Smart Email Forwarder provides reliable, deduplicated email forwarding from internal corporate Microsoft Exchange mailboxes (via IMAP) to external destination accounts (such as Gmail) via an internal Exchange SMTP relay.

Success means:
- Zero dropped messages and zero duplicate forwards across server restarts.
- Full fidelity preservation of email contents, headers, and attachments.
- Clear, immediate visibility for operations staff into email transit history and recipient settings without requiring command-line or database tools.

## Positioning

A zero-external-dependency, 100% offline-ready single executable with an embedded web administration dashboard. Unlike cloud forwarders or complex daemon stacks requiring Node.js, Python, or external SQL engines, Smart Email Forwarder runs as a self-contained binary on restricted, air-gapped enterprise servers (Windows Server 2019 / Linux Mint) that have no public internet access.

## Operating Context

- **Environment**: Air-gapped or heavily firewalled internal corporate LANs where the host machine communicates exclusively with local Exchange servers for both IMAP reading and SMTP relay.
- **Access**: Accessed locally or across corporate intranet browsers (`http://localhost:8080`).
- **Execution Mode**: Unattended background service (native Windows Service on Windows Server 2019; systemd daemon on Linux Mint).

## Capabilities and Constraints

- **Capabilities**:
  - Automated IMAP mailbox polling with duplicate detection backed by embedded SQLite.
  - SMTP relay forwarding with full MIME preservation (rich HTML, plain text, multipart attachments, original `Reply-To`).
  - Web UI for monitoring real-time sync statistics, process resource consumption (CPU, RSS/VmSize, threads), and event logs.
  - Immediate manual "Sync Now" triggers and Exchange IMAP/SMTP connection diagnostic tests.
  - Forwarding recipient configuration and audit trails.
- **Constraints**:
  - **Strict 100% offline operation**: Zero runtime downloads, zero external CDNs, zero third-party remote font or script dependencies.
  - All web assets (HTML, CSS, JavaScript) must be statically embedded into the Rust executable at compile time (`include_str!`).
  - Must remain fully functional in air-gapped enterprise server browsers.

## Brand Commitments

- **Name**: Smart Email Forwarder
- **Tone & Voice**: Calm, precise, reliable, and functional. Avoids marketing fluff or decorative distraction in favor of clear status communication and actionable controls.

## Evidence on Hand

- Rust backend source code in [`src/`](file:///Users/anhht/gits/GitHub/anhhtca/smart-email-fowarder/src) using Axum, bundled SQLite, and IMAP/SMTP clients.
- Existing vanilla web UI in [`static/`](file:///Users/anhht/gits/GitHub/anhhtca/smart-email-fowarder/static) (`index.html`, `style.css`, `app.js`).
- System deployment configurations for Windows Service and Linux systemd in [`deploy/`](file:///Users/anhht/gits/GitHub/anhhtca/smart-email-fowarder/deploy).
- Comprehensive test suite in [`tests/`](file:///Users/anhht/gits/GitHub/anhhtca/smart-email-fowarder/tests).

## Product Principles

1. **Air-Gapped Autonomy**: The application and UI must function flawlessly without internet connectivity; zero external network requests allowed.
2. **Operational Transparency**: Both technical sysadmins and non-technical office staff should grasp email forwarding status and service health at a glance.
3. **Data Integrity & Deduplication**: Never drop an email, never lose an attachment, and never forward duplicate messages.
4. **Utilitarian Density & Ergonomics**: Dense, scannable data layouts that prioritize clarity, searchability, and immediate diagnostic feedback over decorative animation.
