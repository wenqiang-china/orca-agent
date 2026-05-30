# Orca Agent Documentation Site — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a 7-page static documentation website for Orca Agent with a modern dark theme inspired by OpenAI/Anthropic design language.

**Architecture:** Pure static HTML/CSS/JS site placed in `docs/`. No build tools, no dependencies. Shared `styles.css` defines the design system tokens, reusable components (nav, cards, code blocks, info boxes, sidebar). `main.js` handles mobile nav toggle, sidebar active-section tracking, and smooth scrolling.

**Tech Stack:** HTML5, CSS3 (custom properties, flexbox, grid), vanilla JavaScript

---

## File Structure

```
docs/
├── styles.css          # Design system: tokens, reset, nav, sidebar, cards, code blocks, info boxes, responsive
├── main.js             # Mobile nav toggle, sidebar scroll-spy, smooth scroll
├── index.html          # Landing page: hero, feature grid, stats bar, footer
├── quickstart.html     # Quick Start: build, install, first run
├── providers.html      # Providers: DeepSeek, Anthropic, OpenAI, custom/OpenAI-compatible
├── tools.html          # Tools: 8 built-in tools reference
├── architecture.html   # Architecture: crate layout, data flow, key components
├── cli.html            # CLI: commands, flags, in-chat shortcuts
├── config.html         # Config: config.toml reference
└── superpowers/        # (already exists, do not touch)
```

---

### Task 1: Create `docs/styles.css`

**Files:**
- Create: `docs/styles.css`

- [ ] **Step 1: Write the complete stylesheet**

Write `docs/styles.css` with the following sections in order:

