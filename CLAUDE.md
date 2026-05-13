# Global Rules

Domain rules in `.claude/rules/` load by file path. Skills in `.claude/skills/` load by keyword. **If the user's question relates to a topic below but the rule wasn't auto-loaded, read the rule file before answering.**

## Project Stack

**On first interaction with any project, check for `.claude/stack.md`. If it doesn't exist, run the `stack-detect` skill before doing anything else.** This file defines what technologies the project actually uses — never assume a stack.

Preferences for NEW projects (when no existing codebase):
- Runtime: Bun. Package manager: Bun (yarn/npm OK for existing).
- Frontend: Astro 6 + Preact. Styling: CSS Modules. State: Nanostores.
- Backend: Hono. Database: PostgreSQL 16 + Drizzle ORM.
- Lint: Oxlint + Prettier.

These are defaults only. **Always defer to the detected stack in `.claude/stack.md` or project CLAUDE.md.**

## Rule Index

| Topic | Rule File | Read when user asks about... |
|---|---|---|
| frontend | `rules/frontend.md` | components, pages, islands, hydration, client-side |
| design | `rules/design.md` | spacing, typography, color, tokens, motion, states |
| backend | `rules/backend.md` | API routes, middleware, response shapes, server |
| database | `rules/database.md` | schema, migrations, queries, ORM, data modeling |
| security | `rules/security.md` | auth, sessions, encryption, headers, rate limiting |
| testing | `rules/testing.md` | tests, E2E, unit tests, integration, accessibility |
| accessibility | `rules/accessibility.md` | WCAG 2.2 AA baseline, alt text, labels, focus, contrast, ARIA — loads on any UI file |
| devops | `rules/devops.md` | Docker, CI/CD, deployment, backups, infra |
| laravel | `rules/laravel.md` | PHP, Laravel, Livewire, Blade, Eloquent |
| compliance | `rules/compliance.md` | GDPR, cookies, privacy, licenses, accessibility legal |
| incident | `rules/incident.md` | outage, postmortem, rollback, severity, status page |
| copywriting | `rules/copywriting.md` | labels, error messages, empty states, microcopy |
| git | `rules/git.md` | branches, commits, gitignore, push safety |
| ticketing | `rules/ticketing.md` | tickets, acceptance criteria, phases |
| planning | `rules/planning.md` | architecture decisions, ADRs, roadmap |
| research | `rules/research.md` | library eval, API eval, spikes |

## Accessibility is always on

**Default mode is accessibility-hawk.** On every UI change (new or edited component, page, form, media, widget) run the `a11y-hawk` skill before reporting done. Target **WCAG 2.2 Level AA** — this satisfies EAA (EN 301 549), UK PSBAR + Equality Act 2010, US ADA Title II (DOJ 2024 rule) / Title III, Section 508, and CVAA.

Route to the sub-skills for depth:
- POUR: `eaa-perceivable`, `eaa-operable`, `eaa-understandable`, `eaa-robust`
- ARIA deep-dive: `aria-authoring` (roles, attributes, widget patterns, naming, live regions)
- EAA extras: `eaa-products` (checkout/banking/transport/e-books), `eaa-statement`
- Jurisdictions: `uk-accessibility`, `us-ada`, `us-section-508`, `us-cvaa`
- AAA uplift is **opt-in only**: run `a11y-aaa` when the user explicitly asks and scopes it.

When you fix an a11y issue, add a one-line `a11y [<CRITERION>]: <why>` comment at the change site — see `rules/accessibility.md` for the convention — and report the fix with file, line, criterion, and what changed.

## Hard Constraints

Things Claude gets wrong without explicit instruction:

- **Icons: Tabler Icons by default** (`@tabler/icons-preact`), individual imports only. User can override with a different icon set. No emojis in UI.
- CSS custom properties for theming — never hardcode colors/spacing
- OKLCH for color definitions — not HSL, not hex (hex for final output only)
- Light + dark mode: design and validate both simultaneously
- Mobile-first: `min-width` media queries
- Only animate `transform` and `opacity` — never layout properties
- `@media (prefers-reduced-motion: reduce)` on all non-essential animation

## Anti-AI-Slop

If you showed the output to someone and said "AI made this," would they believe immediately? If yes, fix it.

- No Inter/Roboto/system font defaults without explicit user choice
- No purple-to-blue gradients, no cyan-on-dark palettes
- No cards-in-cards, no center-aligned everything
- No generic hero metric layouts (big number + small label)
- Tinted neutrals (OKLCH chroma 0.01-0.02), never pure gray

## Workflow

- Only change what was asked. No drive-by refactors.
- Read existing code before modifying.
- If unsure, ask. Do not assume.
- Git branches: `feature/TICKET-ID-short-desc`, `fix/short-desc`
- **Never commit or push without explicit user consent.** Always ask before running `git commit`, `git push`, `gh pr create`, or `gh pr merge`. Present what will be committed/pushed and wait for approval.
- **Merge conflicts**: If you encounter conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`), stop and show the user both sides. Do NOT resolve silently — ask which version to keep or whether to merge. If the user explicitly asks you to resolve, use the `merge-conflict` skill.

## Paired Subagent Sparring

When a skill calls for sparring, use the Agent tool to spawn parallel workers with opposing creative lenses. Always present results to the user for feedback before finalizing.
