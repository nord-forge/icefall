# IF-246: README overhaul, docs website setup, and documentation update

**Phase:** 29 — Frontend UI
**Priority:** High
**Estimate:** M

## Description

Update the repository README to reflect the current feature set, set up the Starlight docs website for production deployment, and ensure all documentation pages are accurate and cross-linked.

## Acceptance Criteria

### README
- [ ] Hero section: what Icefall is, one-liner install, key stats (220 tickets, 27 MCP tools, etc.)
- [ ] Feature comparison table (vs comparable platforms)
- [ ] Architecture diagram (single binary + SQLite + Caddy + Docker/Podman)
- [ ] Screenshot of the dashboard
- [ ] Quick start section (install → deploy first app)
- [ ] Links to docs, CLI reference, MCP guide
- [ ] Contributing section
- [ ] License

### Docs Website
- [ ] Starlight config: site title, logo, sidebar navigation matching all 79+ pages
- [ ] Search working (Pagefind)
- [ ] Deploy docs site (static build, can be hosted on Icefall itself)
- [ ] Sidebar organized by section: Getting Started, Concepts, Frameworks, Guides, API, Troubleshooting, Reference
- [ ] All pages cross-linked with "Next steps" at bottom
- [ ] Edit links pointing to GitHub

### Documentation Update
- [ ] Verify all 79 pages have accurate content matching current features
- [ ] Add missing pages for features added in Phases 24-28
- [ ] Ensure Docker/Podman dual coverage in all relevant pages
- [ ] Remove any stale/incorrect information

## Dependencies

- All previous phases (documentation reflects final feature set)
