# Plan: Git source → detect-and-resolve deploy method (+ Podman, monorepo, compose/Dockerfile pickup)

## Problem (confirmed in code)

1. **Build Settings fields are discarded.** `dashboard/src/islands/app-create/AppCreateWizard/AppCreateWizard.tsx:169-172` — the Git branch of `handleDeploy()` sends only `git_repo` + `git_branch`. The `build_command` / `output_dir` / `start_command` / `port` the user typed are never sent.
2. **No detection until deploy.** `detect()` (`common/src/build/detect.rs`) only runs at deploy time inside the clone. The user can't see or correct the resolved deploy method before committing.
3. **"Git" never resolves to a deploy method** in the UI — it always proceeds as build-from-source, even when the repo is really Dockerfile- or compose-based.

## Reproduction fixture: kaartje (`github.com/NickBevers/kaartje`)

A workspace monorepo (`"workspaces": ["packages/*"]`). Traced through `detect.rs` at repo root:

- `detect_framework`: no literal `Dockerfile` (has `Dockerfile.api`/`Dockerfile.web`); root `package.json` deps are `oxlint/oxfmt/expo/react/react-native` → no astro/next/nuxt/vite; no `scripts.start`, no `main` → **falls through to `StaticSite`**.
- `detect_package_manager`: `package-lock.json` → `Npm`.
- `framework_defaults(StaticSite)` → `(None, ".", None, 80)`.
- `should_use_native(StaticSite)` → **true** → native static deployer serves the **repo root** as static files.

**Result: silently wrong.** No build runs, no service starts; icefall serves README/source as a "static site." The real services live in `packages/web` (Astro→Caddy, port 4321) and `packages/api` (Bun, port 3000), and the intended deploy is `docker-compose.prod.yml`. kaartje breaks detection in **three** independent ways → these become acceptance criteria.

---

## Folded-in findings → Acceptance Criteria

### AC1 — Compose detection at repo root
- `detect()` (or the new endpoint layer) flags `docker-compose.yml`, `docker-compose.*.yml`, `compose.yaml`, `compose.yml` at root.
- Endpoint returns `has_compose: true` + list of compose file names + `suggested_deploy_source: "compose"`.
- Wizard offers "Deploy as Compose stack instead?" and pre-loads the chosen file into `form.compose_content`.
- **kaartje expectation:** root `docker-compose.yml` (dev) and `docker-compose.prod.yml` both detected; user picks `prod`.

### AC2 — Variant Dockerfile detection
- Detection recognizes `Dockerfile` **and** `Dockerfile.*` (e.g. `Dockerfile.api`, `Dockerfile.web`).
- When only variant Dockerfiles exist (no plain `Dockerfile`), surface them rather than silently missing → `has_dockerfile: true`, `dockerfiles: ["Dockerfile.api","Dockerfile.web"]`.
- If a single `Dockerfile` exists, behavior unchanged (`Framework::Dockerfile`). If multiple variants, the wizard must ask which (or steer to compose), because a single build target is ambiguous.
- **kaartje expectation:** both variant Dockerfiles listed; wizard does not auto-pick one.

### AC3 — Monorepo signal + base_directory
- If root `package.json` has a `workspaces` field AND no deployable app resolves at root, return `is_monorepo: true` + discovered workspace dirs (glob `packages/*`).
- Wizard prompts for `base_directory` (which workspace) and **re-runs detection** against that subdir before showing Build Settings.
- `base_directory` is persisted on the app (field already exists in the apps model) and passed to deploy-time `detect()`.
- **kaartje expectation:** `is_monorepo: true`, workspaces `packages/api`, `packages/web`, `packages/shared`, `packages/mobile`; choosing `packages/web` detects Astro, `packages/api` detects Node/Bun.
- **Guardrail:** never silently ship repo root as `StaticSite` when `is_monorepo` is true — block and prompt instead.