```css
/* === Design Tokens === */
:root {
  --bg: #0d0d0d;
  --surface: #1a1a2e;
  --primary: #7c3aed;
  --accent: #06b6d4;
  --success: #10b981;
  --warning: #f59e0b;
  --error: #ef4444;
  --text-primary: rgba(255, 255, 255, 0.9);
  --text-body: rgba(255, 255, 255, 0.6);
  --text-secondary: rgba(255, 255, 255, 0.35);
  --text-muted: rgba(255, 255, 255, 0.45);
  --border: rgba(255, 255, 255, 0.06);
  --border-strong: rgba(255, 255, 255, 0.08);
  --card-bg: rgba(255, 255, 255, 0.03);
  --gradient: linear-gradient(135deg, var(--primary), var(--accent));
  --gradient-text: linear-gradient(135deg, #7c3aed, #06b6d4);
  --font-sans: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  --font-mono: 'SF Mono', 'Fira Code', monospace;
  --nav-h: 64px;
  --sidebar-w: 280px;
  --max-w: 1200px;
  --content-w: 720px;
  --radius: 12px;
  --radius-sm: 8px;
}

/* === Reset === */
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
html { scroll-behavior: smooth; scroll-padding-top: calc(var(--nav-h) + 24px); }
body {
  background: var(--bg);
  color: var(--text-body);
  font-family: var(--font-sans);
  font-size: 15px;
  line-height: 1.7;
  -webkit-font-smoothing: antialiased;
}

/* === Typography === */
h1 { font-size: 36px; font-weight: 700; letter-spacing: -1px; }
h2 { font-size: 24px; font-weight: 600; letter-spacing: -0.5px; color: var(--text-primary); margin-bottom: 12px; }
h3 { font-size: 18px; font-weight: 600; color: var(--text-primary); margin-bottom: 8px; }
a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }
code {
  background: var(--surface);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--primary);
  /* overridden to #a78bfa for lighter purple in content */
  color: #a78bfa;
}

/* === Page Title (gradient) === */
.page-title {
  font-size: 36px;
  font-weight: 700;
  letter-spacing: -1px;
  background: var(--gradient-text);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  margin-bottom: 16px;
}

/* === Navigation === */
.nav {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: var(--nav-h);
  background: rgba(13, 13, 13, 0.85);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border-bottom: 1px solid var(--border);
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 48px;
}
.nav-logo {
  font-size: 16px;
  font-weight: 700;
  letter-spacing: -0.5px;
  color: white;
}
.nav-logo .gradient {
  background: var(--gradient-text);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}
.nav-logo .light { color: var(--text-muted); font-weight: 400; margin-left: 4px; }
.nav-links { display: flex; gap: 4px; }
.nav-links a {
  padding: 6px 14px;
  font-size: 13px;
  color: var(--text-muted);
  border-radius: var(--radius-sm);
  transition: background 0.15s, color 0.15s;
}
.nav-links a:hover { background: rgba(255, 255, 255, 0.04); color: var(--text-body); text-decoration: none; }
.nav-links a.active { color: white; background: rgba(255, 255, 255, 0.06); }
.nav-right { display: flex; align-items: center; gap: 12px; }
.nav-version {
  font-size: 12px;
  color: var(--text-secondary);
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border-strong);
  padding: 5px 10px;
  border-radius: 6px;
  font-family: var(--font-mono);
}
.nav-github { display: flex; align-items: center; color: rgba(255, 255, 255, 0.5); }
.nav-github svg { width: 18px; height: 18px; fill: currentColor; }

/* Mobile hamburger */
.nav-hamburger {
  display: none;
  flex-direction: column;
  gap: 3px;
  cursor: pointer;
  padding: 4px;
}
.nav-hamburger span {
  width: 18px;
  height: 2px;
  background: rgba(255, 255, 255, 0.6);
  border-radius: 1px;
  transition: transform 0.2s;
}
.nav-mobile-menu {
  display: none;
  position: fixed;
  top: var(--nav-h);
  left: 0;
  right: 0;
  background: rgba(13, 13, 13, 0.95);
  border-bottom: 1px solid var(--border);
  padding: 12px 20px;
  z-index: 99;
  flex-direction: column;
  gap: 2px;
}
.nav-mobile-menu.open { display: flex; }
.nav-mobile-menu a {
  padding: 10px 12px;
  font-size: 14px;
  color: var(--text-muted);
  border-radius: var(--radius-sm);
}
.nav-mobile-menu a:hover, .nav-mobile-menu a.active { color: white; background: rgba(255, 255, 255, 0.06); }

/* === Sidebar === */
.doc-layout { display: flex; padding-top: var(--nav-h); min-height: 100vh; }
.sidebar {
  position: fixed;
  top: var(--nav-h);
  left: 0;
  bottom: 0;
  width: var(--sidebar-w);
  border-right: 1px solid var(--border);
  padding: 24px 0;
  overflow-y: auto;
}
.sidebar-heading {
  padding: 0 24px 16px;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 2px;
  color: var(--text-secondary);
}
.sidebar-section {
  padding: 0 8px;
  display: flex;
  flex-direction: column;
  gap: 1px;
  margin-bottom: 8px;
}
.sidebar-section-label {
  padding: 8px 8px 8px 16px;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 2px;
  color: rgba(255, 255, 255, 0.25);
  margin-top: 8px;
}
.sidebar-link {
  padding: 8px 8px 8px 16px;
  font-size: 13px;
  color: var(--text-muted);
  border-radius: 6px;
  border-left: 2px solid transparent;
  transition: all 0.15s;
}
.sidebar-link:hover { color: var(--text-body); background: rgba(255, 255, 255, 0.03); text-decoration: none; }
.sidebar-link.active {
  color: var(--text-primary);
  border-left-color: var(--primary);
  background: rgba(124, 58, 237, 0.08);
}

/* === Main Content === */
.doc-content {
  flex: 1;
  margin-left: var(--sidebar-w);
  padding: 48px 64px;
  max-width: calc(var(--content-w) + 128px);
}
.breadcrumb { font-size: 12px; color: var(--text-secondary); margin-bottom: 16px; }
.breadcrumb span { color: var(--text-muted); }
.doc-subtitle { font-size: 16px; color: var(--text-muted); line-height: 1.7; margin-bottom: 32px; }
.doc-section { padding: 20px 0 8px; }
.doc-section h2 { scroll-margin-top: calc(var(--nav-h) + 24px); }

/* === Code Block === */
.code-block {
  background: var(--surface);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  padding: 20px;
  margin: 24px 0;
  font-family: var(--font-mono);
  font-size: 13px;
  line-height: 1.8;
  overflow-x: auto;
}
.code-block .dots { display: flex; gap: 6px; margin-bottom: 12px; }
.code-block .dots span {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}
.code-block .dots .red { background: var(--error); }
.code-block .dots .amber { background: var(--warning); }
.code-block .dots .green { background: var(--success); }
.code-block .line { white-space: pre; }
.code-block .prompt { color: var(--primary); }
.code-block .comment { color: rgba(255, 255, 255, 0.3); }
.code-block .output { color: var(--text-muted); }

/* === Info Boxes === */
.info-box {
  padding: 16px 20px;
  border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
  margin: 24px 0;
  font-size: 14px;
  line-height: 1.6;
}
.info-box.tip { background: rgba(6, 182, 212, 0.08); border-left: 3px solid var(--accent); }
.info-box.warning { background: rgba(245, 158, 11, 0.08); border-left: 3px solid var(--warning); }
.info-box.error { background: rgba(239, 68, 68, 0.08); border-left: 3px solid var(--error); }
.info-box-title { font-weight: 600; color: var(--text-primary); margin-bottom: 6px; }
.info-box-body { color: var(--text-body); }

/* === Card Grid === */
.card-grid { display: grid; gap: 16px; }
.card-grid.cols-3 { grid-template-columns: repeat(3, 1fr); }
.card-grid.cols-2 { grid-template-columns: repeat(2, 1fr); }
.card {
  background: var(--card-bg);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  padding: 24px;
}
.card-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 12px;
  font-size: 18px;
}
.card-icon.purple { background: linear-gradient(135deg, rgba(124, 58, 237, 0.2), rgba(124, 58, 237, 0.05)); }
.card-icon.cyan { background: linear-gradient(135deg, rgba(6, 182, 212, 0.2), rgba(6, 182, 212, 0.05)); }
.card-icon.green { background: linear-gradient(135deg, rgba(16, 185, 129, 0.2), rgba(16, 185, 129, 0.05)); }
.card-icon.amber { background: linear-gradient(135deg, rgba(245, 158, 11, 0.2), rgba(245, 158, 11, 0.05)); }
.card-icon.red { background: linear-gradient(135deg, rgba(239, 68, 68, 0.2), rgba(239, 68, 68, 0.05)); }
.card-title { font-size: 14px; font-weight: 600; color: var(--text-primary); margin-bottom: 6px; }
.card-body { font-size: 12px; color: rgba(255, 255, 255, 0.4); line-height: 1.6; }

/* === Table === */
.doc-table { width: 100%; border-collapse: collapse; margin: 24px 0; }
.doc-table th, .doc-table td { padding: 10px 16px; text-align: left; border-bottom: 1px solid var(--border); }
.doc-table th { font-size: 12px; text-transform: uppercase; letter-spacing: 1px; color: var(--text-secondary); font-weight: 600; }
.doc-table td { font-size: 14px; color: var(--text-body); }
.doc-table tr:last-child td { border-bottom: none; }

/* === Buttons === */
.btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 24px;
  border-radius: 10px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.15s;
  text-decoration: none;
}
.btn:hover { opacity: 0.85; text-decoration: none; }
.btn-primary { background: linear-gradient(135deg, #7c3aed, #6d28d9); color: white; }
.btn-outline { border: 1px solid var(--border-strong); color: var(--text-muted); }

/* === Footer === */
.footer {
  border-top: 1px solid var(--border);
  padding: 24px 48px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 80px;
}
.footer-brand { font-size: 13px; color: var(--text-secondary); }
.footer-brand .gradient {
  background: var(--gradient-text);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  font-weight: 600;
}
.footer-links { display: flex; gap: 16px; font-size: 12px; color: var(--text-secondary); }
.footer-links a { color: var(--text-secondary); }
.footer-links a:hover { color: var(--text-body); }

/* === Hero (landing only) === */
.hero { text-align: center; padding: 80px 32px 48px; max-width: 700px; margin: 0 auto; }
.hero-badge {
  display: inline-block;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 3px;
  color: var(--text-secondary);
  background: rgba(124, 58, 237, 0.12);
  border: 1px solid rgba(124, 58, 237, 0.2);
  padding: 4px 12px;
  border-radius: 20px;
  margin-bottom: 16px;
}
.hero-title { font-size: 44px; font-weight: 700; letter-spacing: -1.5px; line-height: 1.15; margin-bottom: 16px; }
.hero-title .gradient {
  background: var(--gradient-text);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}
.hero-title .white { color: var(--text-primary); }
.hero-subtitle { font-size: 16px; color: var(--text-muted); line-height: 1.7; max-width: 560px; margin: 0 auto 28px; }
.hero-buttons { display: flex; gap: 12px; justify-content: center; margin-bottom: 32px; }

/* === Stats Bar === */
.stats-bar { display: flex; justify-content: center; gap: 48px; text-align: center; padding: 48px 0; }
.stat-number {
  font-size: 28px;
  font-weight: 700;
  background: var(--gradient-text);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}
.stat-label { font-size: 12px; color: var(--text-secondary); margin-top: 4px; }

/* === Section titles (landing) === */
.section-title { font-size: 22px; font-weight: 600; color: var(--text-primary); text-align: center; margin-bottom: 32px; }

/* === Max-width container === */
.container { max-width: var(--max-w); margin: 0 auto; padding: 0 48px; }

/* === Responsive === */
@media (max-width: 1024px) {
  .sidebar { display: none; }
  .doc-content { margin-left: 0; padding: 32px 24px; }
}
@media (max-width: 768px) {
  .nav { padding: 0 20px; height: 56px; --nav-h: 56px; }
  .nav-links { display: none; }
  .nav-hamburger { display: flex; }
  .nav-right { gap: 8px; }
  .container { padding: 0 20px; }
  .hero { padding: 48px 20px 32px; }
  .hero-title { font-size: 32px; }
  .hero-buttons { flex-direction: column; align-items: center; }
  .card-grid.cols-3 { grid-template-columns: 1fr; }
  .card-grid.cols-2 { grid-template-columns: 1fr; }
  .stats-bar { flex-wrap: wrap; gap: 24px 32px; }
  .footer { flex-direction: column; gap: 16px; text-align: center; padding: 24px 20px; }
  .doc-content { padding: 24px 20px; }
  h1, .page-title { font-size: 28px; }
  .code-block { font-size: 12px; padding: 16px; }
}
```

