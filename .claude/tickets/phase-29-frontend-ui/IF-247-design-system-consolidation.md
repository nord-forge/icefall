# IF-247: Design system consolidation and shared component library

**Phase:** 29 — Frontend UI
**Priority:** Critical
**Estimate:** L

## Description

Before building 20 new UI features, consolidate the existing component patterns into a proper shared component library. Currently there are 112 scattered form inputs, 39 table instances, 64 inline styles, and 27 non-shared card/panel usages. Centralizing these into reusable components will make the Phase 29 UI work faster and more consistent.

## Acceptance Criteria

### New Shared Components

- [ ] **Input** — text/email/password/number/url with label, error state, help text, reveal toggle for secrets
- [ ] **Textarea** — with label, character count, auto-resize option
- [ ] **Toggle** — switch component with label, description text
- [ ] **Card** — consistent panel component with optional header, footer, actions
- [ ] **Table** — sortable, with column headers, loading state, empty state, pagination
- [ ] **Badge** — colored pill for status, category, environment, tags
- [ ] **Tabs** — accessible tab panel with keyboard nav (consolidate AppTabs and other tab patterns)
- [ ] **Form** — form wrapper with submit handler, validation, loading state
- [ ] **Alert/Banner** — info/warning/error/success banners (consolidate OfflineServerBanner pattern)
- [ ] **Dropdown** — accessible dropdown menu (consolidate deploy dropdown pattern)
- [ ] **Timeline** — vertical timeline for config history, incidents, deploy events
- [ ] **CodeBlock** — syntax-highlighted code with copy button (for proxy config, compose, etc.)
- [ ] **Stat** — metric card with value, label, trend indicator

### Refactoring

- [ ] Replace all inline `<input>` with `<Input>` component
- [ ] Replace all inline `<table>` with `<Table>` component
- [ ] Replace all card/panel divs with `<Card>` component
- [ ] Remove all inline styles (move to CSS modules)
- [ ] Consolidate all status indicators to use StatusDot + Badge
- [ ] Ensure every interactive element has `focus-visible` styling
- [ ] Ensure every form input has a visible `<label>`

### Accessibility Baseline

- [ ] All shared components meet WCAG 2.2 AA
- [ ] Every component supports keyboard navigation
- [ ] Color contrast: 4.5:1 for text, 3:1 for UI components
- [ ] All icons paired with text labels (no icon-only buttons without aria-label)
- [ ] Focus management: modals trap focus, closing returns focus
- [ ] Screen reader: status changes use aria-live regions
- [ ] Reduced motion: respect prefers-reduced-motion

### Design Tokens

- [ ] All shared components use CSS custom properties from the token system
- [ ] No hardcoded colors, spacing, or font sizes in component CSS
- [ ] Dark mode support via token switching (not component-level logic)

## Technical Notes

- Keep existing components working during migration — replace incrementally
- Each new shared component gets its own directory: `islands/shared/{Name}/{Name}.tsx` + `.module.css`
- Export all shared components from a barrel file: `islands/shared/index.ts`
- Document each component with usage examples in Storybook or a `/components` preview page

## Out of Scope

- Full Storybook setup (a simple components page is sufficient)
- React compatibility layer
- Component library publishing (it's internal only)

## Dependencies

- None (this blocks all other Phase 29 tickets)
