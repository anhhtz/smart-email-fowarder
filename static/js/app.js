// Smart Email Forwarder - 100% Offline Vanilla Frontend Client
(function () {
  'use strict';

  // State
  let currentTab = 'dashboard';
  let statusPollTimer = null;
  let isPollerPaused = false;

  // DOM Elements
  const navItems = document.querySelectorAll('.nav-item');
  const tabPanes = document.querySelectorAll('.tab-pane');
  const pageHeading = document.getElementById('pageHeading');
  const pageSubheading = document.getElementById('pageSubheading');
  const alertBox = document.getElementById('alertBox');

  // Topbar elements
  const btnSyncNow = document.getElementById('btnSyncNow');
  const btnTogglePoller = document.getElementById('btnTogglePoller');
  const togglePollerLabel = document.getElementById('togglePollerLabel');

  // Stats elements
  const statStatus = document.getElementById('statStatus');
  const statBadgeStatus = document.getElementById('statBadgeStatus');
  const statNextPoll = document.getElementById('statNextPoll');
  const statTotalForwarded = document.getElementById('statTotalForwarded');
  const statTotalFailed = document.getElementById('statTotalFailed');
  const statInterval = document.getElementById('statInterval');
  const statLastPoll = document.getElementById('statLastPoll');
  const sidebarStatusDot = document.getElementById('sidebarStatusDot');
  const sidebarStatusText = document.getElementById('sidebarStatusText');

  // Tables & Logs
  const activityTableBody = document.getElementById('activityTableBody');
  const btnRefreshActivity = document.getElementById('btnRefreshActivity');
  const logViewer = document.getElementById('logViewer');
  const btnClearLogs = document.getElementById('btnClearLogs');
  const btnRefreshLogs = document.getElementById('btnRefreshLogs');

  // Forms
  const formExchange = document.getElementById('formExchange');
  const btnTestExchange = document.getElementById('btnTestExchange');
  const formForwarding = document.getElementById('formForwarding');
  const formSmtp = document.getElementById('formSmtp');
  const btnTestSmtp = document.getElementById('btnTestSmtp');
  const smtpAuthRequired = document.getElementById('smtpAuthRequired');
  const smtpAuthFields = document.getElementById('smtpAuthFields');

  // Modal
  const detailModal = document.getElementById('detailModal');
  const btnModalClose = document.getElementById('btnModalClose');
  const modalContent = document.getElementById('modalContent');

  const tabTitles = {
    dashboard: { title: 'Dashboard', sub: 'System health, forward statistics, and recent activity.' },
    exchange: { title: 'Exchange Mailbox', sub: 'Configure the IMAP connection to access your internal MS Exchange mailbox.' },
    forwarding: { title: 'Forwarding Rules', sub: 'Specify destination Gmail accounts and forwarding appearance.' },
    smtp: { title: 'Exchange Relay', sub: 'Configure the outbound SMTP connection to relay emails to Gmail.' },
    logs: { title: 'Activity Logs', sub: 'Real-time operational events and detailed diagnostic logs.' }
  };

  // Helper: Display Alert
  function showAlert(message, type = 'info', duration = 5000) {
    alertBox.textContent = message;
    alertBox.className = `alert-box ${type}`;
    if (duration > 0) {
      setTimeout(() => {
        alertBox.className = 'alert-box hidden';
      }, duration);
    }
  }

  // Tab Navigation
  function switchTab(tabId) {
    currentTab = tabId;
    navItems.forEach(item => {
      item.classList.toggle('active', item.dataset.tab === tabId);
    });
    tabPanes.forEach(pane => {
      pane.classList.toggle('active', pane.id === `tab-${tabId}`);
    });
    if (tabTitles[tabId]) {
      pageHeading.textContent = tabTitles[tabId].title;
      pageSubheading.textContent = tabTitles[tabId].sub;
    }
    if (tabId === 'logs') {
      loadLogs();
    }
  }

  navItems.forEach(item => {
    item.addEventListener('click', () => switchTab(item.dataset.tab));
  });

  // Toggle SMTP Auth Fields
  smtpAuthRequired.addEventListener('change', () => {
    smtpAuthFields.classList.toggle('hidden', !smtpAuthRequired.checked);
  });

  // Modal handlers
  function showModal(title, html) {
    document.getElementById('modalTitle').textContent = title;
    modalContent.innerHTML = html;
    detailModal.classList.remove('hidden');
  }

  btnModalClose.addEventListener('click', () => detailModal.classList.add('hidden'));
  detailModal.querySelector('.modal-backdrop').addEventListener('click', () => detailModal.classList.add('hidden'));

  // API Call Helpers
  async function api(path, options = {}) {
    const res = await fetch(`/api${path}`, {
      headers: { 'Content-Type': 'application/json' },
      ...options
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
      const status = await api('/status');
      
      isPollerPaused = !status.is_polling;
      togglePollerLabel.textContent = isPollerPaused ? 'Resume Poller' : 'Pause Poller';
      
      statStatus.textContent = isPollerPaused ? 'Paused' : (status.is_busy ? 'Syncing...' : 'Active');
      statBadgeStatus.textContent = isPollerPaused ? 'Paused' : 'Active';
      statBadgeStatus.className = isPollerPaused ? 'badge badge-warning' : 'badge badge-success';
      
      sidebarStatusDot.className = `status-dot ${isPollerPaused ? 'paused' : 'active'}`;
      sidebarStatusText.textContent = isPollerPaused ? 'Service Paused' : 'Service Active';

      statTotalForwarded.textContent = status.total_forwarded || 0;
      statTotalFailed.textContent = status.total_failed || 0;
      statInterval.textContent = `${status.poll_interval_minutes || 1} min`;
      statLastPoll.textContent = status.last_poll_time ? new Date(status.last_poll_time).toLocaleTimeString() : 'Never';
      
      if (status.next_poll_seconds != null) {
        if (isPollerPaused) {
          statNextPoll.textContent = 'Paused';
        } else if (status.next_poll_seconds <= 0) {
          statNextPoll.textContent = 'Polling now...';
        } else {
          statNextPoll.textContent = `In ${status.next_poll_seconds}s`;
        }
      }
    } catch (e) {
      console.error('Status load error:', e);
    }
  }

  // Load Configurations
  async function loadConfig() {
    try {
      const config = await api('/config');
      
      // Exchange Form
      if (config.exchange) {
        document.getElementById('exchangeHost').value = config.exchange.host || '';
        document.getElementById('exchangePort').value = config.exchange.port || 993;
        document.getElementById('exchangeSecurity').value = config.exchange.security || 'ssl';
        document.getElementById('exchangeUsername').value = config.exchange.username || '';
        document.getElementById('exchangePassword').value = '';
        document.getElementById('exchangeFolder').value = config.exchange.folder || 'INBOX';
        document.getElementById('exchangeMarkRead').checked = config.exchange.mark_as_read !== false;
        document.getElementById('exchangeAllowSelfSigned').checked = !!config.exchange.allow_self_signed;
      }

      // Forwarding Form
      if (config.forwarding) {
        document.getElementById('targetEmails').value = (config.forwarding.target_emails || []).join(', ');
        document.getElementById('subjectPrefix').value = config.forwarding.subject_prefix || '';
        document.getElementById('pollInterval').value = config.forwarding.poll_interval_minutes || 1;
        document.getElementById('includeBanner').checked = config.forwarding.include_banner !== false;
        document.getElementById('preserveReplyTo').checked = config.forwarding.preserve_reply_to !== false;
      }

      // SMTP Relay Form
      if (config.smtp) {
        document.getElementById('smtpHost').value = config.smtp.host || '';
        document.getElementById('smtpPort').value = config.smtp.port || 25;
        document.getElementById('smtpSecurity').value = config.smtp.security || 'plain';
        document.getElementById('smtpFromEmail').value = config.smtp.from_email || '';
        document.getElementById('smtpFromName').value = config.smtp.from_name || 'Exchange Email Forwarder';
        smtpAuthRequired.checked = !!config.smtp.auth_required;
        smtpAuthFields.classList.toggle('hidden', !config.smtp.auth_required);
        document.getElementById('smtpUsername').value = config.smtp.username || '';
        document.getElementById('smtpPassword').value = '';
      }
    } catch (e) {
      showAlert(`Failed to load configuration: ${e.message}`, 'error');
    }
  }

  // Load Recent Forward Activity
  async function loadActivity() {
    try {
      const logs = await api('/logs?limit=25');
      if (!logs || logs.length === 0) {
        activityTableBody.innerHTML = `<tr><td colspan="6" class="text-center empty-state">No email forwarding events recorded yet.</td></tr>`;
        return;
      }

      activityTableBody.innerHTML = logs.map(log => {
        const dateStr = new Date(log.timestamp).toLocaleString();
        const badgeClass = log.status === 'SUCCESS' ? 'badge-success' : 'badge-danger';
        return `
          <tr>
            <td>${escapeHtml(dateStr)}</td>
            <td><strong>${escapeHtml(log.original_sender || 'Unknown')}</strong></td>
            <td>${escapeHtml(log.subject || '(No Subject)')}</td>
            <td><code>${escapeHtml(log.forwarded_to || '')}</code></td>
            <td><span class="badge ${badgeClass}">${escapeHtml(log.status)}</span></td>
            <td><button class="btn btn-sm btn-outline btn-view-log" data-log='${JSON.stringify(log)}'>View</button></td>
          </tr>
        `;
      }).join('');

      document.querySelectorAll('.btn-view-log').forEach(btn => {
        btn.addEventListener('click', () => {
          const log = JSON.parse(btn.dataset.log);
          showModal('Forwarding Details', `
            <p><strong>Time:</strong> ${new Date(log.timestamp).toLocaleString()}</p>
            <p><strong>Message ID:</strong> <code>${escapeHtml(log.message_id || 'N/A')}</code></p>
            <p><strong>Sender:</strong> ${escapeHtml(log.original_sender || 'N/A')}</p>
            <p><strong>Subject:</strong> ${escapeHtml(log.subject || 'N/A')}</p>
            <p><strong>Recipients:</strong> ${escapeHtml(log.forwarded_to || 'N/A')}</p>
            <p><strong>Status:</strong> <span class="badge ${log.status === 'SUCCESS' ? 'badge-success' : 'badge-danger'}">${log.status}</span></p>
            ${log.error_details ? `<div style="margin-top:10px; color:var(--danger)"><strong>Error Details:</strong><pre style="background:#fee2e2; padding:10px; border-radius:4px; overflow:auto;">${escapeHtml(log.error_details)}</pre></div>` : ''}
          `);
        });
      });
    } catch (e) {
      console.error('Activity load error:', e);
    }
  }

  // Load System Event Logs
  async function loadLogs() {
    try {
      const logs = await api('/system-logs');
      if (logs && logs.length > 0) {
        logViewer.innerHTML = logs.map(entry => {
          const typeClass = entry.level.toLowerCase();
          return `<div class="log-entry ${typeClass}">[${new Date(entry.time).toLocaleTimeString()}] [${entry.level}] ${escapeHtml(entry.message)}</div>`;
        }).join('');
        logViewer.scrollTop = logViewer.scrollHeight;
      }
    } catch (e) {
      console.error('Log load error:', e);
    }
  }

  // Save Exchange Settings
  formExchange.addEventListener('submit', async (e) => {
    e.preventDefault();
    const data = {
      host: document.getElementById('exchangeHost').value.trim(),
      port: parseInt(document.getElementById('exchangePort').value, 10),
      security: document.getElementById('exchangeSecurity').value,
      username: document.getElementById('exchangeUsername').value.trim(),
      password: document.getElementById('exchangePassword').value,
      folder: document.getElementById('exchangeFolder').value.trim(),
      mark_as_read: document.getElementById('exchangeMarkRead').checked,
      allow_self_signed: document.getElementById('exchangeAllowSelfSigned').checked
    };
    try {
      await api('/config/exchange', { method: 'POST', body: JSON.stringify(data) });
      showAlert('Exchange settings saved successfully.', 'success');
      loadStatus();
    } catch (err) {
      showAlert(`Error saving Exchange settings: ${err.message}`, 'error');
    }
  });

  // Test Exchange Connection
  btnTestExchange.addEventListener('click', async () => {
    btnTestExchange.disabled = true;
    btnTestExchange.innerHTML = '<span class="spin">&#9696;</span> Testing connection...';
    try {
      const data = {
        host: document.getElementById('exchangeHost').value.trim(),
        port: parseInt(document.getElementById('exchangePort').value, 10),
        security: document.getElementById('exchangeSecurity').value,
        username: document.getElementById('exchangeUsername').value.trim(),
        password: document.getElementById('exchangePassword').value,
        folder: document.getElementById('exchangeFolder').value.trim(),
        allow_self_signed: document.getElementById('exchangeAllowSelfSigned').checked
      };
      const res = await api('/test/exchange', { method: 'POST', body: JSON.stringify(data) });
      showAlert(`Exchange Connection Success! Found ${res.message_count} total messages (${res.unseen_count} unread) in ${data.folder}.`, 'success', 8000);
    } catch (err) {
      showAlert(`Exchange Connection Failed: ${err.message}`, 'error', 10000);
    } finally {
      btnTestExchange.disabled = false;
      btnTestExchange.innerHTML = '<span>&#128269;</span> Test Exchange Connection';
    }
  });

  // Save Forwarding Rules
  formForwarding.addEventListener('submit', async (e) => {
    e.preventDefault();
    const emails = document.getElementById('targetEmails').value
      .split(/[\n,]+/)
      .map(s => s.trim())
      .filter(Boolean);
    const data = {
      target_emails: emails,
      subject_prefix: document.getElementById('subjectPrefix').value,
      poll_interval_minutes: parseInt(document.getElementById('pollInterval').value, 10),
      include_banner: document.getElementById('includeBanner').checked,
      preserve_reply_to: document.getElementById('preserveReplyTo').checked
    };
    try {
      await api('/config/forwarding', { method: 'POST', body: JSON.stringify(data) });
      showAlert('Forwarding rules saved successfully.', 'success');
      loadStatus();
    } catch (err) {
      showAlert(`Error saving forwarding rules: ${err.message}`, 'error');
    }
  });

  // Save SMTP Relay Settings
  formSmtp.addEventListener('submit', async (e) => {
    e.preventDefault();
    const data = {
      host: document.getElementById('smtpHost').value.trim(),
      port: parseInt(document.getElementById('smtpPort').value, 10),
      security: document.getElementById('smtpSecurity').value,
      from_email: document.getElementById('smtpFromEmail').value.trim(),
      from_name: document.getElementById('smtpFromName').value.trim(),
      auth_required: smtpAuthRequired.checked,
      username: document.getElementById('smtpUsername').value.trim(),
      password: document.getElementById('smtpPassword').value
    };
    try {
      await api('/config/smtp', { method: 'POST', body: JSON.stringify(data) });
      showAlert('Exchange SMTP relay settings saved successfully.', 'success');
      loadStatus();
    } catch (err) {
      showAlert(`Error saving relay settings: ${err.message}`, 'error');
    }
  });

  // Test SMTP Relay (Send Test Email)
  btnTestSmtp.addEventListener('click', async () => {
    btnTestSmtp.disabled = true;
    btnTestSmtp.innerHTML = '<span class="spin">&#9696;</span> Sending test email...';
    try {
      const data = {
        host: document.getElementById('smtpHost').value.trim(),
        port: parseInt(document.getElementById('smtpPort').value, 10),
        security: document.getElementById('smtpSecurity').value,
        from_email: document.getElementById('smtpFromEmail').value.trim(),
        from_name: document.getElementById('smtpFromName').value.trim(),
        auth_required: smtpAuthRequired.checked,
        username: document.getElementById('smtpUsername').value.trim(),
        password: document.getElementById('smtpPassword').value
      };
      const res = await api('/test/smtp', { method: 'POST', body: JSON.stringify(data) });
      showAlert(`Test email successfully relayed to ${res.recipient}! Check your Gmail inbox.`, 'success', 8000);
    } catch (err) {
      showAlert(`Relay Test Failed: ${err.message}`, 'error', 10000);
    } finally {
      btnTestSmtp.disabled = false;
      btnTestSmtp.innerHTML = '<span>&#9993;</span> Send Test Email to Target Gmail';
    }
  });

  // Sync Now Button
  btnSyncNow.addEventListener('click', async () => {
    btnSyncNow.disabled = true;
    const originalText = btnSyncNow.innerHTML;
    btnSyncNow.innerHTML = '<span class="spin">&#9696;</span> Syncing...';
    try {
      const res = await api('/sync', { method: 'POST' });
      showAlert(`Sync completed: ${res.forwarded_count} emails forwarded, ${res.failed_count} failed.`, 'success');
      loadStatus();
      loadActivity();
    } catch (err) {
      showAlert(`Sync failed: ${err.message}`, 'error');
    } finally {
      btnSyncNow.disabled = false;
      btnSyncNow.innerHTML = originalText;
    }
  });

  // Toggle Poller Pause/Resume
  btnTogglePoller.addEventListener('click', async () => {
    try {
      const action = isPollerPaused ? 'resume' : 'pause';
      await api(`/poller/${action}`, { method: 'POST' });
      loadStatus();
    } catch (err) {
      showAlert(`Failed to toggle poller: ${err.message}`, 'error');
    }
  });

  // Refresh Activity / Logs
  btnRefreshActivity.addEventListener('click', () => loadActivity());
  btnRefreshLogs.addEventListener('click', () => loadLogs());
  btnClearLogs.addEventListener('click', () => {
    logViewer.innerHTML = '<div class="log-entry info">[System] Log display cleared.</div>';
  });

  function escapeHtml(str) {
    if (!str) return '';
    return String(str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  // Initialize
  loadConfig();
  loadStatus();
  loadActivity();

  // Periodic Polling for Status and Activity
  statusPollTimer = setInterval(() => {
    loadStatus();
    if (currentTab === 'dashboard') {
      loadActivity();
    }
  }, 5000);

})();
