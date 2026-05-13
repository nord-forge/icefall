# IF-152: Ensure Node.js / Bun available for native builds

**Phase:** 21 — Static Hosting Expansion
**Priority:** High
**Estimate:** S
**Dependencies:** None

## Description

The native deploy pipeline (`src/deploy/native.rs`) runs `npm ci` / `yarn install` / `pnpm install` / `bun install` and `npm run build` directly on the host. This assumes Node.js (or Bun) is installed on the server. Currently nothing validates this, and there's no clear error if the runtime is missing.

Add a pre-flight check before native builds that verifies the required runtime is available, and provide a clear error message if it's not. Optionally, document or automate runtime installation via the install script.

## Acceptance Criteria

### Pre-flight check in `src/deploy/native.rs`
- [ ] Before `run_command(install_cmd)`, verify the package manager binary exists on PATH
- [ ] If missing, fail with a descriptive error: "Native deploy requires {node/bun} to be installed on the server. Install it or use deploy_mode: container."
- [ ] Check Node version matches the detected/required version (warn if mismatch, don't block)

### Install script
- [ ] `install.sh` installs Node.js LTS (via NodeSource or fnm) when Icefall is configured for native deploys
- [ ] Install is opt-in (flag or prompt), not forced on all servers

### Tests
- [ ] Unit test: pre-flight check fails gracefully when binary not found
- [ ] Unit test: pre-flight check passes when binary exists

## Out of Scope

- Automatic Node version switching (nvm/fnm) per-deploy
- Sandboxing native builds

## Files to Modify

- `src/deploy/native.rs` — add pre-flight runtime check
- `install.sh` — optional Node.js installation step
