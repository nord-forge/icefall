# IF-162: Deploy by git tag

**Phase:** 25 — Parity Gaps
**Priority:** Medium
**Estimate:** S

## Description

Allow users to trigger a deploy from a specific git tag instead of a branch. The deploy form and API accept an optional `tag` parameter. When specified, the build pipeline checks out that tag instead of the configured branch.

## Acceptance Criteria

- [ ] `POST /apps/{id}/deploy` accepts optional `tag` field (e.g., `v1.2.3`)
- [ ] When `tag` is provided, the build pipeline runs `git checkout tags/{tag}` after clone
- [ ] Deploy record stores the tag in a new `tag` column (nullable)
- [ ] DeploysTab shows the tag badge next to the commit SHA when deployed from a tag
- [ ] App overview: "Deploy" dropdown with options: "Deploy latest" (branch HEAD) and "Deploy tag..." (opens tag input)
- [ ] Tag input with autocomplete from `git ls-remote --tags`
- [ ] Webhook support: deploy on tag push events (GitHub `create` event with `ref_type: tag`)

## Technical Notes

- The clone step in `orchestrator.rs` already supports branch checkout — extend it to accept a tag
- Git tags and branches are both refs — `git checkout refs/tags/{tag}` is unambiguous
- Tag autocomplete requires a `git ls-remote --tags {repo_url}` call — cache for 5 minutes

## Dependencies

- IF-010 (Image build orchestrator)
- IF-012 (Webhook receiver)