- [ ] **Step 2: Verify the CSS file is valid**

Run: `wc -l docs/styles.css`
Expected: ~250-300 lines

- [ ] **Step 3: Commit**

```bash
git add docs/styles.css
git commit -m "feat(docs): add shared stylesheet with design system tokens and components"
```

---

### Task 2: Create `docs/main.js`

**Files:**
- Create: `docs/main.js`

- [ ] **Step 1: Write the JavaScript**

Write `docs/main.js`:

```javascript
(function () {
  'use strict';

  // Mobile nav toggle
  const hamburger = document.querySelector('.nav-hamburger');
  const mobileMenu = document.querySelector('.nav-mobile-menu');
  if (hamburger && mobileMenu) {
    hamburger.addEventListener('click', () => {
      mobileMenu.classList.toggle('open');
    });
    // Close on link click
    mobileMenu.querySelectorAll('a').forEach((a) => {
      a.addEventListener('click', () => mobileMenu.classList.remove('open'));
    });
    // Close on outside click
    document.addEventListener('click', (e) => {
      if (!hamburger.contains(e.target) && !mobileMenu.contains(e.target)) {
        mobileMenu.classList.remove('open');
      }
    });
  }

  // Sidebar scroll-spy
  const sidebarLinks = document.querySelectorAll('.sidebar-link');
  if (sidebarLinks.length > 0) {
    const sections = [];
    sidebarLinks.forEach((link) => {
      const id = link.getAttribute('href')?.replace('#', '');
      if (id) {
        const el = document.getElementById(id);
        if (el) sections.push({ id, el, link });
      }
    });

    function updateActive() {
      const scrollY = window.scrollY + 100;
      let current = sections[0];
      for (const s of sections) {
        if (s.el.offsetTop <= scrollY) {
          current = s;
        }
      }
      sidebarLinks.forEach((l) => l.classList.remove('active'));
      if (current) current.link.classList.add('active');
    }

    window.addEventListener('scroll', updateActive, { passive: true });
    updateActive();
  }
})();
```

