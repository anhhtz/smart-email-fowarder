# Smart Email Forwarder Documentation

This folder contains the complete, static documentation website for **Smart Email Forwarder**, ready for deployment to **GitHub Pages**.

---

## Live GitHub Pages Setup

To host this documentation on GitHub Pages:

1. Open your GitHub repository in your web browser.
2. Navigate to **Settings** &rarr; **Pages** (under the "Code and automation" section in the left sidebar).
3. Under **Build and deployment**:
   - **Source**: Select `Deploy from a branch`.
   - **Branch**: Select your default branch (e.g., `main` or `master`).
   - **Folder**: Select `/docs` from the folder dropdown.
4. Click **Save**.
5. GitHub Pages will build and deploy the site in approximately 30–60 seconds. Your documentation will be live at:
   ```text
   https://<your-github-username>.github.io/<repository-name>/
   ```

> [!NOTE]
> The `.nojekyll` file is included in this directory to ensure GitHub Pages serves HTML, CSS, JavaScript, and static assets directly without processing by Jekyll.

---

## Documentation Structure & Languages

The documentation is fully bilingual (**English** and **Tiếng Việt / Vietnamese**) with an instant language switcher in the header and bilingual search:

```text
docs/
├── .nojekyll                 # Bypasses Jekyll on GitHub Pages
├── README.md                 # This overview guide
├── index.html                # [EN] Homepage, system highlights, and quick start
├── architecture.html         # [EN] Network topology, MIME pipeline, deduplication, security
├── installation.html         # [EN] Windows Server 2019 (service) and Linux Mint deployment
├── configuration.html        # [EN] Web UI settings, IMAP, SMTP relay, rules, and CLI flags
├── operations.html           # [EN] Telemetry, health checks, SQLite maintenance, troubleshooting
├── mailbox-explorer.html     # [EN] Interactive folder browser, raw MIME viewer, manual forwards
├── api.html                  # [EN] Full REST API specification with cURL and JSON examples
├── vi/                       # [VI] Phiên bản Tiếng Việt hoàn chỉnh
│   ├── index.html            # [VI] Trang chủ, điểm nổi bật và bắt đầu nhanh
│   ├── architecture.html     # [VI] Kiến trúc hệ thống, mạng air-gapped, SQLite deduplication
│   ├── installation.html     # [VI] Cài đặt Windows Server 2019 (Service) & Linux Mint (systemd)
│   ├── configuration.html    # [VI] Hướng dẫn cấu hình IMAP, SMTP relay, quy tắc chuyển tiếp
│   ├── operations.html       # [VI] Vận hành, giám sát tài nguyên, bảo trì SQLite & xử lý sự cố
│   ├── mailbox-explorer.html # [VI] Trình khám phá hòm thư, xem trước MIME, tải file đính kèm
│   └── api.html              # [VI] Tài liệu REST API đầy đủ kèm ví dụ JSON & cURL
└── assets/
    ├── css/
    │   └── docs.css          # Responsive styling, modern IT console theme, dark/light modes
    └── js/
        └── docs.js           # Bilingual search (Ctrl+K), theme switcher, copy buttons, tabs
```

---

## Local Preview

You can preview the documentation locally with any static web server:

```bash
# Using Python 3
cd docs
python3 -m http.server 8000

# Using Node / npx
npx serve docs

# Using Rust (e.g. miniserve)
miniserve docs --index index.html -p 8000
```

Open `http://localhost:8000` in your browser.
