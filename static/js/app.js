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
  const btnTestSmtp = document.getElementById("btnTestSmtp");
  const btnTestForwarding = document.getElementById("btnTestForwarding");
  const formForwarding = document.getElementById("formForwarding");

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
    mailbox: {
      title: "Email Browser",
      sub: "Browse, inspect, and forward emails directly from your Exchange mailbox.",
    },
    process: {
      title: "Process Monitor",
      sub: "Real-time telemetry, memory usage, worker threads, and process health.",
    },
    exchange: {
      title: "Exchange Mailbox",
      sub: "Configure your internal MS Exchange mailbox for inbound retrieval and outbound forwarding.",
    },
    forwarding: {
      title: "Forwarding Rules",
      sub: "Specify destination Gmail accounts and forwarding appearance.",
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
    } else if (tabId === "mailbox") {
      loadMailboxFolders();
      loadMailboxMessages();
    }
  }


  navItems.forEach((item) => {
    item.addEventListener("click", () => switchTab(item.dataset.tab));
  });


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
    const token = localStorage.getItem("auth_token");
    const headers = {
      "Content-Type": "application/json",
      ...(options.headers || {}),
    };
    if (token) {
      headers["Authorization"] = `Bearer ${token}`;
    }

    const res = await fetch(`/api${path}`, {
      ...options,
      headers,
    });

    if (!res.ok) {
      if (res.status === 401 && path !== "/auth/status" && path !== "/auth/login") {
        localStorage.removeItem("auth_token");
        showAuthScreen();
      }
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
        document.getElementById("exchangeSmtpPort").value =
          config.exchange.smtp_port || 587;
        document.getElementById("exchangeSmtpSecurity").value =
          config.exchange.smtp_security || "starttls";
        document.getElementById("exchangeFromEmail").value =
          config.exchange.from_email || "";
        document.getElementById("exchangeFromName").value =
          config.exchange.from_name || "Exchange Email Forwarder";
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
      smtp_port: parseInt(document.getElementById("exchangeSmtpPort").value, 10) || 587,
      smtp_security: document.getElementById("exchangeSmtpSecurity").value,
      from_email: document.getElementById("exchangeFromEmail").value.trim() || null,
      from_name: document.getElementById("exchangeFromName").value.trim() || "Exchange Email Forwarder",
    };
    try {
      await api("/config/exchange", {
        method: "POST",
        body: JSON.stringify(data),
      });
      showAlert("Exchange mailbox & outbound settings saved successfully.", "success");
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
        "<span>&#128269;</span> Test Mailbox Access";
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

  // Send Test Email Out via Exchange Mailbox
  async function sendTestEmail(btn) {
    if (btn) {
      btn.disabled = true;
      btn.innerHTML = '<span class="spin">&#9696;</span> Sending test email...';
    }
    try {
      const res = await api("/test/smtp", {
        method: "POST",
        body: JSON.stringify({}),
      });
      showAlert(
        `Test email successfully sent to ${res.recipient}! Check your Gmail inbox.`,
        "success",
        8000,
      );
    } catch (err) {
      showAlert(`Outbound Test Failed: ${err.message}`, "error", 10000);
    } finally {
      if (btn) {
        btn.disabled = false;
        btn.innerHTML = "<span>&#9993;</span> Send Test Email to Target Gmail";
      }
    }
  }

  if (btnTestSmtp) {
    btnTestSmtp.addEventListener("click", () => sendTestEmail(btnTestSmtp));
  }
  if (btnTestForwarding) {
    btnTestForwarding.addEventListener("click", () => sendTestEmail(btnTestForwarding));
  }

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

      if (cfg.exchange && cfg.exchange.host) {
        subSmtpDot.className = "status-dot active";
        subSmtpBadge.textContent = "Configured";
        subSmtpBadge.className = "badge badge-success";
        const smtpPort = cfg.exchange.smtp_port || 587;
        const smtpSec = (cfg.exchange.smtp_security || "starttls").toUpperCase();
        subSmtpHost.textContent = `${cfg.exchange.host}:${smtpPort}`;
        subSmtpPortSec.textContent = `${smtpPort} (${smtpSec})`;
        subSmtpSender.textContent = cfg.exchange.from_email || cfg.exchange.username || "--";
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

  // Mailbox Browser State & Elements
  let mailboxCurrentFolder = "INBOX";
  let mailboxFilter = "all";
  let mailboxPage = 1;
  let mailboxLimit = 20;
  let mailboxSearch = "";
  let mailboxSearchTimer = null;
  let currentViewingUid = null;
  let currentViewingMessage = null;

  const mailboxFolderSelect = document.getElementById("mailboxFolderSelect");
  const mailboxFilterPills = document.querySelectorAll("#mailboxFilterPills .pill-btn");
  const mailboxSearchInput = document.getElementById("mailboxSearchInput");
  const btnMailboxSearchClear = document.getElementById("btnMailboxSearchClear");
  const btnRefreshMailbox = document.getElementById("btnRefreshMailbox");
  const mailboxCountBadge = document.getElementById("mailboxCountBadge");
  const mailboxTableBody = document.getElementById("mailboxTableBody");
  const mailboxPaginationInfo = document.getElementById("mailboxPaginationInfo");
  const mailboxPageLimit = document.getElementById("mailboxPageLimit");
  const btnMailboxPrevPage = document.getElementById("btnMailboxPrevPage");
  const btnMailboxNextPage = document.getElementById("btnMailboxNextPage");
  const mailboxCurrentPageLabel = document.getElementById("mailboxCurrentPageLabel");

  const mailboxListView = document.getElementById("mailboxListView");
  const mailboxReaderView = document.getElementById("mailboxReaderView");
  const btnBackToMailboxList = document.getElementById("btnBackToMailboxList");
  const btnReaderForward = document.getElementById("btnReaderForward");
  const btnReaderToggleRead = document.getElementById("btnReaderToggleRead");
  const btnReaderToggleReadText = document.getElementById("btnReaderToggleReadText");
  const btnReaderDownloadRaw = document.getElementById("btnReaderDownloadRaw");

  const readerSubject = document.getElementById("readerSubject");
  const readerSeenBadge = document.getElementById("readerSeenBadge");
  const readerForwardedBadge = document.getElementById("readerForwardedBadge");
  const readerFrom = document.getElementById("readerFrom");
  const readerTo = document.getElementById("readerTo");
  const readerCc = document.getElementById("readerCc");
  const readerCcRow = document.getElementById("readerCcRow");
  const readerDate = document.getElementById("readerDate");
  const readerMsgId = document.getElementById("readerMsgId");

  const readerAttachmentsBox = document.getElementById("readerAttachmentsBox");
  const readerAttachmentCount = document.getElementById("readerAttachmentCount");
  const readerAttachmentsList = document.getElementById("readerAttachmentsList");

  const readerBodyTabs = document.querySelectorAll(".body-tab-btn");
  const readerPaneHtml = document.getElementById("readerPaneHtml");
  const readerPaneText = document.getElementById("readerPaneText");
  const readerPaneHeaders = document.getElementById("readerPaneHeaders");
  const readerHtmlFrame = document.getElementById("readerHtmlFrame");
  const readerTextContent = document.getElementById("readerTextContent");
  const readerHeadersContent = document.getElementById("readerHeadersContent");

  async function loadMailboxFolders() {
    try {
      const res = await api("/mailbox/folders");
      if (res.folders && res.folders.length > 0) {
        mailboxFolderSelect.innerHTML = res.folders
          .map((f) => `<option value="${escapeHtml(f)}" ${f === mailboxCurrentFolder ? "selected" : ""}>${escapeHtml(f)}</option>`)
          .join("");
        if (!res.folders.includes(mailboxCurrentFolder)) {
          mailboxCurrentFolder = res.current_folder || res.folders[0];
        }
      }
    } catch (err) {
      console.error("Mailbox folders error:", err);
    }
  }

  async function loadMailboxMessages() {
    if (!mailboxTableBody) return;
    mailboxTableBody.innerHTML = `<tr><td colspan="7" class="text-center empty-state"><span class="spin">&#9696;</span> Fetching emails from Exchange server...</td></tr>`;
    try {
      const qParam = mailboxSearch ? `&q=${encodeURIComponent(mailboxSearch)}` : "";
      const res = await api(
        `/mailbox/messages?folder=${encodeURIComponent(mailboxCurrentFolder)}&filter=${mailboxFilter}&page=${mailboxPage}&limit=${mailboxLimit}${qParam}`
      );

      mailboxCountBadge.textContent = `${res.total} email${res.total === 1 ? "" : "s"}`;

      if (!res.messages || res.messages.length === 0) {
        if (mailboxSearch) {
          mailboxTableBody.innerHTML = `<tr><td colspan="7" class="text-center empty-state">No emails matching "<strong>${escapeHtml(mailboxSearch)}</strong>".</td></tr>`;
        } else {
          mailboxTableBody.innerHTML = `<tr><td colspan="7" class="text-center empty-state">No emails found in <strong>${escapeHtml(mailboxCurrentFolder)}</strong>.</td></tr>`;
        }
        mailboxPaginationInfo.textContent = "Showing 0 of 0 emails";
        mailboxCurrentPageLabel.textContent = "Page 1 of 1";
        btnMailboxPrevPage.disabled = true;
        btnMailboxNextPage.disabled = true;
        return;
      }

      const startItem = (res.page - 1) * res.limit + 1;
      const endItem = Math.min(res.total, res.page * res.limit);
      mailboxPaginationInfo.textContent = `Showing ${startItem}-${endItem} of ${res.total} emails`;
      mailboxCurrentPageLabel.textContent = `Page ${res.page} of ${res.total_pages || 1}`;
      btnMailboxPrevPage.disabled = res.page <= 1;
      btnMailboxNextPage.disabled = res.page >= res.total_pages;

      mailboxTableBody.innerHTML = res.messages
        .map((msg) => {
          const isUnread = !msg.is_seen;
          const statusHtml = isUnread
            ? `<span class="read-status-dot unread" title="Unread"></span><span style="color:var(--primary); font-weight:600;">Unread</span>`
            : `<span class="read-status-dot read" title="Read"></span><span style="color:var(--text-muted);">Read</span>`;

          const fwdHtml = msg.is_forwarded
            ? `<span class="badge badge-success" title="Forwarded to target Gmail">&#10003; Forwarded</span>`
            : `<span class="badge badge-neutral" title="Pending forwarding">Pending</span>`;

          const attHtml = msg.has_attachments
            ? `<span class="att-icon" title="Has Attachments">&#128206;</span>`
            : "";

          return `
            <tr class="mailbox-table-row ${isUnread ? "unread" : ""}" data-uid="${msg.uid}">
              <td>${statusHtml}</td>
              <td>${fwdHtml}</td>
              <td class="cell-sender" title="${escapeHtml(msg.from)}"><strong>${escapeHtml(msg.from || "Unknown")}</strong></td>
              <td class="cell-subject" title="${escapeHtml(msg.subject)}">${escapeHtml(msg.subject || "(No Subject)")}${attHtml}</td>
              <td>${escapeHtml(msg.date)}</td>
              <td><span style="font-size:12px; color:var(--text-muted);">${escapeHtml(msg.size_formatted)}</span></td>
              <td>
                <div style="display:flex; gap:6px; align-items:center;">
                  <button type="button" class="btn btn-sm btn-outline btn-msg-view" data-uid="${msg.uid}" title="View Email">View</button>
                  <button type="button" class="btn btn-sm btn-outline btn-msg-fwd" data-uid="${msg.uid}" title="Forward to Gmail Now">&#10148;</button>
                  <button type="button" class="btn btn-sm btn-outline btn-msg-read" data-uid="${msg.uid}" data-seen="${msg.is_seen}" title="${isUnread ? "Mark as Read" : "Mark as Unread"}">${isUnread ? "&#9993;" : "&#9992;"}</button>
                </div>
              </td>
            </tr>
          `;
        })
        .join("");

      mailboxTableBody.querySelectorAll(".mailbox-table-row").forEach((row) => {
        row.addEventListener("click", (e) => {
          if (e.target.closest("button")) return;
          const uid = parseInt(row.dataset.uid, 10);
          openMailboxMessage(uid);
        });
      });

      mailboxTableBody.querySelectorAll(".btn-msg-view").forEach((btn) => {
        btn.addEventListener("click", (e) => {
          e.stopPropagation();
          openMailboxMessage(parseInt(btn.dataset.uid, 10));
        });
      });

      mailboxTableBody.querySelectorAll(".btn-msg-fwd").forEach((btn) => {
        btn.addEventListener("click", (e) => {
          e.stopPropagation();
          forwardMailboxEmail(parseInt(btn.dataset.uid, 10), btn);
        });
      });

      mailboxTableBody.querySelectorAll(".btn-msg-read").forEach((btn) => {
        btn.addEventListener("click", (e) => {
          e.stopPropagation();
          const uid = parseInt(btn.dataset.uid, 10);
          const currentSeen = btn.dataset.seen === "true";
          toggleMailboxReadStatus(uid, !currentSeen);
        });
      });
    } catch (err) {
      console.error("Mailbox messages error:", err);
      mailboxTableBody.innerHTML = `
        <tr>
          <td colspan="7" class="text-center empty-state" style="color:var(--danger); padding:30px;">
            <p><strong>Failed to load emails:</strong> ${escapeHtml(err.message)}</p>
            <div style="margin-top:12px; display:flex; gap:8px; justify-content:center;">
              <button type="button" class="btn btn-sm btn-outline" id="btnRetryMailbox">Retry</button>
              <button type="button" class="btn btn-sm btn-primary" id="btnGoToExchangeConfig">Configure Exchange Settings</button>
            </div>
          </td>
        </tr>
      `;
      const btnRetry = document.getElementById("btnRetryMailbox");
      if (btnRetry) btnRetry.addEventListener("click", () => loadMailboxMessages());
      const btnGo = document.getElementById("btnGoToExchangeConfig");
      if (btnGo) {
        btnGo.addEventListener("click", () => {
          switchTab("exchange");
        });
      }
    }
  }

  async function openMailboxMessage(uid) {
    currentViewingUid = uid;
    mailboxListView.classList.add("hidden");
    mailboxReaderView.classList.remove("hidden");

    readerSubject.textContent = "Loading email details...";
    readerFrom.textContent = "--";
    readerTo.textContent = "--";
    readerCc.textContent = "--";
    readerCcRow.classList.add("hidden");
    readerDate.textContent = "--";
    readerMsgId.textContent = "--";
    readerAttachmentsBox.classList.add("hidden");
    readerAttachmentsList.innerHTML = "";
    readerHtmlFrame.srcdoc = "<html><body style='font-family:sans-serif; color:#64748b; padding:20px;'>Loading body...</body></html>";
    readerTextContent.textContent = "Loading...";
    readerHeadersContent.textContent = "Loading...";

    try {
      const msg = await api(
        `/mailbox/message?folder=${encodeURIComponent(mailboxCurrentFolder)}&uid=${uid}`
      );
      currentViewingMessage = msg;

      readerSubject.textContent = msg.subject || "(No Subject)";
      readerFrom.textContent = msg.from || "Unknown";
      readerTo.textContent = msg.to || "Unknown";
      if (msg.cc && msg.cc.trim()) {
        readerCc.textContent = msg.cc;
        readerCcRow.classList.remove("hidden");
      } else {
        readerCcRow.classList.add("hidden");
      }
      readerDate.textContent = msg.date;
      readerMsgId.textContent = msg.message_id || "N/A";

      readerSeenBadge.textContent = msg.is_seen ? "Read" : "Unread";
      readerSeenBadge.className = msg.is_seen ? "badge badge-neutral" : "badge badge-primary";
      btnReaderToggleReadText.textContent = msg.is_seen ? "Mark as Unread" : "Mark as Read";

      readerForwardedBadge.textContent = msg.is_forwarded ? "Forwarded" : "Not Forwarded";
      readerForwardedBadge.className = msg.is_forwarded ? "badge badge-success" : "badge badge-neutral";

      btnReaderDownloadRaw.href = `/api/mailbox/raw?folder=${encodeURIComponent(mailboxCurrentFolder)}&uid=${uid}`;

      if (msg.attachments && msg.attachments.length > 0) {
        readerAttachmentsBox.classList.remove("hidden");
        readerAttachmentCount.textContent = msg.attachments.length;
        readerAttachmentsList.innerHTML = msg.attachments
          .map(
            (att) => `
          <div class="attachment-chip">
            <span style="font-size:14px;">&#128206;</span>
            <span class="att-name" title="${escapeHtml(att.filename)}">${escapeHtml(att.filename)}</span>
            <span class="att-size">(${escapeHtml(att.size_formatted)})</span>
            <a class="att-dl-btn" href="/api/mailbox/attachment?folder=${encodeURIComponent(mailboxCurrentFolder)}&uid=${uid}&index=${att.index}" download="${escapeHtml(att.filename)}">Download</a>
          </div>
        `
          )
          .join("");
      } else {
        readerAttachmentsBox.classList.add("hidden");
      }

      readerTextContent.textContent = msg.body_text || "(Empty plain text body)";
      readerHeadersContent.textContent = msg.raw_headers || "";

      if (msg.body_html) {
        switchBodyTab("html");
        const safeHtml = `
          <!DOCTYPE html>
          <html>
          <head>
            <meta charset="utf-8">
            <base target="_blank">
            <style>
              body {
                font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
                font-size: 14px;
                line-height: 1.5;
                color: #1e293b;
                margin: 16px;
                word-break: break-word;
              }
              img { max-width: 100%; height: auto; }
              table { max-width: 100%; }
            </style>
          </head>
          <body>${msg.body_html}</body>
          </html>
        `;
        readerHtmlFrame.srcdoc = safeHtml;
      } else {
        switchBodyTab("text");
        readerHtmlFrame.srcdoc = "<html><body style='font-family:sans-serif; color:#64748b; padding:20px;'>(No HTML version available for this email)</body></html>";
      }
    } catch (err) {
      showAlert(`Failed to load email: ${err.message}`, "error");
      closeMailboxReader();
    }
  }

  function closeMailboxReader() {
    mailboxReaderView.classList.add("hidden");
    mailboxListView.classList.remove("hidden");
    currentViewingUid = null;
    currentViewingMessage = null;
    loadMailboxMessages();
  }

  function switchBodyTab(tabName) {
    readerBodyTabs.forEach((btn) => {
      btn.classList.toggle("active", btn.dataset.body === tabName);
    });
    readerPaneHtml.classList.toggle("hidden", tabName !== "html");
    readerPaneText.classList.toggle("hidden", tabName !== "text");
    readerPaneHeaders.classList.toggle("hidden", tabName !== "headers");
  }

  async function forwardMailboxEmail(uid, btn) {
    const origText = btn ? btn.innerHTML : null;
    if (btn) {
      btn.disabled = true;
      btn.innerHTML = '<span class="spin">&#9696;</span>';
    }
    try {
      const res = await api("/mailbox/forward", {
        method: "POST",
        body: JSON.stringify({ folder: mailboxCurrentFolder, uid }),
      });
      showAlert(`Email forwarded successfully to ${res.forwarded_to}!`, "success");
      if (currentViewingUid === uid && currentViewingMessage) {
        currentViewingMessage.is_forwarded = true;
        readerForwardedBadge.textContent = "Forwarded";
        readerForwardedBadge.className = "badge badge-success";
      }
      loadMailboxMessages();
    } catch (err) {
      showAlert(`Forward failed: ${err.message}`, "error");
    } finally {
      if (btn) {
        btn.disabled = false;
        btn.innerHTML = origText;
      }
    }
  }

  async function toggleMailboxReadStatus(uid, seen) {
    try {
      await api("/mailbox/mark-read", {
        method: "POST",
        body: JSON.stringify({ folder: mailboxCurrentFolder, uid, seen }),
      });
      showAlert(`Email marked as ${seen ? "read" : "unread"}.`, "success", 3000);
      if (currentViewingUid === uid && currentViewingMessage) {
        currentViewingMessage.is_seen = seen;
        readerSeenBadge.textContent = seen ? "Read" : "Unread";
        readerSeenBadge.className = seen ? "badge badge-neutral" : "badge badge-primary";
        btnReaderToggleReadText.textContent = seen ? "Mark as Unread" : "Mark as Read";
      }
      loadMailboxMessages();
    } catch (err) {
      showAlert(`Failed to update read status: ${err.message}`, "error");
    }
  }

  // Mailbox Event Listeners
  if (mailboxFolderSelect) {
    mailboxFolderSelect.addEventListener("change", () => {
      mailboxCurrentFolder = mailboxFolderSelect.value;
      mailboxPage = 1;
      loadMailboxMessages();
    });
  }

  mailboxFilterPills.forEach((pill) => {
    pill.addEventListener("click", () => {
      mailboxFilterPills.forEach((p) => p.classList.remove("active"));
      pill.classList.add("active");
      mailboxFilter = pill.dataset.filter;
      mailboxPage = 1;
      loadMailboxMessages();
    });
  });

  if (mailboxSearchInput) {
    mailboxSearchInput.addEventListener("input", () => {
      const val = mailboxSearchInput.value.trim();
      btnMailboxSearchClear.classList.toggle("hidden", val.length === 0);
      clearTimeout(mailboxSearchTimer);
      mailboxSearchTimer = setTimeout(() => {
        mailboxSearch = val;
        mailboxPage = 1;
        loadMailboxMessages();
      }, 400);
    });
  }

  if (btnMailboxSearchClear) {
    btnMailboxSearchClear.addEventListener("click", () => {
      mailboxSearchInput.value = "";
      btnMailboxSearchClear.classList.add("hidden");
      mailboxSearch = "";
      mailboxPage = 1;
      loadMailboxMessages();
    });
  }

  if (btnRefreshMailbox) {
    btnRefreshMailbox.addEventListener("click", () => {
      loadMailboxFolders();
      loadMailboxMessages();
    });
  }

  if (mailboxPageLimit) {
    mailboxPageLimit.addEventListener("change", () => {
      mailboxLimit = parseInt(mailboxPageLimit.value, 10) || 20;
      mailboxPage = 1;
      loadMailboxMessages();
    });
  }

  if (btnMailboxPrevPage) {
    btnMailboxPrevPage.addEventListener("click", () => {
      if (mailboxPage > 1) {
        mailboxPage -= 1;
        loadMailboxMessages();
      }
    });
  }

  if (btnMailboxNextPage) {
    btnMailboxNextPage.addEventListener("click", () => {
      mailboxPage += 1;
      loadMailboxMessages();
    });
  }

  if (btnBackToMailboxList) {
    btnBackToMailboxList.addEventListener("click", () => {
      closeMailboxReader();
    });
  }

  if (btnReaderForward) {
    btnReaderForward.addEventListener("click", () => {
      if (currentViewingUid) {
        forwardMailboxEmail(currentViewingUid, btnReaderForward);
      }
    });
  }

  if (btnReaderToggleRead) {
    btnReaderToggleRead.addEventListener("click", () => {
      if (currentViewingUid && currentViewingMessage) {
        toggleMailboxReadStatus(currentViewingUid, !currentViewingMessage.is_seen);
      }
    });
  }

  readerBodyTabs.forEach((btn) => {
    btn.addEventListener("click", () => {
      switchBodyTab(btn.dataset.body);
    });
  });


  function escapeHtml(str) {
    if (!str) return "";
    return String(str)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  // Authentication Elements & Handlers
  const authScreen = document.getElementById("authScreen");
  const tabSelectExchange = document.getElementById("tabSelectExchange");
  const tabSelectGoogle = document.getElementById("tabSelectGoogle");
  const exchangeLoginForm = document.getElementById("exchangeLoginForm");
  const googleOAuthPane = document.getElementById("googleOAuthPane");
  const formExchangeLogin = document.getElementById("formExchangeLogin");
  const authExchangeHost = document.getElementById("authExchangeHost");
  const authExchangeUsername = document.getElementById("authExchangeUsername");
  const authExchangePassword = document.getElementById("authExchangePassword");
  const authAllowSelfSigned = document.getElementById("authAllowSelfSigned");
  const authAlert = document.getElementById("authAlert");
  const btnSubmitExchangeLogin = document.getElementById("btnSubmitExchangeLogin");
  const btnLoginText = document.getElementById("btnLoginText");
  const btnBackToExchange = document.getElementById("btnBackToExchange");
  const btnBackToExchangeFromActive = document.getElementById("btnBackToExchangeFromActive");
  const btnPreviewGoogleOAuth = document.getElementById("btnPreviewGoogleOAuth");
  const googleConfiguredView = document.getElementById("googleConfiguredView");
  const googleUnconfiguredView = document.getElementById("googleUnconfiguredView");
  const googleTabBadge = document.getElementById("googleTabBadge");
  const userAuthBadge = document.getElementById("userAuthBadge");
  const userAvatar = document.getElementById("userAvatar");
  const userDisplayName = document.getElementById("userDisplayName");
  const authProviderTag = document.getElementById("authProviderTag");
  const btnSignOut = document.getElementById("btnSignOut");

  // Google OAuth2 Slot Form
  const formGoogleOAuthSlot = document.getElementById("formGoogleOAuthSlot");
  const googleClientId = document.getElementById("googleClientId");
  const googleClientSecret = document.getElementById("googleClientSecret");
  const btnTestGoogleSlot = document.getElementById("btnTestGoogleSlot");

  function showAuthScreen() {
    if (authScreen) authScreen.classList.remove("hidden");
    if (userAuthBadge) userAuthBadge.style.display = "none";
    if (statusPollTimer) {
      clearInterval(statusPollTimer);
      statusPollTimer = null;
    }
  }

  function hideAuthScreen() {
    if (authScreen) authScreen.classList.add("hidden");
    if (userAuthBadge) userAuthBadge.style.display = "flex";
  }

  function showAuthAlert(msg, type = "error") {
    if (!authAlert) return;
    authAlert.textContent = msg;
    authAlert.className = `alert-box alert-${type}`;
    authAlert.classList.remove("hidden");
  }

  function hideAuthAlert() {
    if (authAlert) authAlert.classList.add("hidden");
  }

  function switchAuthTab(tab) {
    if (tab === "exchange") {
      tabSelectExchange.classList.add("active");
      tabSelectGoogle.classList.remove("active");
      exchangeLoginForm.classList.remove("hidden");
      googleOAuthPane.classList.add("hidden");
    } else {
      tabSelectGoogle.classList.add("active");
      tabSelectExchange.classList.remove("active");
      googleOAuthPane.classList.remove("hidden");
      exchangeLoginForm.classList.add("hidden");
    }
  }

  if (tabSelectExchange) {
    tabSelectExchange.addEventListener("click", () => switchAuthTab("exchange"));
  }
  if (tabSelectGoogle) {
    tabSelectGoogle.addEventListener("click", () => switchAuthTab("google"));
  }
  if (btnBackToExchange) {
    btnBackToExchange.addEventListener("click", () => switchAuthTab("exchange"));
  }
  if (btnBackToExchangeFromActive) {
    btnBackToExchangeFromActive.addEventListener("click", () => switchAuthTab("exchange"));
  }

  if (btnPreviewGoogleOAuth) {
    btnPreviewGoogleOAuth.addEventListener("click", async () => {
      try {
        const res = await fetch("/api/auth/oauth/google?format=json");
        const data = await res.json();
        if (data.is_configured) {
          showModal(
            "Google OAuth2 Active Status",
            `<div style="font-size: 13px; line-height: 1.6;">
              <p><strong>Status:</strong> <span class="badge badge-success" style="background: rgba(16, 185, 129, 0.2); color: #10b981;">Active & Ready</span></p>
              <p><strong>Provider:</strong> Google Workspace / Gmail OAuth 2.0</p>
              <div style="margin: 12px 0;">
                <strong>Authorization URL:</strong>
                <pre style="background: #0f172a; color: #38bdf8; padding: 12px; border-radius: 6px; font-size: 11px; overflow-x: auto; margin-top: 6px; white-space: pre-wrap; word-break: break-all;">${escapeHtml(data.auth_url || "N/A")}</pre>
              </div>
              <div style="margin-top: 8px;">
                <strong>Authorized Redirect URI:</strong>
                <code style="background: var(--bg-main); padding: 4px 8px; border-radius: 4px; display: inline-block; margin-top: 4px;">${escapeHtml(data.redirect_uri || "/api/auth/oauth/google/callback")}</code>
              </div>
            </div>`
          );
        } else {
          showModal(
            "Google OAuth2 Configuration Guide",
            `<div style="font-size: 13px; line-height: 1.6;">
              <p><strong>Status:</strong> <span class="badge badge-warning">Awaiting .env Configuration</span></p>
              <p style="margin-top: 8px;">Please set the following environment variables in your <code>.env</code> file:</p>
              <div style="background: #0f172a; color: #38bdf8; padding: 12px; border-radius: 6px; font-size: 12px; font-family: monospace; line-height: 1.5; margin: 10px 0;">
                <div>GOOGLE_CLIENT_ID=your_google_client_id</div>
                <div>GOOGLE_CLIENT_SECRET=your_google_client_secret</div>
                <div>GOOGLE_REDIRECT_URI=http://localhost:8080/api/auth/oauth/google/callback</div>
              </div>
              <p style="font-size: 12px; color: var(--text-muted); margin-top: 6px;">Once you add these to <code>.env</code> and restart the app, Google Sign-In will be immediately available.</p>
            </div>`
          );
        }
      } catch (e) {
        showModal("Google OAuth2", `<p>${escapeHtml(e.message)}</p>`);
      }
    });
  }

  // Handle Exchange Login
  if (formExchangeLogin) {
    formExchangeLogin.addEventListener("submit", async (e) => {
      e.preventDefault();
      hideAuthAlert();
      btnLoginText.textContent = "Verifying with Exchange...";
      btnSubmitExchangeLogin.disabled = true;

      const payload = {
        provider: "exchange",
        host: authExchangeHost.value.trim(),
        username: authExchangeUsername.value.trim(),
        password: authExchangePassword.value,
        allow_self_signed: authAllowSelfSigned.checked,
      };

      try {
        const res = await fetch("/api/auth/login", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        });

        const data = await res.json();
        if (!res.ok) {
          throw new Error(data.message || `Login failed (${res.status})`);
        }

        if (data.token) {
          localStorage.setItem("auth_token", data.token);
        }

        hideAuthScreen();
        showAlert("Authentication successful! Welcome to Smart Email Forwarder.", "success");
        initApplicationAfterAuth(data.user);
      } catch (err) {
        showAuthAlert(err.message, "error");
      } finally {
        btnLoginText.textContent = "Sign In with Exchange Mailbox";
        btnSubmitExchangeLogin.disabled = false;
      }
    });
  }

  // Handle Logout
  if (btnSignOut) {
    btnSignOut.addEventListener("click", async () => {
      try {
        await api("/auth/logout", { method: "POST" });
      } catch (_) {}
      localStorage.removeItem("auth_token");
      showAuthScreen();
      showAuthAlert("You have been signed out.", "info");
    });
  }

  // Google OAuth2 Configuration in Settings Tab
  async function loadGoogleOAuthSlotConfig() {
    if (!formGoogleOAuthSlot) return;
    try {
      const data = await api("/auth/oauth/google/config");
      const cfg = data.config || data;
      if (googleClientId && cfg.client_id) {
        googleClientId.value = cfg.client_id;
      }
      if (googleClientSecret && cfg.client_secret) {
        googleClientSecret.placeholder = "••••••••";
      }
      const statusBadge = document.getElementById("googleSettingsStatus");
      if (statusBadge) {
        if (data.is_configured) {
          statusBadge.textContent = data.from_env ? "Configured via .env" : "Configured";
          statusBadge.className = "badge badge-success";
        } else {
          statusBadge.textContent = "Awaiting .env Configuration";
          statusBadge.className = "badge badge-warning";
        }
      }
    } catch (_) {}
  }

  if (formGoogleOAuthSlot) {
    formGoogleOAuthSlot.addEventListener("submit", async (e) => {
      e.preventDefault();
      try {
        const payload = {
          enabled: false,
          client_id: googleClientId.value.trim() || null,
          client_secret: googleClientSecret.value.trim() || null,
          redirect_uri: "/api/auth/oauth/google/callback",
          scopes: ["openid", "email", "profile", "https://mail.google.com/"],
        };
        await api("/auth/oauth/google/config", {
          method: "POST",
          body: JSON.stringify(payload),
        });
        showAlert("Google OAuth2 configuration saved.", "success");
        loadGoogleOAuthSlotConfig();
      } catch (err) {
        showAlert(`Failed to save OAuth config: ${err.message}`, "error");
      }
    });
  }

  if (btnTestGoogleSlot) {
    btnTestGoogleSlot.addEventListener("click", async () => {
      try {
        const res = await fetch("/api/auth/oauth/google?format=json");
        const data = await res.json();
        if (data.is_configured) {
          showModal(
            "Google OAuth2 Test",
            `<div style="font-size: 13px; line-height: 1.6;">
              <p><strong>Status:</strong> <span class="badge badge-success" style="background: rgba(16, 185, 129, 0.2); color: #10b981;">Active & Ready</span></p>
              <p><strong>Provider:</strong> Google Workspace / Gmail OAuth 2.0</p>
              <div style="margin-top: 10px;">
                <strong>Generated Auth URL:</strong>
                <pre style="background: #0f172a; color: #38bdf8; padding: 10px; border-radius: 6px; font-size: 11px; word-break: break-all; white-space: pre-wrap;">${escapeHtml(data.auth_url || "N/A")}</pre>
              </div>
            </div>`
          );
        } else {
          showModal(
            "Google OAuth2 Test",
            `<div style="font-size: 13px; line-height: 1.6;">
              <p><strong>Status:</strong> <span class="badge badge-warning">Credentials Missing</span></p>
              <p style="margin-top: 8px;">${escapeHtml(data.message)}</p>
              <p style="font-size: 12px; color: var(--text-muted); margin-top: 8px;">Please update <code>.env</code> with your Google OAuth 2.0 Web Application credentials.</p>
            </div>`
          );
        }
      } catch (e) {
        showAlert(`Error testing OAuth endpoint: ${e.message}`, "error");
      }
    });
  }

  async function checkAuthStatus() {
    try {
      const res = await fetch("/api/auth/status", {
        headers: {
          "Authorization": `Bearer ${localStorage.getItem("auth_token") || ""}`,
        },
      });
      const data = await res.json();

      if (data.default_host && authExchangeHost) {
        authExchangeHost.value = data.default_host;
      }
      if (data.default_username && authExchangeUsername) {
        authExchangeUsername.value = data.default_username;
      }
      if (authAllowSelfSigned) {
        authAllowSelfSigned.checked = !!data.default_allow_self_signed;
      }
      if (data.exchange_configured) {
        const hintEl = document.getElementById("authHostHint");
        if (hintEl) {
          hintEl.textContent = `Configured Mailbox Server: ${data.default_host || ""} (IMAP/TLS)`;
        }
      }

      if (data.google_configured) {
        if (googleConfiguredView) googleConfiguredView.classList.remove("hidden");
        if (googleUnconfiguredView) googleUnconfiguredView.classList.add("hidden");
        if (googleTabBadge) {
          googleTabBadge.textContent = "Default";
          googleTabBadge.className = "auth-pill auth-pill-active";
        }
      } else {
        if (googleConfiguredView) googleConfiguredView.classList.add("hidden");
        if (googleUnconfiguredView) googleUnconfiguredView.classList.remove("hidden");
        if (googleTabBadge) {
          googleTabBadge.textContent = ".env needed";
          googleTabBadge.className = "auth-pill auth-pill-slot";
        }
      }
      switchAuthTab("google");

      if (data.authenticated && data.user) {
        hideAuthScreen();
        initApplicationAfterAuth(data.user);
      } else {
        showAuthScreen();
        if (data.default_username && authExchangePassword) {
          setTimeout(() => authExchangePassword.focus(), 150);
        }
      }
    } catch (_) {
      showAuthScreen();
    }
  }

  function initApplicationAfterAuth(user) {
    if (userDisplayName && user) {
      userDisplayName.textContent = user.username;
    }
    if (userAvatar && user && user.username) {
      userAvatar.textContent = user.username.substring(0, 2).toUpperCase();
    }
    if (authProviderTag) {
      authProviderTag.textContent = user && user.provider === "google" ? "Google Account" : "Exchange Mailbox";
    }

    loadConfig();
    loadStatus();
    loadActivity();
    loadGoogleOAuthSlotConfig();

    if (!statusPollTimer) {
      statusPollTimer = setInterval(() => {
        loadStatus();
        if (currentTab === "dashboard") {
          loadActivity();
        } else if (currentTab === "process" && isProcAutoRefresh) {
          loadProcessMetrics();
        }
      }, 3000);
    }
  }

  // Initialize Auth
  checkAuthStatus();
})();