- [ ] **Step 2: Verify file size**

Run: `wc -l docs/main.js`
Expected: ~45 lines

- [ ] **Step 3: Commit**

```bash
git add docs/main.js
git commit -m "feat(docs): add mobile nav toggle and sidebar scroll-spy"
```

---

### Task 3: Create Landing Page `docs/index.html`

**Files:**
- Create: `docs/index.html`

- [ ] **Step 1: Write the landing page**

Write `docs/index.html`. The page structure (top to bottom):

1. `<!DOCTYPE html>` with `<html lang="en">`, charset utf-8, viewport meta, title "Orca Agent", link to `styles.css`
2. **Nav bar**: `.nav` with `.nav-logo` ("Orca" with `.gradient` class + "Agent" with `.light` class), `.nav-links` with "Docs" as `.active`, plus Quick Start, Providers, Tools, Architecture, CLI, Config links. `.nav-right` with version badge `v0.1.0` and GitHub SVG icon link
3. **Mobile menu**: `.nav-mobile-menu` with same links
4. **Hero section**: `.container > .hero` containing:
   - `.hero-badge`: "Open Source · Written in Rust"
   - `.hero-title`: `<span class="gradient">AI-Powered</span><br><span class="white">Coding Agent</span>`
   - `.hero-subtitle`: "Multi-provider support with OS-level sandboxing, semantic capacity control, and checkpoint recovery."
   - `.hero-buttons`: two `<a class="btn btn-primary">Get Started &rarr;</a>` and `<a class="btn btn-outline">` with inline GitHub SVG + "GitHub"
   - Terminal code block (`.code-block`) with dots, two prompt lines (`cargo install orca-agent`, `orca chat --provider deepseek`), agent response line
