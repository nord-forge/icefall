# IF-151: Cloudflare Tunnel integration

**Phase:** 22 — Expansion (v1.2)
**Priority:** Medium
**Estimate:** M

## Description

Add a guided Cloudflare Tunnel setup flow so users behind NAT, CGNAT, or restrictive firewalls can expose their Icefall apps to the internet without opening ports. The user provides their Cloudflare tunnel token, Icefall configures the tunnel routing per app, and `cloudflared` runs as a managed container alongside the apps. This replaces the need for port forwarding and public IP addresses.

## Acceptance Criteria

### Tunnel Setup

- [ ] New section in global Settings: "Cloudflare Tunnel"
- [ ] Setup flow:
  1. User enters their Cloudflare tunnel token (from `cloudflared tunnel create` or the Cloudflare Zero Trust dashboard)
  2. Icefall pulls and starts the `cloudflare/cloudflared:latest` container with `tunnel run --token <token>`
  3. Health check: verify the tunnel container is running and connected
  4. Status display: connected / disconnected / error with tunnel ID
- [ ] Tunnel token stored encrypted in the settings table (AES-256-GCM)
- [ ] "Disconnect Tunnel" button: stops and removes the cloudflared container, clears the token

### Per-App Tunnel Routing

- [ ] When Cloudflare Tunnel is configured, each app's domain settings show a "Route via Tunnel" toggle
- [ ] When enabled: Icefall generates a `cloudflared` ingress rule mapping the domain to the app's internal container address (e.g., `http://app-container:port`)
- [ ] The ingress configuration is written to a config file mounted into the cloudflared container
- [ ] Ingress rules are regenerated whenever:
  - An app enables/disables tunnel routing
  - An app's domain changes
  - An app is deleted
  - A new app enables tunnel routing
- [ ] Catch-all rule at the bottom: `http_status:404` (Cloudflare requirement)

### Cloudflared Container Management

- [ ] Container name: `icefall-cloudflared`
- [ ] Image: `cloudflare/cloudflared:latest` (pinned to a specific version in production)
- [ ] Restart policy: `unless-stopped`
- [ ] Config file mounted at `/etc/cloudflared/config.yml`
- [ ] Connected to the same container network as Icefall app containers (so it can reach them by container name)
- [ ] Logs captured and available via the existing log viewer (treated as a system service, not an app)
- [ ] Auto-restart on config change: after regenerating ingress rules, restart the cloudflared container
- [ ] Resource limits: 128MB memory, 0.25 CPU shares (cloudflared is lightweight)

### Config File Generation

- [ ] Generated `config.yml` format:
  ```yaml
  tunnel: <tunnel-id>
  credentials-file: /etc/cloudflared/credentials.json
  ingress:
    - hostname: app1.example.com
      service: http://icefall-app1-blue:3000
    - hostname: app2.example.com
      service: http://icefall-app2-blue:8080
    - service: http_status:404
  ```
- [ ] Hostname resolution uses the app's container name + port
- [ ] If the app has path-based routing (IF-069), the ingress rule includes the path matcher
- [ ] Wildcard domains (`*.example.com`) are supported if the user's Cloudflare plan allows it

### Dashboard UI

- [ ] Settings page: Cloudflare Tunnel section
  - Status indicator: connected (green) / disconnected (red) / not configured (gray)
  - Tunnel token input (password field with reveal toggle)
  - "Connect" / "Disconnect" buttons
  - Tunnel ID and connection info when connected
  - Link to Cloudflare Zero Trust dashboard for tunnel management
- [ ] App domain settings: "Route via Cloudflare Tunnel" toggle per domain
  - Only visible when tunnel is configured
  - Shows the external URL that Cloudflare will route to this app
  - Warning when enabling: "DNS for this domain must point to Cloudflare (orange-clouded)"
- [ ] System status: cloudflared container health on the dashboard home server stats

### Multi-Server

- [ ] Each server (control plane + workers) can have its own Cloudflare tunnel
- [ ] Tunnel token is per-server, stored in the server's config
- [ ] On worker servers: the agent manages the cloudflared container
- [ ] Ingress rules on each server only include apps deployed to that server

### API Endpoints

- [ ] `GET /settings/tunnel` — get tunnel status and config (token masked)
- [ ] `POST /settings/tunnel` — configure tunnel token and start cloudflared
- [ ] `DELETE /settings/tunnel` — disconnect and remove tunnel
- [ ] `GET /settings/tunnel/status` — real-time tunnel status (connected/disconnected, uptime, connections)
- [ ] `PUT /apps/{id}/domains/{domain_id}/tunnel` — enable/disable tunnel routing for a domain
- [ ] For multi-server: `POST /servers/{id}/tunnel`, `DELETE /servers/{id}/tunnel`, `GET /servers/{id}/tunnel/status`

## Technical Notes

- `cloudflared` supports both token-based auth (simpler, recommended for Icefall) and credential-file auth — start with token-based only
- The tunnel token encodes the tunnel ID and credentials — no need to store them separately
- Ingress rules are order-dependent — more specific rules must come before less specific ones, catch-all last
- `cloudflared` can reload config without restart via `SIGHUP`, but a container restart is simpler and more reliable
- Network: create a shared container network (`icefall-tunnel`) that both app containers and the cloudflared container join (works with both Docker bridge and Podman netavark)
- If the user is already using Caddy for SSL: Cloudflare Tunnel handles TLS termination at Cloudflare's edge, so the tunnel sends plain HTTP to the app container. Caddy is bypassed for tunneled domains.

## Out of Scope

- Cloudflare Access policies (Zero Trust auth rules) — users manage those in the Cloudflare dashboard
- Cloudflare Workers or Pages integration
- Other tunnel providers (ngrok, Tailscale Funnel) — could be added later with a similar pattern
- Automatic DNS record creation via Cloudflare API (requires API token with DNS permissions — too much scope)
- Cloudflare Argo Smart Routing configuration

## Dependencies

- IF-004 (Container runtime client — for managing the cloudflared container)
- IF-069 (Path-based routing — for path-aware ingress rules)
- IF-023 (Domain management — tunnel toggle lives on domain settings)
