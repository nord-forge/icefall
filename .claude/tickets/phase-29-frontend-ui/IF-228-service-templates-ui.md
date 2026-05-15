# IF-228: One-click service templates browser UI

**Phase:** 29 — Frontend UI
**Priority:** High
**Estimate:** L

## Description

Build the template browser and deploy flow for one-click services (IF-148). Users see a grid of available services, can search/filter by category, and deploy with auto-generated config forms.

## Acceptance Criteria

- [ ] New sidebar nav item: "Services" (or "Templates")
- [ ] Grid of template cards: icon, name, description, category badge, version
- [ ] Category filter chips: All, AI/ML, Analytics, CMS, Communication, Database, DevTools, Media, Monitoring, Productivity, Security, Storage
- [ ] Search by name
- [ ] Template detail drawer: description, README content, resource requirements, version
- [ ] Deploy form: auto-generated from `required_inputs` — text, email, URL, password field types
- [ ] Server selection (if multi-server) + project assignment
- [ ] Deploy button → creates Compose app and redirects to deploy view
- [ ] "Deployed" badge on cards for templates user already has running
- [ ] "Update available" indicator when template version changes
- [ ] a11y: card grid navigable by keyboard, form validation announced

## Dependencies

- IF-148 (Service templates backend)