5. **Features section**: `.container > .section-title "Why Orca Agent?"` then `.card-grid.cols-3` with 6 cards:
   - OS-Level Sandboxing (lock emoji, `.purple` icon) — "Seatbelt on macOS, AppContainer on Windows, Landlock on Linux. Commands run isolated."
   - Semantic Capacity (brain emoji, `.cyan` icon) — "CanonicalState tracks goals, constraints, and facts. 3-checkpoint evaluation prevents drift."
   - Multi-Provider (plug emoji, `.green` icon) — "DeepSeek, Claude, OpenAI, plus any OpenAI-compatible provider (Ollama, Groq, vLLM...)"
   - Checkpoint Recovery (refresh emoji, `.amber` icon) — "Save session state to disk. Resume from any checkpoint. Never lose progress on long tasks."
   - 8 Built-in Tools (wrench emoji, `.red` icon) — "Filesystem, shell, git, web — with fuzzy name matching and argument repair for reliability."
   - Loop Guard (target emoji, `.purple` icon) — "Detects repetitive tool calls and offers self-correction. Mutation-aware window clearing."
6. **Stats bar**: `.container > .stats-bar` with 4 stat blocks (8,084 / Lines of Rust, 19 / Crates, 65 / Tests Passing, 3 / Providers)
7. **Footer**: `.footer` with brand and links
8. `<script src="main.js"></script>`, closing tags

- [ ] **Step 2: Open in browser and verify**

Run: `open docs/index.html`
Expected: Dark-themed landing page with hero, feature grid, stats, footer. Navigation bar visible at top.

- [ ] **Step 3: Commit**

```bash
git add docs/index.html
git commit -m "feat(docs): add landing page with hero, feature grid, stats, and footer"
```

---

### Task 4: Create Quick Start Page `docs/quickstart.html`

**Files:**
- Create: `docs/quickstart.html`

- [ ] **Step 1: Write the page**

Write `docs/quickstart.html` with the doc page layout. Nav "Quick Start" link is `.active`.

