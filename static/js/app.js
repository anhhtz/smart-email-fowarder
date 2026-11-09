// Smart Email Forwarder - 100% Offline Vanilla Frontend Client
(function () {
  "use strict";

  // State
  let currentTab = "dashboard";
  let statusPollTimer = null;
  let isPollerPaused = false;

  // DOM Elements
  const navItems = document.querySelectorAll(".nav-item");
  const tabPanes = document.querySelectorAll(".tab-pane");
  const pageHeading = document.getElementById("pageHeading");
  const pageSubheading = document.getElementById("pageSubheading");
  const alertBox = document.getElementById("alertBox");

  // Topbar elements
  const btnSyncNow = document.getElementById("btnSyncNow");
  const btnTogglePoller = document.getElementById("btnTogglePoller");
  const togglePollerLabel = document.getElementById("togglePollerLabel");

  // Stats elements
  const statStatus = document.getElementById("statStatus");
  const statBadgeStatus = document.getElementById("statBadgeStatus");
  const statNextPoll = document.getElementById("statNextPoll");
  const statTotalForwarded = document.getElementById("statTotalForwarded");
  const statTotalFailed = document.getElementById("statTotalFailed");
  const statInterval = document.getElementById("statInterval");
  const statLastPoll = document.getElementById("statLastPoll");
  const sidebarStatusDot = document.getElementById("sidebarStatusDot");
  const sidebarStatusText = document.getElementById("sidebarStatusText");

  // Tables & Logs
  const activityTableBody = document.getElementById("activityTableBody");
  const btnRefreshActivity = document.getElementById("btnRefreshActivity");
  const logViewer = document.getElementById("logViewer");
  const btnClearLogs = document.getElementById("btnClearLogs");
  const btnRefreshLogs = document.getElementById("btnRefreshLogs");

  // Forms
  const formExchange = document.getElementById("formExchange");
  const btnTestExchange = document.getElementById("btnTestExchange");
  const formForwarding = document.getElementById("formForwarding");
  const formSmtp = document.getElementById("formSmtp");
  const btnTestSmtp = document.getElementById("btnTestSmtp");
  const smtpAuthRequired = document.getElementById("smtpAuthRequired");
  const smtpAuthFields = document.getElementById("smtpAuthFields");
  const smtpUseExchangeCredentials = document.getElementById("smtpUseExchangeCredentials");
  const smtpExchangeCredentialsInfo = document.getElementById("smtpExchangeCredentialsInfo");
  const exchangeAccountDisplay = document.getElementById("exchangeAccountDisplay");
  const smtpCustomCredentials = document.getElementById("smtpCustomCredentials");

  // Process Monitor Elements
  const procBadgeStatus = document.getElementById("procBadgeStatus");
  const procStatusText = document.getElementById("procStatusText");
  const procServiceMode = document.getElementById("procServiceMode");
  const procPid = document.getElementById("procPid");
  const procPpid = document.getElementById("procPpid");
  const procUptime = document.getElementById("procUptime");
  const procStartedAt = document.getElementById("procStartedAt");
  const btnToggleProcAutoRefresh = document.getElementById(
    "btnToggleProcAutoRefresh",
  );
  const procAutoRefreshLabel = document.getElementById("procAutoRefreshLabel");
  const btnRefreshProcess = document.getElementById("btnRefreshProcess");
  const btnRunHealthCheck = document.getElementById("btnRunHealthCheck");
  const procMemRss = document.getElementById("procMemRss");
  const procMemBar = document.getElementById("procMemBar");
  const procMemVm = document.getElementById("procMemVm");
  const procMemPeak = document.getElementById("procMemPeak");
  const procLoad1m = document.getElementById("procLoad1m");
  const procLoadAll = document.getElementById("procLoadAll");
  const procCpuCores = document.getElementById("procCpuCores");
  const procThreads = document.getElementById("procThreads");
  const procDbBadge = document.getElementById("procDbBadge");
  const procDbSize = document.getElementById("procDbSize");
  const procDbProcessed = document.getElementById("procDbProcessed");
  const procDbJournal = document.getElementById("procDbJournal");
  const procLastUpdated = document.getElementById("procLastUpdated");

  // Subsystems Elements
  const subPollerDot = document.getElementById("subPollerDot");
  const subPollerBadge = document.getElementById("subPollerBadge");
  const subPollerInterval = document.getElementById("subPollerInterval");
  const subPollerNext = document.getElementById("subPollerNext");
  const subPollerForwarded = document.getElementById("subPollerForwarded");
  const subPollerFailed = document.getElementById("subPollerFailed");

  const subDbDot = document.getElementById("subDbDot");
  const subDbBadge = document.getElementById("subDbBadge");
  const subDbPath = document.getElementById("subDbPath");
  const subDbIntegrity = document.getElementById("subDbIntegrity");
  const subDbLogs = document.getElementById("subDbLogs");

  const subImapDot = document.getElementById("subImapDot");
  const subImapBadge = document.getElementById("subImapBadge");
  const subImapHost = document.getElementById("subImapHost");
  const subImapSec = document.getElementById("subImapSec");
  const subImapFolder = document.getElementById("subImapFolder");

  const subSmtpDot = document.getElementById("subSmtpDot");
  const subSmtpBadge = document.getElementById("subSmtpBadge");
  const subSmtpHost = document.getElementById("subSmtpHost");
  const subSmtpPortSec = document.getElementById("subSmtpPortSec");
  const subSmtpSender = document.getElementById("subSmtpSender");

  // Health Diagnostics elements
  const healthOverallBadge = document.getElementById("healthOverallBadge");
  const healthTableBody = document.getElementById("healthTableBody");

  // Environment elements
  const envPid = document.getElementById("envPid");
  const envPpid = document.getElementById("envPpid");
  const envPlatform = document.getElementById("envPlatform");
  const envCores = document.getElementById("envCores");
  const envExePath = document.getElementById("envExePath");
  const envWorkDir = document.getElementById("envWorkDir");
  const envCmdArgs = document.getElementById("envCmdArgs");
  const envAppVersion = document.getElementById("envAppVersion");
  const envStartedTime = document.getElementById("envStartedTime");

  let isProcAutoRefresh = true;

  // Modal
  const detailModal = document.getElementById("detailModal");
  const btnModalClose = document.getElementById("btnModalClose");
  const modalContent = document.getElementById("modalContent");

  const tabTitles = {
    dashboard: {
      title: "Dashboard",
      sub: "System health, forward statistics, and recent activity.",
    },
    process: {
      title: "Process Monitor",
      sub: "Real-time telemetry, memory usage, worker threads, and process health.",
    },
    exchange: {
      title: "Exchange Mailbox",
      sub: "Configure the IMAP connection to access your internal MS Exchange mailbox.",
    },
    forwarding: {
      title: "Forwarding Rules",
      sub: "Specify destination Gmail accounts and forwarding appearance.",
    },
    smtp: {
      title: "Exchange Relay",
      sub: "Configure the outbound SMTP connection to relay emails to Gmail.",
    },
    logs: {
      title: "Activity Logs",
      sub: "Real-time operational events and detailed diagnostic logs.",
    },
  };

  // Helper: Display Alert
  function showAlert(message, type = "info", duration = 5000) {
    alertBox.textContent = message;
    alertBox.className = `alert-box ${type}`;
    if (duration > 0) {
      setTimeout(() => {
        alertBox.className = "alert-box hidden";
      }, duration);
    }
  }

  // Tab Navigation
  function switchTab(tabId) {
    currentTab = tabId;
    navItems.forEach((item) => {
      item.classList.toggle("active", item.dataset.tab === tabId);
    });
    tabPanes.forEach((pane) => {
      pane.classList.toggle("active", pane.id === `tab-${tabId}`);
    });
    if (tabTitles[tabId]) {
      pageHeading.textContent = tabTitles[tabId].title;
      pageSubheading.textContent = tabTitles[tabId].sub;
    }
    if (tabId === "logs") {
      loadLogs();
    } else if (tabId === "process") {
      loadProcessMetrics();
    }
  }

  navItems.forEach((item) => {
    item.addEventListener("click", () => switchTab(item.dataset.tab));
  });

  // Toggle SMTP Auth Fields and Exchange Credentials
  function updateSmtpAuthUI() {
    const isAuth = smtpAuthRequired.checked;
    smtpAuthFields.classList.toggle("hidden", !isAuth);

    if (!isAuth) {
      return;
    }

    const useExCreds = smtpUseExchangeCredentials.checked;
    smtpCustomCredentials.classList.toggle("hidden", useExCreds);
    smtpExchangeCredentialsInfo.classList.toggle("hidden", !useExCreds);

    if (useExCreds) {
      const exUser = (document.getElementById("exchangeUsername").value || "").trim();
      const badge = smtpExchangeCredentialsInfo.querySelector(".credentials-badge-info");
      if (exUser) {
        exchangeAccountDisplay.textContent = exUser;
        if (badge) badge.className = "credentials-badge-info";
        const fromEmailInput = document.getElementById("smtpFromEmail");
        if (fromEmailInput && !fromEmailInput.value.trim() && exUser.includes("@")) {
          fromEmailInput.value = exUser;
        }
      } else {
        exchangeAccountDisplay.textContent = "None configured (please configure in Exchange Mailbox tab)";
        if (badge) badge.className = "credentials-badge-info warning";
      }
    }
  }

  smtpAuthRequired.addEventListener("change", updateSmtpAuthUI);
  smtpUseExchangeCredentials.addEventListener("change", updateSmtpAuthUI);
  const exUsernameEl = document.getElementById("exchangeUsername");
  if (exUsernameEl) {
    exUsernameEl.addEventListener("input", updateSmtpAuthUI);
  }

  // Modal handlers
  function showModal(title, html) {
    document.getElementById("modalTitle").textContent = title;
    modalContent.innerHTML = html;
    detailModal.classList.remove("hidden");
  }

  btnModalClose.addEventListener("click", () =>
    detailModal.classList.add("hidden"),
  );
  detailModal
    .querySelector(".modal-backdrop")
    .addEventListener("click", () => detailModal.classList.add("hidden"));

  // API Call Helpers
  async function api(path, options = {}) {
    const res = await fetch(`/api${path}`, {
      headers: { "Content-Type": "application/json" },
      ...options,
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({ message: res.statusText }));
      throw new Error(err.message || `Request failed with code ${res.status}`);
    }
    return res.json();
  }

  // Load Status & Stats
  async function loadStatus() {
    try {
      const status = await api("/status");

      isPollerPaused = !status.is_polling;
      togglePollerLabel.textContent = isPollerPaused
        ? "Resume Poller"
        : "Pause Poller";

      statStatus.textContent = isPollerPaused
        ? "Paused"
        : status.is_busy
          ? "Syncing..."
          : "Active";
      statBadgeStatus.textContent = isPollerPaused ? "Paused" : "Active";
      statBadgeStatus.className = isPollerPaused
        ? "badge badge-warning"
        : "badge badge-success";

      sidebarStatusDot.className = `status-dot ${isPollerPaused ? "paused" : "active"}`;
      sidebarStatusText.textContent = isPollerPaused
        ? "Service Paused"
        : "Service Active";

      statTotalForwarded.textContent = status.total_forwarded || 0;
      statTotalFailed.textContent = status.total_failed || 0;
      statInterval.textContent = `${status.poll_interval_minutes || 1} min`;
      statLastPoll.textContent = status.last_poll_time
        ? new Date(status.last_poll_time).toLocaleTimeString()
        : "Never";

      if (status.next_poll_seconds != null) {
        if (isPollerPaused) {
          statNextPoll.textContent = "Paused";
        } else if (status.next_poll_seconds <= 0) {
          statNextPoll.textContent = "Polling now...";
        } else {
          statNextPoll.textContent = `In ${status.next_poll_seconds}s`;
        }
      }
    } catch (e) {
      console.error("Status load error:", e);
    }
  }

  // Load Configurations
  async function loadConfig() {
    try {
      const config = await api("/config");

      // Exchange Form
      if (config.exchange) {
        document.getElementById("exchangeHost").value =
          config.exchange.host || "";
        document.getElementById("exchangePort").value =
          config.exchange.port || 993;
        document.getElementById("exchangeSecurity").value =
          config.exchange.security || "ssl";
        document.getElementById("exchangeUsername").value =
          config.exchange.username || "";
        document.getElementById("exchangePassword").value = "";
        document.getElementById("exchangeFolder").value =
          config.exchange.folder || "INBOX";
        document.getElementById("exchangeMarkRead").checked =
          config.exchange.mark_as_read !== false;
        document.getElementById("exchangeAllowSelfSigned").checked =
          !!config.exchange.allow_self_signed;
      }

      // Forwarding Form
      if (config.forwarding) {
        document.getElementById("targetEmails").value = (
          config.forwarding.target_emails || []
        ).join(", ");
        document.getElementById("subjectPrefix").value =
          config.forwarding.subject_prefix || "";
        document.getElementById("pollInterval").value =
          config.forwarding.poll_interval_minutes || 1;
        document.getElementById("includeBanner").checked =
          config.forwarding.include_banner !== false;
        document.getElementById("preserveReplyTo").checked =
          config.forwarding.preserve_reply_to !== false;
      }

      // SMTP Relay Form
      if (config.smtp) {
        document.getElementById("smtpHost").value = config.smtp.host || "";
        document.getElementById("smtpPort").value = config.smtp.port || 25;
        document.getElementById("smtpSecurity").value =
          config.smtp.security || "plain";
        document.getElementById("smtpFromEmail").value =
          config.smtp.from_email || "";
        document.getElementById("smtpFromName").value =
          config.smtp.from_name || "Exchange Email Forwarder";
        smtpAuthRequired.checked = !!config.smtp.auth_required;
        smtpUseExchangeCredentials.checked = !!config.smtp.use_exchange_credentials;
        document.getElementById("smtpUsername").value =
          config.smtp.username || "";
        document.getElementById("smtpPassword").value = "";
        updateSmtpAuthUI();
      }
    } catch (e) {
      showAlert(`Failed to load configuration: ${e.message}`, "error");
    }
  }

  // Load Recent Forward Activity
  async function loadActivity() {
    try {
      const logs = await api("/logs?limit=25");
      if (!logs || logs.length === 0) {
        activityTableBody.innerHTML = `<tr><td colspan="6" class="text-center empty-state">No email forwarding events recorded yet.</td></tr>`;
        return;
      }

      activityTableBody.innerHTML = logs
        .map((log) => {
          const dateStr = new Date(log.timestamp).toLocaleString();
          const badgeClass =
            log.status === "SUCCESS" ? "badge-success" : "badge-danger";
          return `
          <tr>
            <td>${escapeHtml(dateStr)}</td>
            <td><strong>${escapeHtml(log.original_sender || "Unknown")}</strong></td>
            <td>${escapeHtml(log.subject || "(No Subject)")}</td>
            <td><code>${escapeHtml(log.forwarded_to || "")}</code></td>
            <td><span class="badge ${badgeClass}">${escapeHtml(log.status)}</span></td>
            <td><button class="btn btn-sm btn-outline btn-view-log" data-log='${JSON.stringify(log)}'>View</button></td>
          </tr>
        `;
        })
        .join("");

      document.querySelectorAll(".btn-view-log").forEach((btn) => {
        btn.addEventListener("click", () => {
          const log = JSON.parse(btn.dataset.log);
          showModal(
            "Forwarding Details",
            `
            <p><strong>Time:</strong> ${new Date(log.timestamp).toLocaleString()}</p>
            <p><strong>Message ID:</strong> <code>${escapeHtml(log.message_id || "N/A")}</code></p>
            <p><strong>Sender:</strong> ${escapeHtml(log.original_sender || "N/A")}</p>
            <p><strong>Subject:</strong> ${escapeHtml(log.subject || "N/A")}</p>
            <p><strong>Recipients:</strong> ${escapeHtml(log.forwarded_to || "N/A")}</p>
            <p><strong>Status:</strong> <span class="badge ${log.status === "SUCCESS" ? "badge-success" : "badge-danger"}">${log.status}</span></p>
            ${log.error_details ? `<div style="margin-top:10px; color:var(--danger)"><strong>Error Details:</strong><pre style="background:#fee2e2; padding:10px; border-radius:4px; overflow:auto;">${escapeHtml(log.error_details)}</pre></div>` : ""}
          `,
          );
        });
      });
    } catch (e) {
      console.error("Activity load error:", e);
    }
  }

  // Load System Event Logs
  async function loadLogs() {
    try {
      const logs = await api("/system-logs");
      if (logs && logs.length > 0) {
        logViewer.innerHTML = logs
          .map((entry) => {
            const typeClass = entry.level.toLowerCase();
            return `<div class="log-entry ${typeClass}">[${new Date(entry.time).toLocaleTimeString()}] [${entry.level}] ${escapeHtml(entry.message)}</div>`;
          })
          .join("");
        logViewer.scrollTop = logViewer.scrollHeight;
      }
    } catch (e) {
      console.error("Log load error:", e);
    }
  }

  // Save Exchange Settings
  formExchange.addEventListener("submit", async (e) => {
    e.preventDefault();
    const data = {
      host: document.getElementById("exchangeHost").value.trim(),
      port: parseInt(document.getElementById("exchangePort").value, 10),
      security: document.getElementById("exchangeSecurity").value,
      username: document.getElementById("exchangeUsername").value.trim(),
      password: document.getElementById("exchangePassword").value,
      folder: document.getElementById("exchangeFolder").value.trim(),
      mark_as_read: document.getElementById("exchangeMarkRead").checked,
      allow_self_signed: document.getElementById("exchangeAllowSelfSigned")
        .checked,
    };
    try {
      await api("/config/exchange", {
        method: "POST",
        body: JSON.stringify(data),
      });
      showAlert("Exchange settings saved successfully.", "success");
      loadStatus();
    } catch (err) {
      showAlert(`Error saving Exchange settings: ${err.message}`, "error");
    }
  });

  // Test Exchange Connection
  btnTestExchange.addEventListener("click", async () => {
    btnTestExchange.disabled = true;
    btnTestExchange.innerHTML =
      '<span class="spin">&#9696;</span> Testing connection...';
    try {
      const data = {
        host: document.getElementById("exchangeHost").value.trim(),
        port: parseInt(document.getElementById("exchangePort").value, 10),
        security: document.getElementById("exchangeSecurity").value,
        username: document.getElementById("exchangeUsername").value.trim(),
        password: document.getElementById("exchangePassword").value,
        folder: document.getElementById("exchangeFolder").value.trim(),
        allow_self_signed: document.getElementById("exchangeAllowSelfSigned")
          .checked,
      };
      const res = await api("/test/exchange", {
        method: "POST",
        body: JSON.stringify(data),
      });
      showAlert(
        `Exchange Connection Success! Found ${res.message_count} total messages (${res.unseen_count} unread) in ${data.folder}.`,
        "success",
        8000,
      );
    } catch (err) {
      showAlert(`Exchange Connection Failed: ${err.message}`, "error", 10000);
    } finally {
      btnTestExchange.disabled = false;
      btnTestExchange.innerHTML =
        "<span>&#128269;</span> Test Exchange Connection";
    }
  });

  // Save Forwarding Rules
  formForwarding.addEventListener("submit", async (e) => {
    e.preventDefault();
    const emails = document
      .getElementById("targetEmails")
      .value.split(/[\n,]+/)
      .map((s) => s.trim())
      .filter(Boolean);
    const data = {
      target_emails: emails,
      subject_prefix: document.getElementById("subjectPrefix").value,
      poll_interval_minutes: parseInt(
        document.getElementById("pollInterval").value,
        10,
      ),
      include_banner: document.getElementById("includeBanner").checked,
      preserve_reply_to: document.getElementById("preserveReplyTo").checked,
    };
    try {
      await api("/config/forwarding", {
        method: "POST",
        body: JSON.stringify(data),
      });
      showAlert("Forwarding rules saved successfully.", "success");
      loadStatus();
    } catch (err) {
      showAlert(`Error saving forwarding rules: ${err.message}`, "error");
    }
  });

  // Save SMTP Relay Settings
  formSmtp.addEventListener("submit", async (e) => {
    e.preventDefault();
    const data = {
      host: document.getElementById("smtpHost").value.trim(),
      port: parseInt(document.getElementById("smtpPort").value, 10),
      security: document.getElementById("smtpSecurity").value,
      from_email: document.getElementById("smtpFromEmail").value.trim(),
      from_name: document.getElementById("smtpFromName").value.trim(),
      auth_required: smtpAuthRequired.checked,
      use_exchange_credentials: smtpUseExchangeCredentials.checked,
      username: document.getElementById("smtpUsername").value.trim(),
      password: document.getElementById("smtpPassword").value,
    };
    try {
      await api("/config/smtp", { method: "POST", body: JSON.stringify(data) });
      showAlert("Exchange SMTP relay settings saved successfully.", "success");
      loadStatus();
    } catch (err) {
      showAlert(`Error saving relay settings: ${err.message}`, "error");
    }
  });

  // Test SMTP Relay (Send Test Email)
  btnTestSmtp.addEventListener("click", async () => {
    btnTestSmtp.disabled = true;
    btnTestSmtp.innerHTML =
      '<span class="spin">&#9696;</span> Sending test email...';
    try {
      const data = {
        host: document.getElementById("smtpHost").value.trim(),
        port: parseInt(document.getElementById("smtpPort").value, 10),
        security: document.getElementById("smtpSecurity").value,
        from_email: document.getElementById("smtpFromEmail").value.trim(),
        from_name: document.getElementById("smtpFromName").value.trim(),
        auth_required: smtpAuthRequired.checked,
        use_exchange_credentials: smtpUseExchangeCredentials.checked,
        username: document.getElementById("smtpUsername").value.trim(),
        password: document.getElementById("smtpPassword").value,
      };
      const res = await api("/test/smtp", {
        method: "POST",
        body: JSON.stringify(data),
      });
      showAlert(
        `Test email successfully relayed to ${res.recipient}! Check your Gmail inbox.`,
        "success",
        8000,
      );
    } catch (err) {
      showAlert(`Relay Test Failed: ${err.message}`, "error", 10000);
    } finally {
      btnTestSmtp.disabled = false;
      btnTestSmtp.innerHTML =
        "<span>&#9993;</span> Send Test Email to Target Gmail";
    }
  });

  // Sync Now Button
  btnSyncNow.addEventListener("click", async () => {
    btnSyncNow.disabled = true;
    const originalText = btnSyncNow.innerHTML;
    btnSyncNow.innerHTML = '<span class="spin">&#9696;</span> Syncing...';
    try {
      const res = await api("/sync", { method: "POST" });
      showAlert(
        `Sync completed: ${res.forwarded_count} emails forwarded, ${res.failed_count} failed.`,
        "success",
      );
      loadStatus();
      loadActivity();
    } catch (err) {
      showAlert(`Sync failed: ${err.message}`, "error");
    } finally {
      btnSyncNow.disabled = false;
      btnSyncNow.innerHTML = originalText;
    }
  });

  // Toggle Poller Pause/Resume
  btnTogglePoller.addEventListener("click", async () => {
    try {
      const action = isPollerPaused ? "resume" : "pause";
      await api(`/poller/${action}`, { method: "POST" });
      loadStatus();
    } catch (err) {
      showAlert(`Failed to toggle poller: ${err.message}`, "error");
    }
  });

  // Refresh Activity / Logs
  btnRefreshActivity.addEventListener("click", () => loadActivity());
  btnRefreshLogs.addEventListener("click", () => loadLogs());
  btnClearLogs.addEventListener("click", () => {
    logViewer.innerHTML =
      '<div class="log-entry info">[System] Log display cleared.</div>';
  });

  // Load Process Metrics & Subsystem Telemetry
  async function loadProcessMetrics() {
    try {
      const data = await api("/process");

      // Top banner
      procPid.textContent = data.pid;
      procPpid.textContent = data.ppid != null ? data.ppid : "N/A";
      procUptime.textContent = data.uptime_formatted;
      procStartedAt.textContent = data.started_at
        ? new Date(data.started_at).toLocaleTimeString()
        : "--";
      procServiceMode.textContent = data.service_mode;

      // Status beacon badge
      const isHealthy = data.database.is_healthy && data.status === "Running";
      procStatusText.textContent = isHealthy ? "HEALTHY" : "DEGRADED";
      procBadgeStatus.className = isHealthy
        ? "badge badge-success beacon-badge"
        : "badge badge-danger beacon-badge";

      // Memory
      procMemRss.textContent = data.memory.rss_formatted || "N/A";
      procMemVm.textContent = data.memory.vmsize_formatted || "N/A";
      procMemPeak.textContent = data.memory.vmpeak_formatted || "N/A";
      if (data.memory.rss_bytes) {
        const pct = Math.min(
          100,
          Math.max(5, (data.memory.rss_bytes / (128 * 1024 * 1024)) * 100),
        );
        procMemBar.style.width = `${pct.toFixed(1)}%`;
      }

      // CPU & Load
      procCpuCores.textContent = `${data.cpu_cores} Cores`;
      if (data.load_average && data.load_average.length >= 3) {
        procLoad1m.textContent = data.load_average[0].toFixed(2);
        procLoadAll.textContent = data.load_average
          .map((n) => n.toFixed(2))
          .join(", ");
      } else {
        procLoad1m.textContent = "Normal";
        procLoadAll.textContent = "N/A (OS loadavg unsupported)";
      }

      // Threads
      procThreads.textContent =
        data.memory.thread_count != null
          ? `${data.memory.thread_count} Threads`
          : "Multi-threaded (Tokio)";

      // Database
      procDbBadge.textContent = data.database.is_healthy ? "HEALTHY" : "ERROR";
      procDbBadge.className = data.database.is_healthy
        ? "badge badge-success"
        : "badge badge-danger";
      procDbSize.textContent = data.database.size_formatted;
      procDbProcessed.textContent = data.database.total_processed_emails;
      procDbJournal.textContent = data.database.journal_mode;

      // Subsystems: Poller
      const isPaused = !data.poller.is_polling;
      subPollerDot.className = `status-dot ${isPaused ? "paused" : "active"}`;
      subPollerBadge.textContent = isPaused
        ? "Paused"
        : data.poller.is_busy
          ? "Syncing..."
          : "Active";
      subPollerBadge.className = isPaused
        ? "badge badge-warning"
        : "badge badge-success";
      subPollerInterval.textContent = `${data.poller.poll_interval_minutes} min`;
      if (isPaused) {
        subPollerNext.textContent = "Paused";
      } else if (data.poller.next_poll_seconds != null) {
        subPollerNext.textContent =
          data.poller.next_poll_seconds <= 0
            ? "Sync in progress..."
            : `In ${data.poller.next_poll_seconds}s`;
      } else {
        subPollerNext.textContent = "Scheduled";
      }
      subPollerForwarded.textContent = data.poller.total_forwarded;
      subPollerFailed.textContent = data.poller.total_failed;

      // Subsystems: Database
      subDbDot.className = `status-dot ${data.database.is_healthy ? "active" : "paused"}`;
      subDbBadge.textContent = data.database.is_healthy
        ? "Connected"
        : "Degraded";
      subDbBadge.className = data.database.is_healthy
        ? "badge badge-success"
        : "badge badge-danger";
      subDbPath.textContent = data.database.path;
      subDbPath.title = data.database.path;
      subDbIntegrity.textContent = `Valid (${data.database.integrity_status})`;
      subDbLogs.textContent = `${data.database.total_system_logs} events / ${data.database.total_forward_logs} forwards`;

      // Subsystems: Exchange & SMTP config reflection
      const cfg = await api("/config").catch(() => ({}));
      if (cfg.exchange && cfg.exchange.host) {
        subImapDot.className = "status-dot active";
        subImapBadge.textContent = "Configured";
        subImapBadge.className = "badge badge-success";
        subImapHost.textContent = `${cfg.exchange.host}:${cfg.exchange.port}`;
        subImapSec.textContent = cfg.exchange.security
          ? cfg.exchange.security.toUpperCase()
          : "SSL";
        subImapFolder.textContent = cfg.exchange.folder || "INBOX";
      } else {
        subImapDot.className = "status-dot paused";
        subImapBadge.textContent = "Not Configured";
        subImapBadge.className = "badge badge-warning";
        subImapHost.textContent = "Not configured";
        subImapSec.textContent = "--";
        subImapFolder.textContent = "--";
      }

      if (cfg.smtp && cfg.smtp.host) {
        subSmtpDot.className = "status-dot active";
        subSmtpBadge.textContent = "Configured";
        subSmtpBadge.className = "badge badge-success";
        const authMode = cfg.smtp.use_exchange_credentials
          ? "Exchange Mailbox Auth"
          : (cfg.smtp.auth_required ? "Custom Auth" : "No Auth");
        subSmtpPortSec.textContent = `${cfg.smtp.port} (${cfg.smtp.security || "plain"}, ${authMode})`;
        subSmtpSender.textContent = cfg.smtp.from_email;
      } else {
        subSmtpDot.className = "status-dot paused";
        subSmtpBadge.textContent = "Not Configured";
        subSmtpBadge.className = "badge badge-warning";
        subSmtpHost.textContent = "Not configured";
        subSmtpPortSec.textContent = "--";
        subSmtpSender.textContent = "--";
      }

      // Environment & Specs
      envPid.textContent = data.pid;
      envPpid.textContent = data.ppid != null ? data.ppid : "None";
      envPlatform.textContent = `${data.os} (${data.arch})`;
      envCores.textContent = `${data.cpu_cores} Cores`;
      envExePath.textContent = data.exe_path;
      envExePath.title = data.exe_path;
      envWorkDir.textContent = data.current_dir;
      envWorkDir.title = data.current_dir;
      envCmdArgs.textContent =
        data.cmd_args && data.cmd_args.length > 0
          ? data.cmd_args.join(" ")
          : "(none)";
      envAppVersion.textContent = `v${data.app_version}`;
      envStartedTime.textContent = data.started_at
        ? new Date(data.started_at).toLocaleString()
        : "--";

      procLastUpdated.textContent = `Updated: ${new Date().toLocaleTimeString()}`;
    } catch (e) {
      console.error("Process metrics error:", e);
    }
  }

  // Run Subsystem Diagnostics Health Check
  async function runHealthCheck() {
    btnRunHealthCheck.disabled = true;
    const origHtml = btnRunHealthCheck.innerHTML;
    btnRunHealthCheck.innerHTML =
      '<span class="spin">&#9696;</span> Testing...';
    try {
      const rep = await api("/process/health");
      healthOverallBadge.textContent = `Status: ${rep.overall_status}`;
      if (rep.overall_status === "HEALTHY") {
        healthOverallBadge.className = "badge badge-success";
      } else if (rep.overall_status === "WARNING") {
        healthOverallBadge.className = "badge badge-warning";
      } else {
        healthOverallBadge.className = "badge badge-danger";
      }

      healthTableBody.innerHTML = rep.checks
        .map((c) => {
          let badgeClass = "badge-success";
          if (c.status === "WARN") badgeClass = "badge-warning";
          else if (c.status === "DOWN") badgeClass = "badge-danger";

          const latText =
            c.latency_ms > 0 ? `${c.latency_ms.toFixed(2)} ms` : "&mdash;";
          return `
          <tr>
            <td><strong>${escapeHtml(c.name)}</strong></td>
            <td><span class="badge ${badgeClass}">${escapeHtml(c.status)}</span></td>
            <td><code>${latText}</code></td>
            <td>${escapeHtml(c.message)}</td>
          </tr>
        `;
        })
        .join("");
      showAlert(
        `Subsystem health diagnostics complete: ${rep.overall_status}`,
        rep.overall_status === "HEALTHY" ? "success" : "info",
      );
    } catch (e) {
      showAlert(`Health check failed: ${e.message}`, "error");
    } finally {
      btnRunHealthCheck.disabled = false;
      btnRunHealthCheck.innerHTML = origHtml;
    }
  }

  // Process Controls Event Listeners
  btnRefreshProcess.addEventListener("click", () => loadProcessMetrics());
  btnRunHealthCheck.addEventListener("click", () => runHealthCheck());
  btnToggleProcAutoRefresh.addEventListener("click", () => {
    isProcAutoRefresh = !isProcAutoRefresh;
    procAutoRefreshLabel.textContent = isProcAutoRefresh ? "ON (3s)" : "OFF";
    procAutoRefreshLabel.style.color = isProcAutoRefresh
      ? "inherit"
      : "var(--danger)";
  });

  function escapeHtml(str) {
    if (!str) return "";
    return String(str)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  // Initialize
  loadConfig();
  loadStatus();
  loadActivity();

  // Periodic Polling for Status and Activity
  statusPollTimer = setInterval(() => {
    loadStatus();
    if (currentTab === "dashboard") {
      loadActivity();
    } else if (currentTab === "process" && isProcAutoRefresh) {
      loadProcessMetrics();
    }
  }, 3000);
})();
