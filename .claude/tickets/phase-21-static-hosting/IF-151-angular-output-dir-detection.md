# IF-151: Angular output directory detection from angular.json

**Phase:** 21 — Static Hosting Expansion
**Priority:** Medium
**Estimate:** S
**Dependencies:** IF-147

## Description

Angular's build output directory is configurable via `angular.json` under `projects.<name>.architect.build.options.outputPath`. The default changed in Angular 17+ to `dist/<project-name>/browser/`. Detect the actual output path so the native deployer copies the correct directory.

## Acceptance Criteria

### Detection in `common/src/build/detect.rs`
- [ ] New function `detect_angular_output_dir(dir: &Path) -> Option<String>`
- [ ] Parses `angular.json`, reads `projects.*.architect.build.options.outputPath`
- [ ] If `outputPath` is found, use it
- [ ] If not found, use heuristic:
  - Check `dist/*/browser/` exists after build → use that
  - Fall back to `dist/` 
- [ ] Integrated into `framework_defaults()` for `Framework::Angular`

### Tests
- [ ] Detects output dir from angular.json with explicit outputPath
- [ ] Falls back to `dist/` when angular.json has no outputPath
- [ ] Handles angular.json with multiple projects (uses the default project)

## Out of Scope

- Angular SSR (Angular Universal) detection — future ticket
- Angular workspace with multiple apps

## Files to Modify

- `common/src/build/detect.rs` — `detect_angular_output_dir()` + integration
