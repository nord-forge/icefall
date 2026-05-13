# IF-078: Command palette

**Phase:** 16 — v1.1 Fast Follow
**Priority:** Medium
**Estimate:** M

## Description

Keyboard-driven global search with quick actions. Once the platform has projects, tags, and 10+ apps, fast navigation is essential. `Cmd+K` (macOS) / `Ctrl+K` (Linux/Windows) opens a palette for searching and executing actions.

## Acceptance Criteria

### Trigger
- [ ] `Cmd+K` / `Ctrl+K` keyboard shortcut opens the palette from anywhere in the dashboard
- [ ] Palette overlays the current page (modal with backdrop)
- [ ] `Escape` closes the palette
- [ ] Clicking outside the palette closes it

### Search
- [ ] Search input with autofocus
- [ ] Fuzzy search across:
  - App names
  - Database names
  - Domain names
  - Project names (if IF-074 is done)
  - Tag names (if IF-072 is done)
- [ ] Results grouped by type: "Apps", "Databases", "Domains", "Pages"
- [ ] Each result shows: icon (type indicator), name, status badge, project (if assigned)
- [ ] Keyboard navigation: arrow keys to move, Enter to select
- [ ] Max 10 results shown (scrollable if more)

### Quick Actions
- [ ] Type `>` prefix to switch to action mode (like VS Code)
- [ ] Available actions:
  - `> Deploy {app-name}` — trigger a deployment
  - `> Restart {app-name}` — restart a container
  - `> Stop {app-name}` — stop a container
  - `> View logs {app-name}` — navigate to app logs tab
  - `> New app` — navigate to app creation
  - `> New database` — navigate to database creation
  - `> Settings` — navigate to settings page
- [ ] Actions show confirmation before executing destructive operations (stop)

### Recent Items
- [ ] When palette opens with empty input, show "Recent" section:
  - Last 5 visited apps/pages
  - Last 3 executed actions
- [ ] Recent items stored in localStorage

### Navigation Results
- [ ] Static pages always available:
  - Dashboard Home
  - Databases
  - Server
  - Users
  - Settings
  - Domains

### General
- [ ] Light and dark theme verified
- [ ] Mobile: accessible via a search icon button in the header (no keyboard shortcut)
- [ ] Palette animation: fade in + slight scale up (respects `prefers-reduced-motion`)

## Technical Notes

- Build as a standalone Preact island that mounts at the layout level (always available)
- Fetch app/database lists from existing nanostores (`apps.ts`, `databases.ts`)
- Fuzzy search: use a simple substring match or a lightweight library like `fuse.js` (< 10KB)
- Action execution: call existing API endpoints (deploy, restart, stop)
- Store recent items in localStorage with a key like `icefall_recent_items`

## Out of Scope

- Plugin/extension actions
- Command history (just recent items)
- Custom keyboard shortcut configuration
- Admin commands (user management, settings changes)

## Dependencies

- IF-016 (Astro project setup — layout level mount)