**Sidebar links** (with corresponding `id` sections):
- Overview
- Prerequisites
- Build from Source
- Set API Keys
- First Run
- TUI Mode
- Next Steps

**Content** (sourced from README.md and `main.rs`):

**Overview** section: "Orca Agent is a Rust-based AI coding agent with OS-level sandboxing, multi-provider support, and checkpoint recovery."

**Prerequisites** section: List Rust toolchain (1.70+), Git, one provider API key. Use `.doc-table` with two columns: Requirement / Details.

**Build from Source** section: Code block with `git clone` and `cargo build --release`. Info box tip about `--profile release` for faster builds.

**Set API Keys** section: Code block showing `export DEEPSEEK_API_KEY`, `export ANTHROPIC_API_KEY`, `export OPENAI_API_KEY`. Info box warning about never committing keys.

**First Run** section: Code block showing `./target/release/orca chat`, output showing "Orca vX.Y.Z - AI Coding Agent", prompt, and example interaction.

**TUI Mode** section: Code block showing `orca chat --tui`. Brief description of rich terminal UI.

**Next Steps** section: Link list to Providers, Tools, Architecture pages.

- [ ] **Step 2: Open in browser and verify**

Run: `open docs/quickstart.html`
Expected: Doc page with left sidebar, breadcrumb, gradient title, code blocks, info boxes. Sidebar scroll-spy highlights active section.

- [ ] **Step 3: Commit**

```bash
git add docs/quickstart.html
git commit -m "feat(docs): add quick start guide page"
```

---

### Task 5: Create Providers Page `docs/providers.html`

**Files:**
- Create: `docs/providers.html`

- [ ] **Step 1: Write the page**

Write `docs/providers.html` with the doc page layout. Nav "Providers" link is `.active`.

**Sidebar links:**
- Built-in Providers
- DeepSeek
- Anthropic (Claude)
- OpenAI
- Custom Providers
- Configuration
- Examples

**Content** (sourced from README.md):

**Built-in Providers** section: Intro paragraph + `.doc-table` with columns: Provider / CLI name / Models / API Key Env Var. Rows:
- DeepSeek / `deepseek` / deepseek-chat, deepseek-reasoner / `DEEPSEEK_API_KEY`
- Anthropic / `anthropic` or `claude` / claude-sonnet-4-20250514, claude-opus-4-20250514 / `ANTHROPIC_API_KEY`
- OpenAI / `openai` / gpt-4o, gpt-4o-mini, gpt-4.1, o1, o3, o3-mini / `OPENAI_API_KEY`

**DeepSeek** section: Brief description, code block showing `orca chat --provider deepseek --model deepseek-chat`.

**Anthropic** section: Code block showing `orca chat --provider anthropic --model claude-sonnet-4-20250514`.

**OpenAI** section: Code block showing `orca chat --provider openai --model gpt-4o`.

**Custom Providers** section: Explanation that any OpenAI-compatible API works via `--base-url`. Second `.doc-table` with third-party providers (Ollama, Groq, Together AI, OpenRouter, Azure OpenAI, vLLM, LiteLLM, Cloudflare Workers AI) showing base_url values.

**Configuration** section: Code block showing `config.toml` snippet for custom provider.

**Examples** section: Multiple code blocks showing Ollama, Groq, Together AI usage.

- [ ] **Step 2: Open in browser and verify**

Run: `open docs/providers.html`
Expected: Providers page with sidebar, tables, code blocks showing provider usage.

- [ ] **Step 3: Commit**

```bash
git add docs/providers.html
git commit -m "feat(docs): add provider configuration page"
```

---

### Task 6: Create Tools Page `docs/tools.html`

**Files:**
- Create: `docs/tools.html`

- [ ] **Step 1: Write the page**

Write `docs/tools.html` with the doc page layout. Nav "Tools" link is `.active`.

**Sidebar links:**
- Overview
- read_file
- write_file
- glob
- grep
- execute_shell
- git
- web_fetch
- web_search
- Tool Name Resolution
- Argument Repair

**Content:**

**Overview** section: Brief description of the 8 built-in tools, mention of sandbox status. Use a summary `.doc-table` with columns: Tool / Description / Sandboxed.