### AC4 — Honest create (the discard bug)
- Build overrides (`build_command`/`output_dir`/`start_command`/`port`) + `base_directory` are sent on app create and persisted into `build_config`. Deploy-time `detect()` overrides still win via `apply_overrides` (detect.rs:242). Blank fields fall back to detection.

### AC5 — Reachability gate
- `POST /apps/detect` checks repo reachability first: private + `github_installation_id` → verify via installation token (pattern in `git_sources.rs:55`); public → `git ls-remote`. Typed errors (`unreachable` / `auth_required`) drive wizard UX.
- **Reconciliation with #107 (already merged):** a repo public/private probe shipped — `GET /github/repo-status?url=…` (`src/api/routes/github/setup.rs:43`) + `GitHubClient::repo_status`/`repo_visibility` (`src/github/client.rs`) + `api.getRepoStatus` + a debounced probe in `GitRepoStep`. AC5 does **not** reimplement reachability: the wizard already knows public/private/missing by the time the user leaves the Repository step. The detect endpoint trusts that signal and uses `list_remote_branches` (`git.rs:161`) only as the clone-time reachability error path (public) / the installation token (private). No duplicate probe.

### AC6 — Detect & strip foreign-platform coupling on compose import
**Rationale:** a compose file authored for a *different* hosting platform isn't broken Podman/Docker — it's coupled to infrastructure that platform provides (an external proxy network, that platform's routing labels, a sidecar that pokes that platform's proxy). On any plain host (Docker *or* Podman) it fails the same way: the external network/proxy isn't there. kaartje's `docker-compose.prod.yml` is a clean reproduction. Rather than let the user paste it and watch it fail, icefall detects the coupling and offers to strip it.

- **Detect** these foreign-coupling signals when a compose file is pasted/imported (Compose source, AC1 prefill, or paste in the Compose step):
  - **External proxy networks** — top-level `networks.<name>.external: true` that isn't created by icefall. On a fresh host these error identically on `docker compose` and `podman compose` (`external network ... not found`). Flag each external network and which services join it.
  - **Foreign routing labels** — service `labels` for proxies icefall doesn't run (e.g. `traefik.*`, and any other-platform routing/magic-comment labels). Inert under icefall's Caddy-based proxy.
  - **Proxy-control sidecars** — services whose sole purpose is restarting/reloading another platform's proxy: tell-tale = mounts the container socket (`/var/run/docker.sock`, `/run/podman/podman.sock`) AND runs a proxy/`*:cli` image AND targets a container icefall didn't create. (The socket mount itself is portable to Podman via path swap — we strip because it targets *foreign* infra, not because of the runtime.)
- **Report, don't silently rewrite.** Return a `foreign_coupling` block: `{ external_networks: [...], foreign_labels: [{service, keys}], proxy_sidecars: [service] }`. Wizard shows: *"This compose file looks built for another platform. icefall can route this for you — remove the external `<name>` network, `traefik.*` labels, and the `<sidecar>` service?"* with a **diff preview** and **[Strip & continue] / [Keep as-is]**.
- **Strip transform** (only on explicit user opt-in): remove flagged external networks + the `networks:` references to them from services; remove foreign routing labels; remove proxy-control sidecars and any now-dangling `depends_on` referencing them. Leave everything else byte-for-byte. Re-validate the YAML after.
- **Never auto-strip** — destructive edits to user-pasted config require consent (matches repo CLAUDE.md "never resolve/overwrite silently" posture).
- **kaartje expectation:** on `docker-compose.prod.yml`, flags external network `coolify`, `traefik.*` labels on `api`/`web`, and the `proxy-reload` sidecar; after strip, `api`+`web`+`libsql` remain and deploy under icefall's proxy. `docker-compose.yml` (dev) flags nothing.
- **Constraint:** report copy must describe couplings generically (external proxy network / foreign routing labels / proxy-control sidecar) — do not hardcode or surface the competing platform's brand name in code, UI strings, or fixtures.

---

## Backend

