# Beta testing on a Hetzner server — `v0.1.0-beta.1`

A quick runbook to spin up Icefall on a fresh Hetzner box and smoke-test it.
The release is a **pre-release**, so the install command must pin the version
(`latest` skips pre-releases).

Release: https://github.com/nord-forge/icefall/releases/tag/v0.1.0-beta.1

---

## 1. Create the server (Hetzner Cloud)

- **Image:** Ubuntu 24.04 (Debian 12 also fine).
- **Type:** any works — the installer auto-detects arch and enables low-memory
  mode on small boxes:
  - x86: `CX22` (2 vCPU / 4 GB) is comfortable; `CX11`/`CPX11` works for a
    minimal test.
  - ARM: `CAX11` (2 vCPU / 4 GB) — the release ships an `aarch64` binary too.
- **SSH key:** add yours so you can log in.

### Firewall / ports

Open inbound:

| Port | Why |
|------|-----|
| 22   | SSH |
| 80   | HTTP + Let's Encrypt HTTP-01 challenge |
| 443  | HTTPS (dashboard + apps once a domain is set) |
| 3000 | Dashboard over IP, **only if testing without a domain** (or your `--port`) |

You can use Hetzner Cloud Firewall or `ufw` on the box. If you set a domain,
you don't need 3000 open (Caddy serves the dashboard on 443).

---

## 2. Install Icefall

SSH in as root (or a sudo user), then pick one of the flows below.

### A) Fully interactive (Astro-style prompts)

```bash
curl -fsSL https://github.com/nord-forge/icefall/raw/main/install.sh | sudo ICEFALL_VERSION=v0.1.0-beta.1 bash
```

It will prompt for: container runtime (if ambiguous), **dashboard port**
(default 3000), and a **base domain** (blank to skip). If you enter a domain it
prints the exact DNS records and offers to wait until they resolve.

### B) Non-interactive with a domain (recommended for a real test)

```bash
curl -fsSL https://github.com/nord-forge/icefall/raw/main/install.sh \
  | sudo ICEFALL_VERSION=v0.1.0-beta.1 bash -s -- \
      --yes --domain=apps.example.com --port=3000
```

Replace `apps.example.com` with a domain you control.

### C) No domain (reach by IP:port)

```bash
curl -fsSL https://github.com/nord-forge/icefall/raw/main/install.sh \
  | sudo ICEFALL_VERSION=v0.1.0-beta.1 bash -s -- --yes
```

> Flags after `bash -s --` are passed to the script. Env vars
> (`ICEFALL_VERSION`, `ICEFALL_RUNTIME`, `ICEFALL_PORT`, `ICEFALL_BASE_DOMAIN`)
> go before `bash`.

---

## 3. DNS (only if you set a domain)

Point these records at the server's public IP (the installer prints them):

```
A      apps.example.com      ->  <SERVER_IP>
A      *.apps.example.com    ->  <SERVER_IP>      # wildcard, per-app subdomains
```

(Use `AAAA` if you're testing over IPv6.) Once DNS resolves and 80/443 are
open, Caddy auto-issues HTTPS. First cert can take ~30–60s.

---

## 4. Open the dashboard

- With a domain: `https://apps.example.com`
- Without:        `http://<SERVER_IP>:3000`

Create the admin account on first load.

---

## 5. Smoke-test checklist

On the server:

```bash
systemctl status icefall            # daemon is active (running)
journalctl -u icefall -f            # live logs; look for the dashboard-route line
icefall --version                   # should print 0.1.0-beta.1
cat /etc/icefall/config.toml        # listen_port + base_domain match your choices
caddy version                       # Caddy installed
docker info  ||  podman info        # runtime up
curl -fsS http://localhost:3000/api/v1/health   # daemon answering locally
```

In the dashboard, try a minimal end-to-end deploy:

- [ ] Create an app from a small public repo (e.g. a static site or a tiny
      Node/Go app).
- [ ] Watch the build + deploy stream to completion.
- [ ] App reachable at its subdomain (`myapp.apps.example.com`) over HTTPS.
- [ ] Env vars: set one, redeploy, confirm it's applied.
- [ ] Logs tab streams container output.

---

## 6. Useful operations

```bash
# Restart / stop the daemon
sudo systemctl restart icefall
sudo systemctl stop icefall

# Re-run the installer to upgrade or change runtime later (idempotent)
curl -fsSL https://github.com/nord-forge/icefall/raw/main/install.sh \
  | sudo ICEFALL_VERSION=v0.1.0-beta.1 bash

# Install log (if something fails during install)
cat /var/log/icefall-install.log
```

---

## 7. Tear down

Destroy the Hetzner server when done. If you want to wipe Icefall in place
without destroying the box:

```bash
sudo systemctl disable --now icefall
sudo rm -rf /etc/icefall /var/lib/icefall /usr/local/bin/icefall \
            /etc/systemd/system/icefall.service
sudo systemctl daemon-reload
```

---

## Verifying a download by checksum (optional)

Each tarball has a `.sha256` sidecar, and the release is signed
(`...-manifest.json` + `.sig`). To verify a manual download:

```bash
sha256sum -c icefall-v0.1.0-beta.1-x86_64-linux.tar.gz.sha256
```

The installer already does this checksum check automatically.