**Individual tool sections** (one `<div class="doc-section">` per tool): Each tool gets:
- `<h2>` with tool name
- Description paragraph
- Code block showing example usage (the tool name as it would appear in an agent call, with sample arguments)
- If sandboxed: info box tip explaining the sandbox restriction (command timeout, output truncation)

**Tool Name Resolution** section: The 5-step fuzzy matching process (exact → case-insensitive → hyphen normalization → CamelCase → prefix fuzzy). Numbered list.

**Argument Repair** section: The 6-stage JSON repair pipeline. Numbered list.

- [ ] **Step 2: Open in browser and verify**

Run: `open docs/tools.html`
Expected: Tools page with sidebar, individual tool sections, code examples.

- [ ] **Step 3: Commit**

```bash
git add docs/tools.html
git commit -m "feat(docs): add tools reference page"
```

---

### Task 7: Create Architecture Page `docs/architecture.html`

**Files:**
- Create: `docs/architecture.html`

- [ ] **Step 1: Write the page**

Write `docs/architecture.html` with the doc page layout. Nav "Architecture" link is `.active`.

**Sidebar links:**
- Overview
- Crate Layout
- Data Flow
- CanonicalState
- LoopGuard
- Seam Manager
- Tool Name Resolution
- Checkpoints

**Content** (sourced from README.md architecture section):

**Overview** section: High-level description of the agent loop architecture.

**Crate Layout** section: ASCII art tree from README.md showing the crate dependency graph. Use a code block with monospace styling. Each crate gets a brief one-line description below the tree.

**Data Flow** section: Description of request → provider → tool calls → execution → response cycle. Use a numbered list to describe the flow.

**CanonicalState** section: Four tracked dimensions (Goals, Constraints, Facts, Open Loops). Three evaluation checkpoints (PreRequest, PostTool, ErrorEscalation).

**LoopGuard** section: Sliding window analysis, mutation-aware clearing, graduated response levels.

**Seam Manager** section: Four-layer compression (Recent, Warm, Cool, Cold) with compression ratios and time windows. Info box noting that user anchors are preserved across compression.

**Tool Name Resolution** section: Brief recap with link to Tools page for details.

**Checkpoints** section: Brief description of checkpoint system with `orca resume` usage.

- [ ] **Step 2: Open in browser and verify**

Run: `open docs/architecture.html`
Expected: Architecture page with crate tree diagram, data flow description, component explanations.

- [ ] **Step 3: Commit**

```bash
git add docs/architecture.html
git commit -m "feat(docs): add architecture overview page"
```

---

### Task 8: Create CLI Page `docs/cli.html`

**Files:**
- Create: `docs/cli.html`

- [ ] **Step 1: Write the page**

Write `docs/cli.html` with the doc page layout. Nav "CLI" link is `.active`.

**Sidebar links:**
- Overview
- Global Flags
- chat
- config
- models
- resume
- In-Chat Commands

**Content** (sourced from `main.rs` Cli struct and Commands enum):

**Overview** section: Brief description of CLI structure (`orca <command> [flags]`).

**Global Flags** section: `.doc-table` with columns: Flag / Short / Description / Default. Rows from the Cli struct:
- `--workdir` / `-w` / Working directory / `.`
- `--model` / `-m` / Model to use / (from config)
- `--provider` / `-p` / Provider to use / (from config)
- `--verbose` / `-v` / Verbose logging / false
- `--prompt` / (none) / Initial prompt (non-interactive) / (none)
- `--tui` / (none) / Use TUI mode / false
- `--base-url` / (none) / Custom API base URL / (from config)

**chat** section: Code block examples: `orca chat`, `orca chat --tui`, `orca chat -p "explain this code"`, `orca chat --provider openai --model gpt-4o --base-url http://localhost:11434/v1`.

**config** section: Subcommands: `show`, `set <key> <value>`, `reset`. Code block examples for each.

**models** section: Code block showing `orca models` and example output listing all providers and models.

**resume** section: Code block showing `orca resume <checkpoint-id>`. Description of checkpoint resumption.