### New endpoint `POST /apps/detect` — `src/api/routes/apps/detect.rs`
Request: `{ git_repo, git_branch?, github_installation_id?, base_directory? }`
- Auth via `authenticate_from_headers`.
- Reachability check (AC5).
- **Shallow clone** `--depth=1 --filter=blob:none` to tempdir, run `detect()` against `base_directory` (or root). (Recommended over Contents API: zero drift vs deploy-time detect; GitHub-agnostic.)
- Compute AC1/AC2/AC3 hints (compose files, variant Dockerfiles, workspaces).
- Return serialized `DetectionResult` + hints (`#[derive(Serialize)]` on `DetectionResult` if missing).

### Detection engine changes — `common/src/build/detect.rs`
- AC2: `detect_framework` matches `Dockerfile*` (track which; if >1 and no plain `Dockerfile`, do **not** auto-resolve — let the endpoint mark ambiguous).
- AC1/AC3: helper to enumerate root compose files and `workspaces` globs (used by the endpoint layer; keep `detect()` core return shape stable, expose hints alongside).
- Add unit tests for: compose-at-root, variant-Dockerfile-only, monorepo-workspaces, kaartje-root (asserts NOT silently StaticSite when monorepo).

### Create request — `src/api/routes/apps/crud.rs`
- Extend `CreateAppRequest` with optional `build_command`, `output_dir`, `start_command`, `port`, `base_directory`; persist into `build_config` (AC4).

