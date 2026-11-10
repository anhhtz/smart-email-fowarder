// docs.js - Interactive behaviors for Smart Email Forwarder documentation

(function () {
  'use strict';

  // 1. Theme Toggle Management
  const themeToggle = document.getElementById('theme-toggle');
  const storedTheme = localStorage.getItem('sef-docs-theme');
  const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;

  function applyTheme(theme) {
    if (theme === 'dark') {
      document.documentElement.setAttribute('data-theme', 'dark');
    } else {
      document.documentElement.removeAttribute('data-theme');
    }
    if (themeToggle) {
      themeToggle.setAttribute('aria-label', theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode');
      themeToggle.innerHTML = theme === 'dark'
        ? '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>'
        : '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>';
    }
  }

  const initialTheme = storedTheme || (systemPrefersDark ? 'dark' : 'light');
  applyTheme(initialTheme);

  if (themeToggle) {
    themeToggle.addEventListener('click', () => {
      const current = document.documentElement.getAttribute('data-theme') === 'dark' ? 'dark' : 'light';
      const next = current === 'dark' ? 'light' : 'dark';
      localStorage.setItem('sef-docs-theme', next);
      applyTheme(next);
    });
  }

  // 2. Mobile Navigation Toggle
  const mobileNavToggle = document.getElementById('mobile-nav-toggle');
  const sidebar = document.querySelector('.docs-sidebar');

  if (mobileNavToggle && sidebar) {
    mobileNavToggle.addEventListener('click', (e) => {
      e.stopPropagation();
      sidebar.classList.toggle('open');
    });

    document.addEventListener('click', (e) => {
      if (sidebar.classList.contains('open') && !sidebar.contains(e.target) && !mobileNavToggle.contains(e.target)) {
        sidebar.classList.remove('open');
      }
    });
  }

  // 3. Tab Switching
  document.querySelectorAll('.tabs-container').forEach(container => {
    const buttons = container.querySelectorAll('.tab-btn');
    const panes = container.querySelectorAll('.tab-pane');

    buttons.forEach(btn => {
      btn.addEventListener('click', () => {
        const targetTab = btn.getAttribute('data-tab');

        buttons.forEach(b => b.classList.remove('active'));
        panes.forEach(p => p.classList.remove('active'));

        btn.classList.add('active');
        const targetPane = container.querySelector(`.tab-pane[data-tab="${targetTab}"]`);
        if (targetPane) targetPane.classList.add('active');
      });
    });
  });

  // 4. Code Block Copy Buttons
  document.querySelectorAll('pre code').forEach(codeBlock => {
    const pre = codeBlock.parentElement;
    if (pre && !pre.closest('.code-wrapper')) {
      const wrapper = document.createElement('div');
      wrapper.className = 'code-wrapper';
      
      const header = document.createElement('div');
      header.className = 'code-header';
      
      const langMatch = codeBlock.className.match(/language-([a-z0-9_-]+)/i);
      const lang = langMatch ? langMatch[1].toUpperCase() : 'SNIPPET';
      
      header.innerHTML = `<span>${lang}</span><button class="copy-btn" type="button">Copy</button>`;
      
      pre.parentNode.insertBefore(wrapper, pre);
      wrapper.appendChild(header);
      wrapper.appendChild(pre);

      const copyBtn = header.querySelector('.copy-btn');
      copyBtn.addEventListener('click', async () => {
        const text = codeBlock.innerText;
        try {
          await navigator.clipboard.writeText(text);
          copyBtn.innerText = 'Copied!';
          copyBtn.style.color = '#10b981';
          setTimeout(() => {
            copyBtn.innerText = 'Copy';
            copyBtn.style.color = '';
          }, 2000);
        } catch (err) {
          copyBtn.innerText = 'Failed';
          setTimeout(() => { copyBtn.innerText = 'Copy'; }, 2000);
        }
      });
    }
  });

  // 5. Active Link Highlight
  const currentPath = window.location.pathname.split('/').pop() || 'index.html';
  document.querySelectorAll('.sidebar-link').forEach(link => {
    const href = link.getAttribute('href');
    if (href === currentPath || (currentPath === '' && href === 'index.html')) {
      link.classList.add('active');
    }
  });

  // 6. Table of Contents Scrollspy
  const tocLinks = document.querySelectorAll('.toc-nav a');
  if (tocLinks.length > 0) {
    const headers = Array.from(document.querySelectorAll('.docs-content h2[id], .docs-content h3[id]'));
    
    function updateToc() {
      const scrollPos = window.scrollY + 100;
      let activeHeader = null;

      for (const header of headers) {
        if (header.offsetTop <= scrollPos) {
          activeHeader = header;
        } else {
          break;
        }
      }

      tocLinks.forEach(link => {
        link.classList.remove('active');
        if (activeHeader && link.getAttribute('href') === `#${activeHeader.id}`) {
          link.classList.add('active');
        }
      });
    }

    window.addEventListener('scroll', updateToc, { passive: true });
    updateToc();
  }

  // 7. Search Index & Dialog
  const isVi = document.documentElement.lang === 'vi' || window.location.pathname.includes('/vi/');

  const searchIndexEn = [
    { title: "Introduction & Overview", path: "index.html", section: "About", snippet: "Offline-ready, enterprise-grade MS Exchange to Gmail forwarder written in Rust." },
    { title: "Key Highlights", path: "index.html#features", section: "Overview", snippet: "Zero external dependencies, single binary, embedded Axum web UI, SQLite deduplication." },
    { title: "Quick Start Guide", path: "index.html#quickstart", section: "Getting Started", snippet: "Run with cargo, dev mode, build release binary, and open localhost:8080." },
    { title: "Architecture Overview", path: "architecture.html", section: "Architecture", snippet: "Air-gapped LAN topology, Exchange IMAP fetching, MIME parsing, and SMTP relay delivery." },
    { title: "Data Flow & Processing Lifecycle", path: "architecture.html#lifecycle", section: "Architecture", snippet: "Step-by-step mailbox polling, message deduplication, and attachment handling." },
    { title: "Security & Air-Gapped Isolation", path: "architecture.html#security", section: "Architecture", snippet: "Strict local network isolation, self-signed TLS support, credential masking." },
    { title: "Deployment: Windows Server 2019", path: "installation.html#windows", section: "Deployment", snippet: "Installing native Windows Service via sc.exe and install_service.bat with auto-recovery." },
    { title: "Deployment: Linux Mint & Ubuntu", path: "installation.html#linux", section: "Deployment", snippet: "Configuring systemd unit service, permissions, and journalctl logging." },
    { title: "Compiling from Source", path: "installation.html#build", section: "Deployment", snippet: "Offline cargo vendoring, target release builds, and cross-compiling." },
    { title: "Exchange IMAP Configuration", path: "configuration.html#imap", section: "Configuration", snippet: "Configuring host, port 993/143, SSL/TLS, credentials, and self-signed certificates." },
    { title: "Forwarding Rules & Routing", path: "configuration.html#forwarding", section: "Configuration", snippet: "Setting destination Gmail addresses, subject prefixes, and polling intervals." },
    { title: "Exchange SMTP Relay Configuration", path: "configuration.html#smtp", section: "Configuration", snippet: "Internal Exchange relay port 25/587, authenticated or anonymous relaying." },
    { title: "CLI Flags & Environment Variables", path: "configuration.html#cli", section: "Configuration", snippet: "Command line arguments: --port, --host, --db, and --service." },
    { title: "Process Telemetry & Monitoring", path: "operations.html#telemetry", section: "Operations", snippet: "Real-time PID, PPID, RSS/VmSize/VmPeak, CPU load, and active OS threads." },
    { title: "Subsystem Health Diagnostics", path: "operations.html#health", section: "Operations", snippet: "Diagnostic latency pings for Tokio runtime, SQLite, poller, and Exchange." },
    { title: "SQLite Database Maintenance", path: "operations.html#database", section: "Operations", snippet: "PRAGMA integrity check, WAL journal mode, backup, and VACUUM routines." },
    { title: "Troubleshooting & Common Issues", path: "operations.html#troubleshooting", section: "Operations", snippet: "Fixing TLS certificate errors, relay rejections, and Exchange auth failures." },
    { title: "Mailbox Explorer Overview", path: "mailbox-explorer.html", section: "Explorer", snippet: "Live IMAP folder browsing, viewing raw MIME messages, attachments, and manual forwarding." },
    { title: "REST API Reference", path: "api.html", section: "API", snippet: "Comprehensive documentation of all Axum endpoints: /api/status, /api/process, /api/config, /api/sync." }
  ];

  const searchIndexVi = [
    { title: "Tổng quan & Điểm nổi bật", path: "index.html", section: "Giới thiệu", snippet: "Ứng dụng chuyển tiếp email MS Exchange sang Gmail viết bằng Rust cho môi trường air-gapped." },
    { title: "Tính năng cốt lõi", path: "index.html#features", section: "Tổng quan", snippet: "Không phụ thuộc ngoài, nhị phân duy nhất, Axum Web UI nhúng sẵn, chống trùng lặp bằng SQLite." },
    { title: "Bắt đầu nhanh", path: "index.html#quickstart", section: "Bắt đầu", snippet: "Chạy với Cargo, chế độ dev, biên dịch binary release và truy cập localhost:8080." },
    { title: "Kiến trúc hệ thống", path: "architecture.html", section: "Kiến trúc", snippet: "Mô hình mạng cô lập air-gapped LAN, lấy mail qua IMAP, phân tích MIME và chuyển tiếp qua SMTP relay." },
    { title: "Vòng đời xử lý email", path: "architecture.html#lifecycle", section: "Kiến trúc", snippet: "Các bước tuần tự kiểm tra hòm thư, deduplication chống trùng và bảo toàn tệp đính kèm." },
    { title: "Bảo mật & Cô lập mạng LAN", path: "architecture.html#security", section: "Kiến trúc", snippet: "Không kết nối internet ra ngoài, hỗ trợ chứng chỉ nội bộ tự ký, che mật khẩu." },
    { title: "Triển khai Windows Server 2019", path: "installation.html#windows", section: "Triển khai", snippet: "Cài đặt Windows Service tự động với sc.exe và install_service.bat, tự khởi động lại khi lỗi." },
    { title: "Triển khai Linux Mint & Ubuntu", path: "installation.html#linux", section: "Triển khai", snippet: "Cấu hình systemd unit service, phân quyền và theo dõi log bằng journalctl." },
    { title: "Biên dịch từ mã nguồn", path: "installation.html#build", section: "Triển khai", snippet: "Cargo vendoring offline, tối ưu hóa release binary và cross-compile cho Windows." },
    { title: "Cấu hình IMAP Exchange", path: "configuration.html#imap", section: "Cấu hình", snippet: "Thiết lập máy chủ Exchange, cổng 993/143, mã hóa SSL/TLS và chứng chỉ tự ký." },
    { title: "Quy tắc chuyển tiếp & Định tuyến", path: "configuration.html#forwarding", section: "Cấu hình", snippet: "Địa chỉ Gmail nhận, tiền tố tiêu đề, chu kỳ quét và bảo toàn Reply-To." },
    { title: "Cấu hình Exchange SMTP Relay", path: "configuration.html#smtp", section: "Cấu hình", snippet: "Cổng 25 hoặc 587, gửi thư thông qua connector nội bộ của máy chủ Exchange." },
    { title: "Tham số dòng lệnh & Biến môi trường", path: "configuration.html#cli", section: "Cấu hình", snippet: "Các tùy chọn khởi chạy: --port, --host, --db, --service." },
    { title: "Giám sát tài nguyên & Tiến trình", path: "operations.html#telemetry", section: "Vận hành", snippet: "Đo kiểm thời gian thực: PID, PPID, dung lượng RAM (RSS, VmPeak), tải CPU và số thread OS." },
    { title: "Kiểm tra sức khỏe hệ thống con", path: "operations.html#health", section: "Vận hành", snippet: "Đo độ trễ phản hồi microsecond của Tokio, SQLite, Poller và cấu hình Exchange." },
    { title: "Bảo trì cơ sở dữ liệu SQLite", path: "operations.html#database", section: "Vận hành", snippet: "Kiểm tra tính toàn vẹn PRAGMA, chế độ WAL, sao lưu an toàn khi dịch vụ đang chạy và VACUUM." },
    { title: "Xử lý sự cố thường gặp", path: "operations.html#troubleshooting", section: "Vận hành", snippet: "Khắc phục lỗi chứng chỉ SSL/TLS, lỗi từ chối relay và lỗi xác thực tài khoản Exchange." },
    { title: "Trình khám phá Hộp thư (Mailbox Explorer)", path: "mailbox-explorer.html", section: "Khám phá", snippet: "Duyệt thư mục Exchange trực tiếp, xem trước MIME, tải file đính kèm và gửi lại thủ công." },
    { title: "Tài liệu REST API", path: "api.html", section: "API", snippet: "Đặc tả kỹ thuật các endpoint: /api/status, /api/process, /api/config, /api/sync." }
  ];

  const searchIndex = isVi ? searchIndexVi : searchIndexEn;

  const searchTrigger = document.getElementById('search-trigger');
  const searchModal = document.getElementById('search-modal');
  const searchInput = document.getElementById('search-input');
  const searchResults = document.getElementById('search-results');

  if (isVi && searchInput) {
    searchInput.placeholder = "Nhập từ khóa tìm kiếm...";
  }

  function openSearch() {
    if (!searchModal) return;
    searchModal.classList.add('active');
    if (searchInput) {
      searchInput.value = '';
      searchInput.focus();
      renderSearchResults('');
    }
  }

  function closeSearch() {
    if (!searchModal) return;
    searchModal.classList.remove('active');
  }

  function renderSearchResults(query) {
    if (!searchResults) return;
    const cleanQuery = query.trim().toLowerCase();
    
    let matches = searchIndex;
    if (cleanQuery.length > 0) {
      matches = searchIndex.filter(item => 
        item.title.toLowerCase().includes(cleanQuery) ||
        item.snippet.toLowerCase().includes(cleanQuery) ||
        item.section.toLowerCase().includes(cleanQuery)
      );
    }

    if (matches.length === 0) {
      const emptyMsg = isVi 
        ? 'Không tìm thấy tài liệu phù hợp cho "' + query + '"'
        : 'No documentation matches found for "' + query + '"';
      searchResults.innerHTML = '<div style="padding: 1.5rem; text-align: center; color: var(--text-muted);">' + emptyMsg + '</div>';
      return;
    }

    searchResults.innerHTML = matches.map(item => `
      <a href="${item.path}" class="search-item">
        <div class="search-item-title">${item.title} <span style="font-size:0.75rem; color:var(--text-subtle); font-weight:normal;">(${item.section})</span></div>
        <div class="search-item-snippet">${item.snippet}</div>
      </a>
    `).join('');
  }

  if (searchTrigger) searchTrigger.addEventListener('click', openSearch);

  if (searchModal) {
    searchModal.addEventListener('click', (e) => {
      if (e.target === searchModal) closeSearch();
    });
  }

  if (searchInput) {
    searchInput.addEventListener('input', (e) => {
      renderSearchResults(e.target.value);
    });
  }

  document.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      if (searchModal && searchModal.classList.contains('active')) {
        closeSearch();
      } else {
        openSearch();
      }
    } else if (e.key === 'Escape' && searchModal && searchModal.classList.contains('active')) {
      closeSearch();
    }
  });

})();
