---
name: Smart Email Forwarder
description: Enterprise IT Operations Console (The Category Standard)
colors:
  primary: "#1d4ed8"
  primary-hover: "#1e40af"
  primary-active: "#172554"
  primary-light: "#eff6ff"
  primary-border: "#bfdbfe"
  neutral-bg: "#f8fafc"
  neutral-surface: "#ffffff"
  neutral-surface-subtle: "#f8fafc"
  neutral-surface-hover: "#f1f5f9"
  neutral-border: "#e2e8f0"
  neutral-border-strong: "#cbd5e1"
  sidebar-bg: "#0f172a"
  sidebar-active: "#1e293b"
  sidebar-text: "#cbd5e1"
  text-main: "#0f172a"
  text-muted: "#475569"
  text-tertiary: "#64748b"
  google-blue: "#1a73e8"
  selection-blue: "#1e3a8a"
  accent-cyan: "#38bdf8"
  success: "#047857"
  success-bg: "#ecfdf5"
  success-border: "#a7f3d0"
  warning: "#b45309"
  warning-bg: "#fffbeb"
  warning-border: "#fde68a"
  danger: "#b91c1c"
  danger-bg: "#fef2f2"
  danger-border: "#fecaca"
typography:
  display:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"
    fontSize: "1.57rem"
    fontWeight: 700
    lineHeight: 1.25
    letterSpacing: "-0.02em"
  title-lg:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"
    fontSize: "1.43rem"
    fontWeight: 700
    lineHeight: 1.3
    letterSpacing: "-0.01em"
  headline:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"
    fontSize: "1.14rem"
    fontWeight: 700
    lineHeight: 1.3
    letterSpacing: "-0.01em"
  headline-sm:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"
    fontSize: "1.07rem"
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: "0em"
  body:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "0em"
  body-sm:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"
    fontSize: "0.93rem"
    fontWeight: 500
    lineHeight: 1.5
    letterSpacing: "0em"
  caption:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"
    fontSize: "0.86rem"
    fontWeight: 500
    lineHeight: 1.4
    letterSpacing: "0.02em"
  mono:
    fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace"
    fontSize: "0.86rem"
    fontWeight: 500
    lineHeight: 1.5
    letterSpacing: "0em"
rounded:
  xs: "4px"
  sm: "6px"
  md: "8px"
  lg: "12px"
  xl: "16px"
  full: "9999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "16px"
  lg: "24px"
  xl: "32px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "#ffffff"
    rounded: "{rounded.md}"
    padding: "8px 16px"
  button-primary-hover:
    backgroundColor: "{colors.primary-hover}"
  button-secondary:
    backgroundColor: "{colors.neutral-surface}"
    textColor: "{colors.text-main}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
  badge-success:
    backgroundColor: "{colors.success-bg}"
    textColor: "{colors.success}"
    rounded: "{rounded.full}"
    padding: "3px 9px"
  badge-warning:
    backgroundColor: "{colors.warning-bg}"
    textColor: "{colors.warning}"
    rounded: "{rounded.full}"
    padding: "3px 9px"
  badge-danger:
    backgroundColor: "{colors.danger-bg}"
    textColor: "{colors.danger}"
    rounded: "{rounded.full}"
    padding: "3px 9px"
---

# Design System: Smart Email Forwarder

## Overview

Smart Email Forwarder follows **The Category Standard (Modern Enterprise IT Operations Console)**. Engineered specifically for air-gapped corporate servers (Windows Server 2019 / Linux Mint), the visual language prioritizes instant scannability, dense information hierarchy, reliable status feedback, and zero runtime dependencies.

The system uses neutral slate grounds, crisp elevated card surfaces, high-contrast typography, and an authoritative cobalt primary accent paired with WCAG AA compliant semantic health indicators.

## Colors

### Brand & Interactive Accent
- **Primary Cobalt (`#1d4ed8`)**: Used for primary action buttons, focused input rings, active status markers, and interactive links. Contrast ratio against white: 5.6:1 (WCAG AA compliant).
- **Primary Hover (`#1e40af`)**: Subtle darkening on mouse hover and active states.
- **Primary Light (`#eff6ff`)**: Subtle selection backgrounds and info banners.

### Neutral Canvas & Surfaces
- **App Canvas (`#f8fafc`)**: Soft slate-50 background that reduces glare during extended monitoring.
- **Surface Card (`#ffffff`)**: Pure white elevated cards for high content contrast.
- **Surface Subtle (`#f8fafc` / `#f1f5f9`)**: Alternating table rows, input backgrounds, and toolbar wrappers.
- **Sidebar Rail (`#0f172a`)**: Deep enterprise midnight slate for structural separation.