### Foreign-coupling analyzer (AC6) — `common/src/build/compose_audit.rs` (new)
- `analyze_foreign_coupling(yaml: &str) -> ForeignCoupling` — parse compose YAML (serde_yaml), return `{ external_networks, foreign_labels, proxy_sidecars }`. Pure function, no I/O → unit-testable with fixtures.
- `strip_foreign_coupling(yaml: &str, selections) -> Result<String>` — apply only user-selected removals; re-parse to validate; preserve unrelated content. Round-trip note: serde_yaml reserializes (won't be byte-identical); acceptable since the user reviews a diff. If comment-preservation matters, evaluate `yaml-rust`/AST-edit later — flagged, not required for v1.
- Wire into the detect endpoint response (when `has_compose`) and into a `POST /apps/compose/audit` (or reuse detect) for the Compose-step paste path.
- Label matcher is a configurable prefix list (`traefik.`, plus an extensible set) so new foreign platforms can be added without code changes — keep the list in config, not hardcoded brand strings.
- Tests: kaartje `docker-compose.prod.yml` fixture (flags `coolify` net + `traefik.*` + `proxy-reload`; strip yields valid 3-service stack), kaartje `docker-compose.yml` (flags nothing), and a no-coupling control. Fixtures must not carry the competitor brand in identifiers beyond the verbatim file content under test.

---

## Frontend — `AppCreateWizard.tsx`

- New "Detecting…" step between Repository and Build Settings: call `api.detectApp(...)`, render resolution card ("Detected: Astro (static) · npm · port 80 → static deploy" / "Will build from Dockerfile" / "Compose stack").
- Pre-fill Build Settings from response (only if user hasn't typed); fields become editable overrides, not dead placeholders.
- AC1 prompt: offer switch to Compose source, load file content.
- AC3 prompt: monorepo → pick workspace (`base_directory`), re-detect.
- AC5: `reachable:false` blocks Next, routes to existing "Connect GitHub" affordance.
- Fix `handleDeploy()` Git branch to send build overrides + `base_directory` (AC4).

---

## Podman compatibility (kaartje compose audit)

icefall talks to the runtime via `bollard` socket API and runs `podman compose` when `runtime=podman` (config `compose_command()`), so icefall itself is Podman-ready. But kaartje's **compose files** are not all drop-in:

### `docker-compose.prod.yml` — needs edits (NOT drop-in)
- **BLOCKER:** `networks: coolify` declared `external` → won't exist on host → compose fails. Remove/rename.
- **BLOCKER:** `proxy-reload` service (`image: docker:cli`, bind-mounts `/var/run/docker.sock`, restarts another platform's proxy) → Podman socket path/perm model differs (esp. rootless) AND targets a proxy icefall doesn't run. **Delete it.**
- **High:** `traefik.*` labels on `api`/`web` → icefall uses Caddy + its own proxy scheme; labels are inert. Strip; let icefall handle routing.
- OK: `libsql` (ghcr.io image), ports 3000/4321 (≥1024, rootless-safe), named `libsql-data` volume, build contexts (build under `podman compose`).
- After stripping the above, remaining `api`+`web`+`libsql` is Podman-clean.

### `docker-compose.yml` (dev) — Podman-clean / drop-in
- `minio`, `minio-init`, `libsql`; named volumes; ports 9000/9001/8080. No Traefik, no external net, no socket mounts. But this is **backing services only** — doesn't build/run the app.

### Dockerfiles — Podman-clean
- `Dockerfile.api`: multi-stage `node:22-slim`→`oven/bun:1`, `EXPOSE 3000`, runs `bun run packages/api/src/index.ts`. No USER (root-in-userns under rootless Podman = fine, no privileged ops, no socket mount).
- `Dockerfile.web`: `node:22-slim` build → `caddy:2`, `EXPOSE 4321`, serves `dist` via Caddy. Clean.
- Note: web listens on **4321** and api on **3000** — neither matches root detection's guessed `80`/`StaticSite`, reinforcing AC2/AC3.

### Is the repo "wrong for Podman"? — No. It's wrong for *icefall*.
Research verdict (2025-26 official Podman/Red Hat/Traefik sources): nothing in kaartje's compose is a Podman *portability* defect. Each item I previously flagged is Docker-convention-that-Podman-supports, OR a coupling to a different PaaS — not a Docker-vs-Podman issue:

- **`networks: coolify` external:** `external: true` has identical semantics on `podman compose` and `docker compose` — both error if the network doesn't pre-exist (podman-compose #1127: `External network does not exist`). So this isn't "Docker-only"; it's "assumes another platform created the network." Wrong for *this host*, not wrong for Podman.
- **`proxy-reload` socket mount:** the `/var/run/docker.sock` bind-mount is the one genuine Docker-ism — Podman is daemonless, no docker.sock. BUT it's not broken, just needs the Podman socket path (`/run/podman/podman.sock` rootful, `$XDG_RUNTIME_DIR/podman/podman.sock` rootless) mounted to `/var/run/docker.sock:z`, with `podman.socket` enabled. The `docker:cli` image works against Podman's Docker-API-compat layer (v1.40) for `restart`. **However** — it targets *another platform's* proxy container, so for icefall it's deleted regardless of runtime. Podman-portable, icefall-irrelevant.
- **`traefik.*` labels:** meaningful under Podman — Traefik's *docker* provider reads them over the Podman socket (no separate Podman provider needed). Not Docker-only. But icefall uses Caddy + its own scheme, so the labels are inert *here*. Wrong for icefall's proxy, not wrong for Podman.
- **Dockerfiles / ports / volumes:** fully Podman-clean. Multi-stage builds via Buildah, named volumes, ghcr.io pulls all work. Root-in-container with no `USER` is the *safe* rootless case (maps to host user via userns). Ports 3000/4321 are ≥1024 → rootless-safe with zero config.

**Conclusion: the repo is not Podman-incompatible.** It's a Docker-targeted file coupled to a specific competing PaaS (its external proxy network + Traefik routing + a sidecar that restarts that PaaS's proxy). On a plain Podman host it would fail the same way it'd fail on a plain Docker host — because the external network and proxy aren't there, not because of Podman.

### Recommended path for deploying kaartje on icefall *today*
Use Compose source with a **trimmed** `docker-compose.prod.yml`: drop the `coolify` external network, delete `proxy-reload`, strip `traefik.*` labels — i.e. remove the other-PaaS coupling and let icefall's own proxy route. Runs as `podman compose` on the user's runtime. (One rootless caveat to verify: if the user later wants any service on :80/:443, that needs `ip_unprivileged_port_start` lowering — but kaartje binds 3000/4321, so N/A.)

---

## Out of scope / follow-ups
- Auto-rewriting third-party-PaaS compose files (Traefik→Caddy label translation) — detect & warn only, for now.
- Picking among multiple compose files automatically — present choice, don't guess.

## Resolved decisions

### D1 — Detection fetch method → **Shallow clone. Decided.**
Reuse the existing `src/build/git.rs` primitives — no new fetch path:
- `clone_repo(&GitCloneOptions { shallow: true, branch, token, .. }, tempdir)` is the *same* call deploy-time auto-mode already makes (`deploys/operations.rs:300-321`). Detect-preview and deploy-time detect therefore run identical code against identical bytes → **zero drift**, which was the whole point.
- Private repos already work: `inject_token_into_url` (`git.rs:185`) injects the installation token into the HTTPS URL exactly as the deploy path does. The detect endpoint resolves the token via `get_valid_installation_token` (same as `git_sources.rs:55`) and passes it through.
- Reachability (AC5) reuses `list_remote_branches` (`git.rs:161`) = a `git ls-remote` that already returns a surfaced error for unreachable/unauthorized repos. So the gate is a function that exists, not new code.
- **Why not Contents API:** its only advantage was "avoid a clone," but the clone path is already written, already handles auth/tokens/branches, and guarantees parity. Contents API would be GitHub-only and a second code path to keep in sync. Rejected.
- Cost control: detect always sets `shallow: true` and `submodules:false`/`lfs:false` (detection reads root + one subdir of plain files; submodules/LFS are irrelevant to framework detection and add latency). Clone to a tempdir, run detection + compose audit, `remove_dir_all` immediately (mirrors `operations.rs:328`).

### D2 — Override persistence → **Extend `CreateAppRequest`. Decided.**
Add optional `build_command`/`output_dir`/`start_command`/`port`/`base_directory` to `CreateAppRequest` (`apps/crud.rs`), persisted into `build_config` at create.
- **Why not follow-up `updateApp`:** the wizard already fires post-create `updateApp` calls for `project_id`/`github_installation_id` (`AppCreateWizard.tsx:181/187`), so a third would "work" — but it leaves a window where the app exists with no build config, and makes the API dishonest on its own (create accepts a Git app it can't fully describe). One atomic create is correct; the create endpoint should be complete without relying on the client to immediately patch it.
- Deploy-time `detect()` still runs and overrides win via `apply_overrides` (`detect.rs:242`); blank fields fall back to detection. So persisting partial overrides is safe — it's additive, never lossy.

### D3 — Ambiguous multi-Dockerfile / monorepo → **Block-and-prompt on true ambiguity; warn-and-proceed when a confident default exists. Decided.**
Tiered, not one-size:
- **Monorepo with no root-deployable app (kaartje):** **hard block.** `is_monorepo:true` + root resolves to `StaticSite`-of-repo-root is always wrong. Wizard must require a `base_directory` selection before Next; no silent proceed. (This is the AC3 guardrail — now the canonical "block" case.)
- **Multiple variant Dockerfiles, no plain `Dockerfile` (kaartje `Dockerfile.api`/`.web`):** **block-and-prompt** — there's no non-arbitrary way to pick a single build target. Wizard lists them and asks which (or steer to the compose file, which is the better answer for kaartje). Never auto-pick.
- **One plain `Dockerfile` (+ optional variants):** **warn-and-proceed** — default to the plain `Dockerfile` (current `Framework::Dockerfile` behavior), surface a one-line note that variants also exist, let the user override. A confident default exists, so don't block.
- **Single framework detected, no ambiguity:** proceed, fields pre-filled, no prompt.
- **Foreign-coupling (AC6):** always **prompt** (never auto-strip) — but that's a consent gate on a destructive edit, orthogonal to detection ambiguity.
- Principle: **block only when proceeding would be silently wrong; warn when a safe default exists.** Every block must state why and what to pick.
