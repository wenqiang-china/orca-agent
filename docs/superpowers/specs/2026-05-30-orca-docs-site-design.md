# Orca Agent Documentation Site Design

## Overview

A static documentation website for Orca Agent, styled after the design language of modern AI product sites (OpenAI, Anthropic). Built with pure HTML/CSS/JS, zero build step, deployable directly to GitHub Pages.

## Tech Stack

**Chosen: Option A — Pure Static HTML + CSS**

- No build tool required, zero dependencies
- Open `index.html` directly to browse
- Native GitHub Pages support by placing files in `docs/`
- Fastest possible load time

## Site Structure

Seven pages total, sharing a consistent navigation and design system:

| Page | File | Purpose |
|------|------|---------|
| Landing | `index.html` | Hero section, feature grid, stats, CTA |
| Quick Start | `quickstart.html` | Build, install, first run guide |
| Providers | `providers.html` | DeepSeek, Anthropic, OpenAI, compatible providers |
| Tools | `tools.html` | 8 built-in tools reference |
| Architecture | `architecture.html` | Crate layout, data flow, capacity control |
| CLI | `cli.html` | All commands, flags, in-chat shortcuts |
| Config | `config.html` | config.toml reference |

All seven pages share a common `styles.css` and `main.js`.

## Design System

### Colors

| Token | Hex | Usage |
|-------|-----|-------|
| Background | `#0d0d0d` | Page background |
| Surface | `#1a1a2e` | Code blocks, elevated cards |
| Primary | `#7c3aed` | Accents, active states, gradient start |
| Accent | `#06b6d4` | Gradient end, secondary highlights |
| Gradient | `#7c3aed` → `#06b6d4` | Hero text, stat numbers, page titles |
| Success | `#10b981` | Success states |
| Warning | `#f59e0b` | Warning states |
| Error | `#ef4444` | Error states |
| Text Primary | `rgba(255,255,255,0.9)` | Headings |
| Text Body | `rgba(255,255,255,0.6)` | Body copy |
| Text Secondary | `rgba(255,255,255,0.35)` | Captions, labels |
| Border | `rgba(255,255,255,0.08)` | Cards, dividers |

### Typography

- **Font Stack**: `-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif`
- **Monospace**: `'SF Mono', 'Fira Code', monospace`
- **H1**: 36px Bold, -1px tracking, gradient fill
- **H2**: 24px Semibold, -0.5px tracking, white
- **H3**: 18px Semibold, white
- **Body**: 15px Regular, 1.7 line-height, 60% opacity
- **Code**: 13px monospace, surface background, primary color
- **Labels**: 11px uppercase, 2px letter-spacing, 35% opacity

### Spacing & Layout

- Max page width: 1200px
- Content width (docs): 800px
- Side padding: 24px mobile, 48px desktop
- Section gap: 80px
- Card padding: 24px, radius: 12px, border: 1px
- Nav height: 64px
- Doc sidebar width: 280px

### Component Patterns

- **Cards**: `rgba(255,255,255,0.03)` background, `1px rgba(255,255,255,0.06)` border, `12px` radius
- **Code blocks**: Surface background, `SF Mono` font, monochrome window dots (red/amber/green)
- **Info boxes**: Colored left border (3px), 0.08 opacity background, 8px radius
- **Buttons**: Gradient background (primary → darker purple) or outlined style

---

## Navigation

**Position**: Fixed, top of viewport

**Desktop (≥768px)**:
- Height: 64px
- Background: `rgba(13,13,13,0.85)` with `backdrop-filter: blur(12px)`
- Left: Logo ("Orca" gradient, "Agent" at 60% white) + horizontal nav links
- Active link: white text, `rgba(255,255,255,0.06)` background pill
- Right: version badge + GitHub icon
- All links: 13px, 45% white for inactive, white for active

**Mobile (<768px)**:
- Height: 56px
- Background: `rgba(13,13,13,0.95)`
- Left: Logo
- Right: Hamburger icon (three lines)
- Click hamburger → slide-down panel with vertical nav links
- Link spacing: 2px, padding: 10px 12px, font-size: 14px

**Nav Links**:
Docs, Quick Start, Providers, Tools, Architecture, CLI, Config

---

## Landing Page (`index.html`)

### Hero Section