### Text Hierarchy
- **Primary Text (`#0f172a`)**: Dominant headings and active cell values.
- **Secondary / Muted Text (`#475569`)**: Labels, descriptions, and secondary metadata (contrast ratio ≥ 5.4:1).
- **Tertiary Text (`#64748b`)**: Subtle timestamps, borders, and empty states.

### Status Indicators (Strict WCAG AA Compliance)
- **Healthy / Active (`#047857` on `#ecfdf5`)**: Successful deliveries, connected IMAP/SMTP nodes, healthy SQLite storage. Contrast ratio: 4.8:1.
- **Warning / Degraded (`#b45309` on `#fffbeb`)**: Paused poller, awaiting configuration, pending syncs. Contrast ratio: 4.7:1.
- **Critical / Failed (`#b91c1c` on `#fef2f2`)**: Failed deliveries, connection timeouts, auth rejections. Contrast ratio: 5.6:1.

## Typography

- **UI Sans**: `-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif`.
- **Data Mono**: `ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace` with `font-variant-numeric: tabular-nums`. Used for message IDs, PIDs, timestamps, ports, and raw headers.
- **Scale**:
  - Page Titles (`h1`): `22px` (`1.57rem`), bold, tracking `-0.02em`.
  - Section Headings (`h2`): `16px` (`1.14rem`), bold, tracking `-0.01em`.
  - Subsystem & Card Titles (`h3`): `13px` (`0.93rem`), semi-bold.
  - Body Text: `13px`–`14px`, regular / medium.
  - Functional Badges & Footers: `12px` minimum floor.

## Layout

- **Shell**: Left-hand persistent navigation rail (`256px`), with flexible right-hand main content canvas (`max-width: 1440px`).
- **Rhythm**: 16px / 24px vertical and horizontal spacing rhythm between content cards.
- **Responsive**: At viewports `<= 768px`, the sidebar smoothly stacks above the main canvas, and form column widths expand to 100%.

## Elevation & Depth

- **Shadows**: Natural, soft blurred shadows only. Zero hard 0-blur block shadows and zero chromatic dark halos.
  - `xs`: `0 1px 2px 0 rgba(15, 23, 42, 0.05)`
  - `sm`: `0 1px 3px 0 rgba(15, 23, 42, 0.07), 0 1px 2px -1px rgba(15, 23, 42, 0.04)`
  - `lg`: `0 10px 15px -3px rgba(15, 23, 42, 0.09), 0 4px 6px -4px rgba(15, 23, 42, 0.04)`
  - `xl` (Modals/Overlays): `0 20px 25px -5px rgba(15, 23, 42, 0.12), 0 8px 10px -6px rgba(15, 23, 42, 0.05)`

## Shapes

- Small interactive elements (buttons, inputs, select pills): `radius: 6px`–`8px`.
- Content cards & containers: `radius: 12px`.
- Modals & dialogs: `radius: 16px`.
- Badges, status pills, and toggle sliders: `radius: 9999px` (fully rounded pill).

## Components

- **Sidebar Rail**: Minimal branding badge, active state with subtle inset highlight and cobalt indicator, live heartbeat dot.
- **Metric Cards (Stats Grid)**: Prominent numerical value in tabular format, uppercase tracking on category labels, and contextual trend footer.
- **Data Tables**: Explicit cell padding (`12px 16px`), distinct uppercase header band with separator border, and smooth row hover transitions.
- **Email Browser**: Filter pill group with elevated active pill, integrated search field with clear trigger, and pagination controls.
- **Process Monitor**: Process telemetry banner with health beacon dot, animated resource progress bars, and subsystem status cards.
- **Buttons**: Clear visual distinction between Primary (`btn-primary`), Secondary (`btn-secondary`), and Ghost Outline (`btn-outline`).

## Do's and Don'ts

### Do's
- Do use authored SVG icons with consistent 2px stroke width for all controls and indicators.
- Do enforce tabular figures (`tabular-nums`) across all timestamps, metrics, IDs, and financial/data figures.
- Do maintain WCAG AA contrast compliance (minimum 4.5:1 for body and badge text).
- Do keep functional text at or above the 12px legibility floor.
- Do preserve 100% offline self-containment with zero external network dependencies.

### Don'ts
- Don't use Unicode emoji or raw text glyphs as substitute icons.
- Don't use colored border-left accents greater than 1px.
- Don't introduce colored shadow halos, glow effects, or gradient text.
- Don't skip heading levels (maintain strict `h1` -> `h2` -> `h3` hierarchy).
- Don't introduce external CDN links or unbundled remote dependencies.
