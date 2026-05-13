# IF-177: Deploy preview screenshots

**Phase:** 25 — Icefall+
**Priority:** Medium
**Estimate:** M

## Description

Automatically capture a screenshot of the deployed app after each successful deploy and display it in the deploy history. Provides a visual timeline of how the app changed over time. Useful for catching visual regressions and for teams where non-technical stakeholders want to see what was deployed.

## Acceptance Criteria

- [ ] After a deploy succeeds and health check passes: capture a screenshot of the app's primary domain
- [ ] Screenshot taken via a headless browser (Chromium) running in a lightweight container
- [ ] Screenshot stored locally (compressed PNG, max 1280×720)
- [ ] Deploy detail: thumbnail of the screenshot with click-to-expand
- [ ] Deploy history: small thumbnail per deploy for visual comparison
- [ ] Compare view: side-by-side screenshots of two deploys
- [ ] Screenshots stored in `data/screenshots/{deploy_id}.png`
- [ ] Retention: keep screenshots for the last 20 deploys per app, auto-cleanup older ones
- [ ] Optional: disable per-app in settings (default: enabled if headless browser is available)
- [ ] Graceful degradation: if headless browser is not installed, skip screenshot silently

## Technical Notes

- Use `chromiumoxide` Rust crate or shell out to `chromium --headless --screenshot`
- The screenshot container/binary needs to be lightweight — consider using `playwright` or `puppeteer-core` with a pre-installed Chromium
- Wait for page load + 2 seconds before capturing (let JS render)
- For preview environments: capture the preview URL
- For multi-server: screenshot taken by the control plane (it has network access to all domains)

## Out of Scope

- Visual diff (pixel comparison between deploys)
- Full-page screenshots (just the viewport)
- Multiple viewport sizes (just desktop 1280×720)

## Dependencies

- IF-011 (Container deployment — trigger after success)
- IF-023 (Domain management — needs the app's URL)