- Small badge at top: "Open Source · Written in Rust" — 11px uppercase, pill shape, purple tint border
- Title line 1: "AI-Powered" — gradient text (purple → cyan), 44px bold
- Title line 2: "Coding Agent" — white, 44px bold
- Subtitle: 16px, 45% opacity, max-width 560px
- Two CTA buttons side by side:
  - "Get Started →" — gradient background, white text, 14px bold
  - "GitHub" — outlined, GitHub icon SVG inline, 14px, 60% opacity text
- Below buttons: terminal code block mockup
  - Surface background, 480px max-width, centered
  - Three colored dots (red, amber, green)
  - Two prompt lines: `$ cargo install orca-agent`, `$ orca chat --provider deepseek`
  - Agent response line with cyan `>` prompt

### Feature Grid

- Section title: "Why Orca Agent?" — 22px semibold, centered
- 3-column grid (responsive to 2 then 1 on smaller screens)
- 6 feature cards:
  1. **OS-Level Sandboxing** (lock emoji) — Seatbelt, AppContainer, Landlock
  2. **Semantic Capacity** (brain emoji) — CanonicalState, 3-checkpoint evaluation
  3. **Multi-Provider** (plug emoji) — DeepSeek, Claude, OpenAI, Ollama, Groq, vLLM
  4. **Checkpoint Recovery** (refresh emoji) — Save/resume from any checkpoint
  5. **8 Built-in Tools** (tools emoji) — Filesystem, shell, git, web
  6. **Loop Guard** (target emoji) — Repetition detection, mutation-aware clearing

### Stats Bar

- 4 columns, centered
- Numbers in gradient text (purple → cyan), 28px bold
- Labels below in 12px, 35% opacity
- Values: 8,084 Lines of Rust / 19 Crates / 65 Tests Passing / 3 Providers

### Footer

- Top border: 1px rgba white 0.06
- Left: "Orca Agent" gradient + "· MIT License"
- Right: "GitHub", "Docs", "Releases" links, 12px, 35% opacity

---

## Documentation Pages

All six doc pages (Quick Start, Providers, Tools, Architecture, CLI, Config) share the same layout.

### Desktop Layout (≥1024px)

**Left Sidebar (280px)**:
- Fixed position, full height below nav
- "On this page" heading at top (11px uppercase, 2px letter-spacing)
- Section links listed vertically
- Active section: purple left border (2px), purple tint background, white text
- Inactive: 45% opacity white text, 13px
- Grouped with section labels (e.g., "Reference")

**Main Content Area**:
- Flex 1, padding 48px 64px
- Max-width 720px
- Breadcrumb at top: "Docs / Section Name" — 12px, 35% opacity
- Page title: gradient text, 36px bold
- Subtitle: 16px, 50% opacity, line-height 1.7
- Sections with `<h2>` headers, anchor links

### Mobile Layout (<1024px)

- Single column, sidebar hidden (or accessible via button)
- Reduced padding: 24px 20px
- Title: 28px
- Code blocks: `overflow-x: auto`

### Content Elements

**Code Blocks**:
```html
<div class="code-block">
  <div class="dots">red, amber, green</div>
  <div class="line">$ cargo build --release</div>
</div>
```
- Surface background, 12px radius, 1px border
- SF Mono font, 13px
- Line numbers in comments: 30% opacity

**Info Boxes** (tips, warnings):
- Left border: 3px (cyan for tips, amber for warnings, red for errors)
- Background: 0.08 opacity of border color
- Title in 14px semibold
- Body in 14px, 60% opacity, 1.6 line-height

**Inline Code**:
- Surface background, 2px 6px padding, 4px radius
- 12px font, primary purple color

---

## Responsive Breakpoints

| Breakpoint | Behavior |
|-----------|----------|
| ≥1200px | Full desktop, max-width 1200px centered |
| ≥1024px | Doc sidebar visible, content side-by-side |
| ≥768px | Horizontal nav, single column content |
| <768px | Mobile nav (hamburger), stacked layout, full-width cards |

## File Structure

```
docs/
├── index.html          # Landing page
├── quickstart.html     # Quick Start guide
├── providers.html      # Provider configuration
├── tools.html          # Tools reference
├── architecture.html   # Architecture overview
├── cli.html            # CLI reference
├── config.html         # Config reference
├── styles.css          # Shared stylesheet
└── main.js             # Navigation toggle, smooth scroll, active section tracking
```