**In-Chat Commands** section: List of interactive commands: `/checkpoint` (create manual checkpoint), `/cost` (show cost and iteration count), `quit` / `exit` / `q` (exit session).

- [ ] **Step 2: Open in browser and verify**

Run: `open docs/cli.html`
Expected: CLI page with flag table, command examples, in-chat commands list.

- [ ] **Step 3: Commit**

```bash
git add docs/cli.html
git commit -m "feat(docs): add CLI reference page"
```

---

### Task 9: Create Config Page `docs/config.html`

**Files:**
- Create: `docs/config.html`

- [ ] **Step 1: Write the page**

Write `docs/config.html` with the doc page layout. Nav "Config" link is `.active`.

**Sidebar links:**
- Overview
- File Location
- Top-Level Settings
- Provider Settings
- Sandbox Settings
- Budget Settings
- Context Settings
- Complete Example

**Content** (sourced from README.md config section):

**Overview** section: Configuration is TOML-based at `~/.config/orca/config.toml`.

**File Location** section: Code block showing `~/.config/orca/config.toml`. Info box tip about `orca config show` to view current config.

**Top-Level Settings** section: `.doc-table` with columns: Key / Type / Default / Description. Rows:
- `provider` / string / `"deepseek"` / Active provider name
- `model` / string / `"deepseek-chat"` / Active model name
- `log_level` / string / `"info"` / Logging level (trace, debug, info, warn, error)

**Provider Settings** section: `[providers.<name>]` table format. `.doc-table`:
- `api_key` / string / (required) / API key (supports `$ENV_VAR` syntax)
- `base_url` / string / (provider default) / Custom API endpoint URL

**Sandbox Settings** section: `[sandbox]` table. `.doc-table`:
- `enabled` / bool / `true` / Enable OS-level sandboxing
- `exec_timeout_secs` / integer / `120` / Command execution timeout
- `max_exec_timeout_secs` / integer / `600` / Maximum allowed timeout
- `network_policy` / string / `"denied"` / Network access: denied, restricted, full

**Budget Settings** section: `[budget]` table. `.doc-table`:
- `max_session_budget_usd` / float / `10.0` / Max spend per session
- `max_iterations` / integer / `200` / Max agent loop iterations
- `rate_limit_per_minute` / integer / `60` / API calls per minute limit

**Context Settings** section: `[context]` table. `.doc-table`:
- `max_context_tokens` / integer / `128000` / Max context window size
- `compress_threshold` / float / `0.75` / Trigger compression at this % of max
- `max_checkpoints` / integer / `10` / Maximum stored checkpoints
- `use_flash_summary` / bool / `true` / Use flash summaries for compression

**Complete Example** section: Full `config.toml` code block from README.md showing all sections together.

- [ ] **Step 2: Open in browser and verify**

Run: `open docs/config.html`
Expected: Config page with sidebar, tables for each config section, complete example code block.

- [ ] **Step 3: Commit**

```bash
git add docs/config.html
git commit -m "feat(docs): add configuration reference page"
```

---

### Task 10: Final Polish and Cross-Page Verification

**Files:**
- Modify: all 7 HTML files (if nav link issues found)

- [ ] **Step 1: Verify all nav links work**

Check that every nav link across all 7 pages points to the correct file:
- Docs → `index.html` (landing page)
- Quick Start → `quickstart.html`
- Providers → `providers.html`
- Tools → `tools.html`
- Architecture → `architecture.html`
- CLI → `cli.html`
- Config → `config.html`

Expected: All links resolve, no 404s.

- [ ] **Step 2: Verify mobile responsiveness**

Open each page at 375px width (or use browser DevTools device toolbar). Verify:
- Hamburger menu appears
- Sidebar is hidden on doc pages
- Cards stack vertically
- Code blocks scroll horizontally
- Hero text is readable

Expected: All pages usable at mobile width.

- [ ] **Step 3: Verify footer links**

Check footer links on all pages point to correct destinations.

Expected: All footer links work.

- [ ] **Step 4: Final commit**

```bash
git add docs/
git commit -m "docs: finalize documentation site — all 7 pages complete"
```
