# IF-207: Podman reference documentation

**Phase:** 28 — Comprehensive Docs
**Priority:** High
**Estimate:** M

## Description

Create a comprehensive Podman reference section in the docs. Every Docker command or concept referenced anywhere in the docs must have a Podman equivalent. This is the page users land on when they search "icefall podman" — it should answer everything.

## Pages to Create

### `reference/podman.mdx` — Podman Command Reference
Full command translation table between Docker and Podman for every operation Icefall users might need:

- [ ] **Container operations**:
  | Docker | Podman |
  |--------|--------|
  | `docker ps` | `podman ps` |
  | `docker run` | `podman run` |
  | `docker stop` | `podman stop` |
  | `docker rm` | `podman rm` |
  | `docker exec -it` | `podman exec -it` |
  | `docker logs` | `podman logs` |
  | `docker inspect` | `podman inspect` |
  | `docker stats` | `podman stats` |

- [ ] **Image operations**:
  | Docker | Podman |
  |--------|--------|
  | `docker build` | `podman build` (uses Buildah) |
  | `docker pull` | `podman pull` |
  | `docker push` | `podman push` |
  | `docker images` | `podman images` |
  | `docker rmi` | `podman rmi` |
  | `docker tag` | `podman tag` |

- [ ] **Network operations**:
  | Docker | Podman |
  |--------|--------|
  | `docker network create` | `podman network create` |
  | `docker network ls` | `podman network ls` |
  | `docker network connect` | `podman network connect` |
  | Bridge driver | netavark (default in Podman 4.0+) |
  | Automatic container DNS | aardvark-dns (requires named network) |

- [ ] **Volume operations**:
  | Docker | Podman |
  |--------|--------|
  | `docker volume create` | `podman volume create` |
  | `docker volume ls` | `podman volume ls` |
  | `-v /host:/container` | `-v /host:/container` (add `:Z` for SELinux) |

- [ ] **Compose**:
  | Docker | Podman |
  |--------|--------|
  | `docker compose up -d` | `podman compose up -d` |
  | `docker compose down` | `podman compose down` |
  | `docker compose logs` | `podman compose logs` |

- [ ] **System**:
  | Docker | Podman |
  |--------|--------|
  | `docker system prune` | `podman system prune` |
  | `docker info` | `podman info` |
  | `systemctl restart docker` | `systemctl restart podman.socket` |
  | `/var/run/docker.sock` | `/run/podman/podman.sock` |

### `reference/podman-config.mdx` — Podman Configuration for Icefall
- [ ] `config.toml` settings: `runtime`, `container_socket` with example values
- [ ] Agent config: same fields for worker servers
- [ ] Environment variables: `DOCKER_HOST` / socket path override
- [ ] Podman socket activation: `systemctl enable --now podman.socket`
- [ ] Podman storage config: `/etc/containers/storage.conf` relevant settings
- [ ] Podman registries config: `/etc/containers/registries.conf` for private registries
- [ ] Resource limits on Podman: cgroup v2 requirement, how to verify

### `reference/podman-differences.mdx` — Behavioral Differences
- [ ] Networking: Docker bridge (automatic DNS) vs Podman netavark (needs named network)
- [ ] Image building: BuildKit vs Buildah — subtle layer caching differences
- [ ] Stats API: `system_cpu_usage` returning 0 in rootless mode
- [ ] Exec/attach: known compat issues with stream multiplexing (issue #19901)
- [ ] Volume permissions: UID mapping in rootless, `:Z` / `:U` suffixes for SELinux
- [ ] Compose: `podman compose` (Docker Compose v2 shim) vs `podman-compose` (Python, community)
- [ ] Restart policies: Docker restart policies vs Podman Quadlet/systemd
- [ ] Log drivers: Docker json-file vs Podman journald (default)
- [ ] Socket path: different default locations, how to verify
- [ ] Rootful vs rootless: what Icefall supports today, what's planned

### `guides/podman-setup.mdx` — Setup Guide
- [ ] Prerequisites: Linux, cgroup v2, kernel 5.11+
- [ ] Install Podman on Ubuntu 22.04/24.04 (step by step)
- [ ] Install Podman on Debian 12 (step by step)
- [ ] Install Podman on CentOS Stream 9 / RHEL (step by step)
- [ ] Enable the Podman socket: `systemctl enable --now podman.socket`
- [ ] Verify: `curl --unix-socket /run/podman/podman.sock http://localhost/v4.0.0/libpod/info`
- [ ] Install Icefall with Podman: `curl -fsSL ... | bash` (auto-detects Podman)
- [ ] Verify Icefall detects Podman: check logs for "Detected runtime: podman"
- [ ] Deploy your first app: same as Docker quickstart, but confirm Podman is the runtime

### `guides/docker-to-podman-migration.mdx` — Migration Guide
- [ ] Pre-migration checklist: backup Icefall, list running containers, note volumes
- [ ] Step 1: Stop Icefall (`systemctl stop icefall`)
- [ ] Step 2: Export container data (volumes, configs)
- [ ] Step 3: Install Podman alongside Docker
- [ ] Step 4: Update `config.toml`: `runtime = "podman"`, `container_socket = "/run/podman/podman.sock"`
- [ ] Step 5: Start Icefall, verify detection
- [ ] Step 6: Redeploy apps (containers need to be recreated under Podman)
- [ ] Step 7: Verify everything works (health checks, logs, terminal, deploys)
- [ ] Step 8: Optionally uninstall Docker
- [ ] Rollback plan: switch `config.toml` back to Docker if issues arise

## Standards

- [ ] Every Docker command has a Podman equivalent — no gaps
- [ ] Differences called out with warning callout boxes, not buried in paragraphs
- [ ] All commands tested on Ubuntu 24.04 with Podman 5.x
- [ ] Searchable: include common error messages verbatim so users find the page via search
- [ ] Tabbed code blocks: Docker / Podman side by side wherever both commands appear

## Dependencies

- IF-206 (Podman runtime support — implementation must be done first)
- IF-047 (Documentation site)
