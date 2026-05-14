# IF-218: Global search across all resources

**Phase:** 24 — Feature Parity
**Priority:** Medium
**Estimate:** S

## Description

IF-078 (Command Palette) provides Cmd+K fuzzy search with actions. This ticket extends it with a persistent search bar in the dashboard header that searches across ALL resource types: apps, databases, servers, projects, domains, deploys, and settings. The command palette is keyboard-driven for power users; this is a visible, always-accessible search for discoverability.

## Acceptance Criteria

- [ ] Search input in the dashboard header/navbar (visible on all pages)
- [ ] Searches across: apps (name, description), databases (name, engine), servers (name, hostname), projects (name), domains (FQDN), tags
- [ ] Results grouped by type with type icons
- [ ] Instant results as you type (debounced, <100ms response)
- [ ] Click result → navigate to resource detail page
- [ ] Keyboard navigation: arrow keys to select, Enter to navigate, Escape to close
- [ ] Recent searches stored in localStorage (last 5)
- [ ] Empty state: show recent items when search is empty and focused
- [ ] API endpoint: `GET /api/v1/search?q={query}` returns unified results across types
- [ ] Search is scoped to resources the current user can access (respects roles)
- [ ] Mobile: search accessible via icon button that expands to full-width input

## Technical Notes

- Backend: single SQL query with UNION across tables, or parallel queries merged in Rust
- SQLite FTS5 could be used for full-text search if needed, but LIKE queries with indexes are probably fast enough for <10k resources
- Frontend: reuse the fuzzy matching logic from IF-078 command palette
- Consider making the command palette and search bar share the same underlying search API

## Out of Scope

- Full-text search of log content
- Search within environment variable values
- Saved searches / search filters

## Dependencies

- IF-078 (Command palette — shared search infrastructure)
