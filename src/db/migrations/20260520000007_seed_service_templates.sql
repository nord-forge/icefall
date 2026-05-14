-- Seed the 50 bundled service templates (IF-148)
-- Targets: solo developers and small teams (1-10 people)

INSERT INTO service_templates (id, name, description, version, icon_url, categories, website, required_inputs, default_env, min_resources, compose_content, readme, created_at, updated_at)
VALUES

-- 1. Gitea
(
  'gitea',
  'Gitea',
  'Lightweight self-hosted Git service with issue tracking and CI',
  '1.22.4',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23609926%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3EG%3C%2Ftext%3E%3C%2Fsvg%3E',
  'DevTools',
  'https://about.gitea.com',
  '[{"key":"HTTP_PORT","label":"HTTP port","type":"number","placeholder":"3000"},{"key":"SSH_PORT","label":"SSH port","type":"number","placeholder":"2222"}]',
  '{"GITEA__server__ROOT_URL":"http://localhost:3000"}',
  '{"memory_mb":256,"disk_mb":1024}',
  'services:
  gitea:
    image: gitea/gitea:1.22.4
    ports:
      - "${HTTP_PORT:-3000}:3000"
      - "${SSH_PORT:-2222}:22"
    volumes:
      - gitea-data:/data
    environment:
      GITEA__server__ROOT_URL: "${GITEA__server__ROOT_URL}"
      GITEA__database__DB_TYPE: sqlite3
    restart: unless-stopped

volumes:
  gitea-data:',
  'Open `http://<server>:${HTTP_PORT}` to complete the installation wizard. The first registered user becomes admin.',
  datetime('now'),
  datetime('now')
),

-- 2. Uptime Kuma
(
  'uptime-kuma',
  'Uptime Kuma',
  'Self-hosted monitoring tool with status pages and notifications',
  '1.23.15',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%235CDD8B%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%231a1a2e%22%3EUK%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Monitoring',
  'https://uptime.kuma.pet',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"3001"}]',
  NULL,
  '{"memory_mb":256,"disk_mb":256}',
  'services:
  uptime-kuma:
    image: louislam/uptime-kuma:1.23.15
    ports:
      - "${PORT:-3001}:3001"
    volumes:
      - uptime-kuma-data:/app/data
    restart: unless-stopped

volumes:
  uptime-kuma-data:',
  'Open `http://<server>:${PORT}` and create your admin account. Add your first monitor to start tracking uptime.',
  datetime('now'),
  datetime('now')
),

-- 3. Portainer
(
  'portainer',
  'Portainer',
  'Container management UI for Docker and Kubernetes',
  '2.21.4',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%2313BEF9%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3EP%3C%2Ftext%3E%3C%2Fsvg%3E',
  'DevTools',
  'https://www.portainer.io',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"9443"}]',
  NULL,
  '{"memory_mb":256,"disk_mb":512}',
  'services:
  portainer:
    image: portainer/portainer-ce:2.21.4
    ports:
      - "${PORT:-9443}:9443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - portainer-data:/data
    restart: unless-stopped

volumes:
  portainer-data:',
  'Open `https://<server>:${PORT}` (HTTPS) and create your admin account within 5 minutes of first start.',
  datetime('now'),
  datetime('now')
),

-- 4. Woodpecker CI
(
  'woodpecker-ci',
  'Woodpecker CI',
  'Lightweight CI/CD engine with YAML pipelines and native Docker support',
  '2.7.3',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%231A73E8%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3EW%3C%2Ftext%3E%3C%2Fsvg%3E',
  'DevTools',
  'https://woodpecker-ci.org',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"8000"},{"key":"WOODPECKER_HOST","label":"Public URL","type":"url","placeholder":"https://ci.example.com"},{"key":"WOODPECKER_AGENT_SECRET","label":"Agent secret","type":"password","placeholder":""}]',
  '{"WOODPECKER_OPEN":"true"}',
  '{"memory_mb":512,"disk_mb":1024}',
  'services:
  woodpecker-server:
    image: woodpeckerci/woodpecker-server:v2.7.3
    ports:
      - "${PORT:-8000}:8000"
    volumes:
      - woodpecker-data:/var/lib/woodpecker
    environment:
      WOODPECKER_HOST: "${WOODPECKER_HOST}"
      WOODPECKER_OPEN: "${WOODPECKER_OPEN}"
      WOODPECKER_AGENT_SECRET: "${WOODPECKER_AGENT_SECRET}"
    restart: unless-stopped

  woodpecker-agent:
    image: woodpeckerci/woodpecker-agent:v2.7.3
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
    environment:
      WOODPECKER_SERVER: woodpecker-server:9000
      WOODPECKER_AGENT_SECRET: "${WOODPECKER_AGENT_SECRET}"
    depends_on:
      - woodpecker-server
    restart: unless-stopped

volumes:
  woodpecker-data:',
  'Open `${WOODPECKER_HOST}` and connect your Gitea, GitHub, or GitLab account to start building pipelines.',
  datetime('now'),
  datetime('now')
),

-- 5. Traefik
(
  'traefik',
  'Traefik',
  'Cloud-native reverse proxy with automatic SSL and Docker integration',
  '3.2.3',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%2324A1C1%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3ET%3C%2Ftext%3E%3C%2Fsvg%3E',
  'DevTools',
  'https://traefik.io',
  '[{"key":"HTTP_PORT","label":"HTTP port","type":"number","placeholder":"80"},{"key":"HTTPS_PORT","label":"HTTPS port","type":"number","placeholder":"443"},{"key":"DASHBOARD_PORT","label":"Dashboard port","type":"number","placeholder":"8080"},{"key":"ACME_EMAIL","label":"Let''s Encrypt email","type":"email","placeholder":"admin@example.com"}]',
  NULL,
  '{"memory_mb":128,"disk_mb":256}',
  'services:
  traefik:
    image: traefik:v3.2.3
    command:
      - "--api.dashboard=true"
      - "--api.insecure=true"
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
      - "--certificatesresolvers.letsencrypt.acme.email=${ACME_EMAIL}"
      - "--certificatesresolvers.letsencrypt.acme.storage=/letsencrypt/acme.json"
      - "--certificatesresolvers.letsencrypt.acme.httpchallenge.entrypoint=web"
    ports:
      - "${HTTP_PORT:-80}:80"
      - "${HTTPS_PORT:-443}:443"
      - "${DASHBOARD_PORT:-8080}:8080"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - traefik-certs:/letsencrypt
    restart: unless-stopped

volumes:
  traefik-certs:',
  'Dashboard at `http://<server>:${DASHBOARD_PORT}`. Add labels to other containers to route traffic through Traefik.',
  datetime('now'),
  datetime('now')
),

-- 6. Vaultwarden
(
  'vaultwarden',
  'Vaultwarden',
  'Lightweight Bitwarden-compatible password manager server',
  '1.32.5',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23175DDC%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3EV%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Security',
  'https://github.com/dani-garcia/vaultwarden',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"8080"},{"key":"DOMAIN","label":"Domain URL","type":"url","placeholder":"https://vault.example.com"},{"key":"ADMIN_TOKEN","label":"Admin token","type":"password","placeholder":""}]',
  '{"SIGNUPS_ALLOWED":"false"}',
  '{"memory_mb":128,"disk_mb":256}',
  'services:
  vaultwarden:
    image: vaultwarden/server:1.32.5
    ports:
      - "${PORT:-8080}:80"
    volumes:
      - vaultwarden-data:/data
    environment:
      DOMAIN: "${DOMAIN}"
      ADMIN_TOKEN: "${ADMIN_TOKEN}"
      SIGNUPS_ALLOWED: "${SIGNUPS_ALLOWED}"
    restart: unless-stopped

volumes:
  vaultwarden-data:',
  'Access at `${DOMAIN}`. Admin panel at `${DOMAIN}/admin` using your admin token. Use any Bitwarden client to connect.',
  datetime('now'),
  datetime('now')
),

-- 7. Directus
(
  'directus',
  'Directus',
  'Instant REST and GraphQL API on top of any SQL database',
  '11.3.0',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%236644FF%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3ED%3C%2Ftext%3E%3C%2Fsvg%3E',
  'CMS',
  'https://directus.io',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"8055"},{"key":"ADMIN_EMAIL","label":"Admin email","type":"email","placeholder":"admin@example.com"},{"key":"ADMIN_PASSWORD","label":"Admin password","type":"password","placeholder":""}]',
  '{"PUBLIC_URL":"http://localhost:8055","DB_CLIENT":"sqlite3","DB_FILENAME":"/directus/database/data.db"}',
  '{"memory_mb":512,"disk_mb":1024}',
  'services:
  directus:
    image: directus/directus:11.3.0
    ports:
      - "${PORT:-8055}:8055"
    volumes:
      - directus-uploads:/directus/uploads
      - directus-db:/directus/database
    environment:
      PUBLIC_URL: "${PUBLIC_URL}"
      DB_CLIENT: "${DB_CLIENT}"
      DB_FILENAME: "${DB_FILENAME}"
      ADMIN_EMAIL: "${ADMIN_EMAIL}"
      ADMIN_PASSWORD: "${ADMIN_PASSWORD}"
      SECRET: "${SECRET:-replace-with-random-value}"
    restart: unless-stopped

volumes:
  directus-uploads:
  directus-db:',
  'Sign in at `http://<server>:${PORT}` with the admin credentials you provided.',
  datetime('now'),
  datetime('now')
),

-- 8. MinIO
(
  'minio',
  'MinIO',
  'High-performance S3-compatible object storage',
  '2024.11.7',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23C72C48%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3EM%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Storage',
  'https://min.io',
  '[{"key":"API_PORT","label":"API port","type":"number","placeholder":"9000"},{"key":"CONSOLE_PORT","label":"Console port","type":"number","placeholder":"9001"},{"key":"MINIO_ROOT_USER","label":"Root user","type":"text","placeholder":"minioadmin"},{"key":"MINIO_ROOT_PASSWORD","label":"Root password","type":"password","placeholder":""}]',
  NULL,
  '{"memory_mb":512,"disk_mb":2048}',
  'services:
  minio:
    image: minio/minio:RELEASE.2024-11-07T00-52-20Z
    command: server /data --console-address ":9001"
    ports:
      - "${API_PORT:-9000}:9000"
      - "${CONSOLE_PORT:-9001}:9001"
    volumes:
      - minio-data:/data
    environment:
      MINIO_ROOT_USER: "${MINIO_ROOT_USER}"
      MINIO_ROOT_PASSWORD: "${MINIO_ROOT_PASSWORD}"
    restart: unless-stopped

volumes:
  minio-data:',
  'Console at `http://<server>:${CONSOLE_PORT}`, API at port `${API_PORT}`. Create buckets and access keys from the console.',
  datetime('now'),
  datetime('now')
),

-- 9. Grafana
(
  'grafana',
  'Grafana',
  'Observability dashboards for metrics, logs, and traces',
  '11.3.0',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23F46800%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3EG%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Monitoring',
  'https://grafana.com',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"3000"},{"key":"ADMIN_PASSWORD","label":"Admin password","type":"password","placeholder":""}]',
  '{"GF_SECURITY_ADMIN_USER":"admin"}',
  '{"memory_mb":256,"disk_mb":512}',
  'services:
  grafana:
    image: grafana/grafana-oss:11.3.0
    ports:
      - "${PORT:-3000}:3000"
    volumes:
      - grafana-data:/var/lib/grafana
    environment:
      GF_SECURITY_ADMIN_USER: "${GF_SECURITY_ADMIN_USER}"
      GF_SECURITY_ADMIN_PASSWORD: "${ADMIN_PASSWORD}"
    restart: unless-stopped

volumes:
  grafana-data:',
  'Sign in at `http://<server>:${PORT}` with username `admin` and the password you set.',
  datetime('now'),
  datetime('now')
),

-- 10. Prometheus
(
  'prometheus',
  'Prometheus',
  'Metrics collection and alerting toolkit for infrastructure monitoring',
  '2.55.1',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23E6522C%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3EP%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Monitoring',
  'https://prometheus.io',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"9090"}]',
  NULL,
  '{"memory_mb":512,"disk_mb":2048}',
  'services:
  prometheus:
    image: prom/prometheus:v2.55.1
    ports:
      - "${PORT:-9090}:9090"
    volumes:
      - prometheus-data:/prometheus
    command:
      - "--config.file=/etc/prometheus/prometheus.yml"
      - "--storage.tsdb.path=/prometheus"
      - "--storage.tsdb.retention.time=30d"
    restart: unless-stopped

volumes:
  prometheus-data:',
  'Open `http://<server>:${PORT}` for the expression browser. Mount a custom `prometheus.yml` to configure scrape targets.',
  datetime('now'),
  datetime('now')
),

-- 11. n8n
(
  'n8n',
  'n8n',
  'Workflow automation tool with 400+ integrations',
  '1.68.0',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23EA4B71%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3En8%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Productivity',
  'https://n8n.io',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"5678"},{"key":"WEBHOOK_URL","label":"Webhook base URL","type":"url","placeholder":"https://n8n.example.com"}]',
  '{"GENERIC_TIMEZONE":"UTC"}',
  '{"memory_mb":512,"disk_mb":512}',
  'services:
  n8n:
    image: n8nio/n8n:1.68.0
    ports:
      - "${PORT:-5678}:5678"
    volumes:
      - n8n-data:/home/node/.n8n
    environment:
      GENERIC_TIMEZONE: "${GENERIC_TIMEZONE}"
      WEBHOOK_URL: "${WEBHOOK_URL}"
      N8N_SECURE_COOKIE: "false"
    restart: unless-stopped

volumes:
  n8n-data:',
  'Open `http://<server>:${PORT}` to create your owner account and build your first workflow.',
  datetime('now'),
  datetime('now')
),

-- 12. Plausible Analytics
(
  'plausible',
  'Plausible Analytics',
  'Privacy-friendly Google Analytics alternative',
  '2.1.4',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%235850EC%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3EP%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Analytics',
  'https://plausible.io',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"8000"},{"key":"BASE_URL","label":"Your domain","type":"url","placeholder":"https://analytics.example.com"},{"key":"ADMIN_EMAIL","label":"Admin email","type":"email","placeholder":"admin@example.com"}]',
  '{"DISABLE_REGISTRATION":"invite_only"}',
  '{"memory_mb":512,"disk_mb":1024}',
  'services:
  plausible:
    image: ghcr.io/plausible/community-edition:v2.1.4
    ports:
      - "${PORT:-8000}:8000"
    volumes:
      - plausible-db:/var/lib/clickhouse
      - plausible-events:/var/lib/plausible
    environment:
      BASE_URL: "${BASE_URL}"
      SECRET_KEY_BASE: "${SECRET_KEY_BASE:-replace-with-64-char-random}"
      DISABLE_REGISTRATION: "${DISABLE_REGISTRATION}"
    restart: unless-stopped

volumes:
  plausible-db:
  plausible-events:',
  'Open `${BASE_URL}` and register with `${ADMIN_EMAIL}`. Add the tracking snippet to your sites.',
  datetime('now'),
  datetime('now')
),

-- 13. GlitchTip
(
  'glitchtip',
  'GlitchTip',
  'Lightweight Sentry-compatible error tracking and uptime monitoring',
  '4.1',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%238852E0%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EGT%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Monitoring',
  'https://glitchtip.com',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"8000"},{"key":"SECRET_KEY","label":"Secret key","type":"password","placeholder":""},{"key":"DEFAULT_FROM_EMAIL","label":"From email","type":"email","placeholder":"noreply@example.com"}]',
  '{"GLITCHTIP_DOMAIN":"http://localhost:8000","ENABLE_OPEN_USER_REGISTRATION":"true"}',
  '{"memory_mb":512,"disk_mb":1024}',
  'services:
  glitchtip:
    image: glitchtip/glitchtip:v4.1
    ports:
      - "${PORT:-8000}:8000"
    environment:
      SECRET_KEY: "${SECRET_KEY}"
      GLITCHTIP_DOMAIN: "${GLITCHTIP_DOMAIN}"
      DEFAULT_FROM_EMAIL: "${DEFAULT_FROM_EMAIL}"
      DATABASE_URL: postgresql://glitchtip:glitchtip@glitchtip-db:5432/glitchtip
      ENABLE_OPEN_USER_REGISTRATION: "${ENABLE_OPEN_USER_REGISTRATION}"
    depends_on:
      - glitchtip-db
      - glitchtip-redis
    restart: unless-stopped

  glitchtip-worker:
    image: glitchtip/glitchtip:v4.1
    command: bin/run-celery-with-beat.sh
    environment:
      SECRET_KEY: "${SECRET_KEY}"
      DATABASE_URL: postgresql://glitchtip:glitchtip@glitchtip-db:5432/glitchtip
      REDIS_URL: redis://glitchtip-redis:6379/0
    depends_on:
      - glitchtip-db
      - glitchtip-redis
    restart: unless-stopped

  glitchtip-db:
    image: postgres:16-alpine
    volumes:
      - glitchtip-db-data:/var/lib/postgresql/data
    environment:
      POSTGRES_DB: glitchtip
      POSTGRES_USER: glitchtip
      POSTGRES_PASSWORD: glitchtip
    restart: unless-stopped

  glitchtip-redis:
    image: valkey/valkey:8-alpine
    restart: unless-stopped

volumes:
  glitchtip-db-data:',
  'Open `http://<server>:${PORT}` and register. Create a project and use any Sentry SDK with the provided DSN.',
  datetime('now'),
  datetime('now')
),

-- 14. Infisical
(
  'infisical',
  'Infisical',
  'Open-source secrets management platform with SDK support',
  '0.91.0',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23A1B659%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%231a1a2e%22%3EIF%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Security',
  'https://infisical.com',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"8080"},{"key":"ENCRYPTION_KEY","label":"Encryption key (16 bytes hex)","type":"password","placeholder":""},{"key":"AUTH_SECRET","label":"Auth secret","type":"password","placeholder":""}]',
  '{"SITE_URL":"http://localhost:8080"}',
  '{"memory_mb":512,"disk_mb":1024}',
  'services:
  infisical:
    image: infisical/infisical:v0.91.0
    ports:
      - "${PORT:-8080}:8080"
    environment:
      ENCRYPTION_KEY: "${ENCRYPTION_KEY}"
      AUTH_SECRET: "${AUTH_SECRET}"
      SITE_URL: "${SITE_URL}"
      DB_CONNECTION_URI: postgresql://infisical:infisical@infisical-db:5432/infisical
      REDIS_URL: redis://infisical-redis:6379
    depends_on:
      - infisical-db
      - infisical-redis
    restart: unless-stopped

  infisical-db:
    image: postgres:16-alpine
    volumes:
      - infisical-db-data:/var/lib/postgresql/data
    environment:
      POSTGRES_DB: infisical
      POSTGRES_USER: infisical
      POSTGRES_PASSWORD: infisical
    restart: unless-stopped

  infisical-redis:
    image: valkey/valkey:8-alpine
    restart: unless-stopped

volumes:
  infisical-db-data:',
  'Open `http://<server>:${PORT}` and create your account. Use the Infisical CLI or SDK to inject secrets into your apps.',
  datetime('now'),
  datetime('now')
),

-- 15. Docker Registry
(
  'docker-registry',
  'Docker Registry',
  'Official self-hosted container image registry',
  '2.8.3',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%232496ED%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EDR%3C%2Ftext%3E%3C%2Fsvg%3E',
  'DevTools',
  'https://docs.docker.com/registry/',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"5000"}]',
  '{"REGISTRY_STORAGE_DELETE_ENABLED":"true"}',
  '{"memory_mb":128,"disk_mb":4096}',
  'services:
  registry:
    image: registry:2.8.3
    ports:
      - "${PORT:-5000}:5000"
    volumes:
      - registry-data:/var/lib/registry
    environment:
      REGISTRY_STORAGE_DELETE_ENABLED: "${REGISTRY_STORAGE_DELETE_ENABLED}"
    restart: unless-stopped

volumes:
  registry-data:',
  'Push images with `docker tag myapp <server>:${PORT}/myapp && docker push <server>:${PORT}/myapp`. Add TLS for production use.',
  datetime('now'),
  datetime('now')
),

-- 16. Meilisearch
(
  'meilisearch',
  'Meilisearch',
  'Lightning-fast, typo-tolerant search engine with a RESTful API',
  '1.11.3',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23FF5CAA%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EMs%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Database',
  'https://www.meilisearch.com',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"7700"},{"key":"MEILI_MASTER_KEY","label":"Master key","type":"password","placeholder":""}]',
  '{"MEILI_ENV":"production"}',
  '{"memory_mb":512,"disk_mb":1024}',
  'services:
  meilisearch:
    image: getmeili/meilisearch:v1.11.3
    ports:
      - "${PORT:-7700}:7700"
    volumes:
      - meili-data:/meili_data
    environment:
      MEILI_ENV: "${MEILI_ENV}"
      MEILI_MASTER_KEY: "${MEILI_MASTER_KEY}"
    restart: unless-stopped

volumes:
  meili-data:',
  'API available at `http://<server>:${PORT}`. Use the master key to create search and admin API keys.',
  datetime('now'),
  datetime('now')
),

-- 17. Ghost
(
  'ghost',
  'Ghost',
  'Professional publishing platform for blogs and newsletters',
  '5.97.0',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%2315171A%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EGh%3C%2Ftext%3E%3C%2Fsvg%3E',
  'CMS',
  'https://ghost.org',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"2368"},{"key":"URL","label":"Site URL","type":"url","placeholder":"https://blog.example.com"}]',
  '{"NODE_ENV":"production"}',
  '{"memory_mb":512,"disk_mb":1024}',
  'services:
  ghost:
    image: ghost:5.97.0-alpine
    ports:
      - "${PORT:-2368}:2368"
    volumes:
      - ghost-content:/var/lib/ghost/content
    environment:
      NODE_ENV: "${NODE_ENV}"
      url: "${URL}"
    restart: unless-stopped

volumes:
  ghost-content:',
  'Visit `${URL}/ghost` to create your admin account and start publishing.',
  datetime('now'),
  datetime('now')
),

-- 18. Supabase
(
  'supabase',
  'Supabase',
  'Open-source Firebase alternative with Postgres, Auth, and Realtime',
  '1.24.07',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%233ECF8E%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%231a1a2e%22%3ES%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Database',
  'https://supabase.com',
  '[{"key":"STUDIO_PORT","label":"Studio port","type":"number","placeholder":"3000"},{"key":"API_PORT","label":"API port","type":"number","placeholder":"8000"},{"key":"ANON_KEY","label":"Anon key","type":"text","placeholder":""},{"key":"SERVICE_ROLE_KEY","label":"Service role key","type":"password","placeholder":""},{"key":"JWT_SECRET","label":"JWT secret","type":"password","placeholder":""}]',
  '{"POSTGRES_PASSWORD":"your-super-secret-password"}',
  '{"memory_mb":2048,"disk_mb":2048}',
  'services:
  supabase-studio:
    image: supabase/studio:20241029-f4ce478
    ports:
      - "${STUDIO_PORT:-3000}:3000"
    environment:
      STUDIO_PG_META_URL: http://supabase-meta:8080
      SUPABASE_URL: http://supabase-kong:8000
      SUPABASE_ANON_KEY: "${ANON_KEY}"
      SUPABASE_SERVICE_KEY: "${SERVICE_ROLE_KEY}"
    depends_on:
      - supabase-db
    restart: unless-stopped

  supabase-kong:
    image: kong:2.8.1
    ports:
      - "${API_PORT:-8000}:8000"
    environment:
      KONG_DECLARATIVE_CONFIG: /var/lib/kong/kong.yml
      KONG_DNS_ORDER: LAST,A,CNAME
    restart: unless-stopped

  supabase-db:
    image: supabase/postgres:15.6.1.143
    volumes:
      - supabase-db-data:/var/lib/postgresql/data
    environment:
      POSTGRES_PASSWORD: "${POSTGRES_PASSWORD}"
      JWT_SECRET: "${JWT_SECRET}"
    restart: unless-stopped

  supabase-meta:
    image: supabase/postgres-meta:0.84.2
    environment:
      PG_META_PORT: 8080
      PG_META_DB_HOST: supabase-db
      PG_META_DB_PASSWORD: "${POSTGRES_PASSWORD}"
    depends_on:
      - supabase-db
    restart: unless-stopped

volumes:
  supabase-db-data:',
  'Studio at `http://<server>:${STUDIO_PORT}`, API at port `${API_PORT}`. Generate your JWT keys at https://supabase.com/docs/guides/self-hosting#api-keys.',
  datetime('now'),
  datetime('now')
),

-- 19. Metabase
(
  'metabase',
  'Metabase',
  'Business intelligence and analytics with a visual query builder',
  '0.51.6',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23509EE3%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EMb%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Analytics',
  'https://www.metabase.com',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"3000"}]',
  '{"MB_DB_TYPE":"h2"}',
  '{"memory_mb":1024,"disk_mb":1024}',
  'services:
  metabase:
    image: metabase/metabase:v0.51.6
    ports:
      - "${PORT:-3000}:3000"
    volumes:
      - metabase-data:/metabase-data
    environment:
      MB_DB_TYPE: "${MB_DB_TYPE}"
      MB_DB_FILE: /metabase-data/metabase.db
    restart: unless-stopped

volumes:
  metabase-data:',
  'Open `http://<server>:${PORT}` and complete the setup wizard to connect your first data source.',
  datetime('now'),
  datetime('now')
),

-- 20. Wiki.js
(
  'wikijs',
  'Wiki.js',
  'Powerful wiki engine with markdown support and built-in access control',
  '2.5.306',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%231976D2%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3EW%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Productivity',
  'https://js.wiki',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"3000"}]',
  NULL,
  '{"memory_mb":512,"disk_mb":1024}',
  'services:
  wikijs:
    image: ghcr.io/requarks/wiki:2.5.306
    ports:
      - "${PORT:-3000}:3000"
    volumes:
      - wikijs-data:/wiki/data/content
    environment:
      DB_TYPE: sqlite
      DB_FILEPATH: /wiki/data/content/db.sqlite
    restart: unless-stopped

volumes:
  wikijs-data:',
  'Open `http://<server>:${PORT}` to complete the setup wizard. Choose your admin email and password during first launch.',
  datetime('now'),
  datetime('now')
),

-- 21. Loki
(
  'loki',
  'Grafana Loki',
  'Log aggregation system designed to work with Grafana',
  '3.3.2',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23F2994A%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3EL%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Monitoring',
  'https://grafana.com/oss/loki/',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"3100"}]',
  NULL,
  '{"memory_mb":512,"disk_mb":2048}',
  'services:
  loki:
    image: grafana/loki:3.3.2
    ports:
      - "${PORT:-3100}:3100"
    volumes:
      - loki-data:/loki
    command: -config.file=/etc/loki/local-config.yaml
    restart: unless-stopped

volumes:
  loki-data:',
  'API at `http://<server>:${PORT}`. Add Loki as a data source in Grafana to query logs with LogQL.',
  datetime('now'),
  datetime('now')
),

-- 22. Mailpit
(
  'mailpit',
  'Mailpit',
  'Email testing tool with SMTP server and web UI for development',
  '1.21.7',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%230EA5E9%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EMp%3C%2Ftext%3E%3C%2Fsvg%3E',
  'DevTools',
  'https://mailpit.axllent.org',
  '[{"key":"UI_PORT","label":"Web UI port","type":"number","placeholder":"8025"},{"key":"SMTP_PORT","label":"SMTP port","type":"number","placeholder":"1025"}]',
  NULL,
  '{"memory_mb":64,"disk_mb":128}',
  'services:
  mailpit:
    image: axllent/mailpit:v1.21.7
    ports:
      - "${UI_PORT:-8025}:8025"
      - "${SMTP_PORT:-1025}:1025"
    volumes:
      - mailpit-data:/data
    environment:
      MP_DATABASE: /data/mailpit.db
    restart: unless-stopped

volumes:
  mailpit-data:',
  'Web UI at `http://<server>:${UI_PORT}`. Point your app''s SMTP to `<server>:${SMTP_PORT}` to catch all outgoing email.',
  datetime('now'),
  datetime('now')
),

-- 23. Unleash
(
  'unleash',
  'Unleash',
  'Open-source feature flag management for safe deployments',
  '6.3.0',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%231A4049%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%234DC89A%22%3EU%3C%2Ftext%3E%3C%2Fsvg%3E',
  'DevTools',
  'https://www.getunleash.io',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"4242"}]',
  NULL,
  '{"memory_mb":512,"disk_mb":512}',
  'services:
  unleash:
    image: unleashorg/unleash-server:6.3.0
    ports:
      - "${PORT:-4242}:4242"
    environment:
      DATABASE_URL: postgresql://unleash:unleash@unleash-db:5432/unleash
      DATABASE_SSL: "false"
    depends_on:
      - unleash-db
    restart: unless-stopped

  unleash-db:
    image: postgres:16-alpine
    volumes:
      - unleash-db-data:/var/lib/postgresql/data
    environment:
      POSTGRES_DB: unleash
      POSTGRES_USER: unleash
      POSTGRES_PASSWORD: unleash
    restart: unless-stopped

volumes:
  unleash-db-data:',
  'Open `http://<server>:${PORT}` and sign in with `admin` / `unleash4all`. Change the password immediately.',
  datetime('now'),
  datetime('now')
),

-- 24. Ntfy
(
  'ntfy',
  'Ntfy',
  'Simple push notification service via HTTP PUT/POST',
  '2.11.0',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%2357A64A%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3EN%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Communication',
  'https://ntfy.sh',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"8080"}]',
  NULL,
  '{"memory_mb":64,"disk_mb":128}',
  'services:
  ntfy:
    image: binwiederhier/ntfy:v2.11.0
    ports:
      - "${PORT:-8080}:80"
    volumes:
      - ntfy-cache:/var/cache/ntfy
    command: serve
    restart: unless-stopped

volumes:
  ntfy-cache:',
  'Send a test notification: `curl -d "Hello from Icefall" http://<server>:${PORT}/test`. Install the ntfy app on your phone to subscribe to topics.',
  datetime('now'),
  datetime('now')
),

-- 25. Authentik
(
  'authentik',
  'Authentik',
  'Identity provider with SSO, SAML, OAuth2, and LDAP',
  '2024.10.5',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23FD4B2D%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3EA%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Security',
  'https://goauthentik.io',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"9000"},{"key":"AUTHENTIK_SECRET_KEY","label":"Secret key","type":"password","placeholder":""},{"key":"AUTHENTIK_ERROR_REPORTING__ENABLED","label":"Error reporting","type":"text","placeholder":"false"}]',
  NULL,
  '{"memory_mb":1024,"disk_mb":1024}',
  'services:
  authentik-server:
    image: ghcr.io/goauthentik/server:2024.10.5
    command: server
    ports:
      - "${PORT:-9000}:9000"
    environment:
      AUTHENTIK_SECRET_KEY: "${AUTHENTIK_SECRET_KEY}"
      AUTHENTIK_ERROR_REPORTING__ENABLED: "${AUTHENTIK_ERROR_REPORTING__ENABLED}"
      AUTHENTIK_REDIS__HOST: authentik-redis
      AUTHENTIK_POSTGRESQL__HOST: authentik-db
      AUTHENTIK_POSTGRESQL__USER: authentik
      AUTHENTIK_POSTGRESQL__PASSWORD: authentik
      AUTHENTIK_POSTGRESQL__NAME: authentik
    depends_on:
      - authentik-db
      - authentik-redis
    restart: unless-stopped

  authentik-worker:
    image: ghcr.io/goauthentik/server:2024.10.5
    command: worker
    environment:
      AUTHENTIK_SECRET_KEY: "${AUTHENTIK_SECRET_KEY}"
      AUTHENTIK_REDIS__HOST: authentik-redis
      AUTHENTIK_POSTGRESQL__HOST: authentik-db
      AUTHENTIK_POSTGRESQL__USER: authentik
      AUTHENTIK_POSTGRESQL__PASSWORD: authentik
      AUTHENTIK_POSTGRESQL__NAME: authentik
    depends_on:
      - authentik-db
      - authentik-redis
    restart: unless-stopped

  authentik-db:
    image: postgres:16-alpine
    volumes:
      - authentik-db-data:/var/lib/postgresql/data
    environment:
      POSTGRES_DB: authentik
      POSTGRES_USER: authentik
      POSTGRES_PASSWORD: authentik
    restart: unless-stopped

  authentik-redis:
    image: valkey/valkey:8-alpine
    restart: unless-stopped

volumes:
  authentik-db-data:',
  'Open `http://<server>:${PORT}/if/flow/initial-setup/` to create the admin account. Then configure providers for OAuth2/SAML SSO.',
  datetime('now'),
  datetime('now')
),

-- 26. pgAdmin
(
  'pgadmin',
  'pgAdmin',
  'PostgreSQL administration and management GUI',
  '8.13',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23336791%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3Epg%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Database',
  'https://www.pgadmin.org',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"5050"},{"key":"PGADMIN_DEFAULT_EMAIL","label":"Admin email","type":"email","placeholder":"admin@example.com"},{"key":"PGADMIN_DEFAULT_PASSWORD","label":"Admin password","type":"password","placeholder":""}]',
  NULL,
  '{"memory_mb":256,"disk_mb":256}',
  'services:
  pgadmin:
    image: dpage/pgadmin4:8.13
    ports:
      - "${PORT:-5050}:80"
    volumes:
      - pgadmin-data:/var/lib/pgadmin
    environment:
      PGADMIN_DEFAULT_EMAIL: "${PGADMIN_DEFAULT_EMAIL}"
      PGADMIN_DEFAULT_PASSWORD: "${PGADMIN_DEFAULT_PASSWORD}"
    restart: unless-stopped

volumes:
  pgadmin-data:',
  'Sign in at `http://<server>:${PORT}` and add your PostgreSQL server connections.',
  datetime('now'),
  datetime('now')
),

-- 27. Umami
(
  'umami',
  'Umami',
  'Simple, fast, privacy-focused web analytics',
  '2.14.0',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23000000%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3EU%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Analytics',
  'https://umami.is',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"3000"}]',
  NULL,
  '{"memory_mb":256,"disk_mb":512}',
  'services:
  umami:
    image: ghcr.io/umami-software/umami:postgresql-v2.14.0
    ports:
      - "${PORT:-3000}:3000"
    environment:
      DATABASE_URL: postgresql://umami:umami@umami-db:5432/umami
    depends_on:
      - umami-db
    restart: unless-stopped

  umami-db:
    image: postgres:16-alpine
    volumes:
      - umami-db-data:/var/lib/postgresql/data
    environment:
      POSTGRES_DB: umami
      POSTGRES_USER: umami
      POSTGRES_PASSWORD: umami
    restart: unless-stopped

volumes:
  umami-db-data:',
  'Sign in at `http://<server>:${PORT}` with username `admin` and password `umami`. Change the default password immediately.',
  datetime('now'),
  datetime('now')
),

-- 28. Ollama
(
  'ollama',
  'Ollama',
  'Run large language models locally with a simple API',
  '0.4.7',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23000000%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3EO%3C%2Ftext%3E%3C%2Fsvg%3E',
  'AI/ML',
  'https://ollama.com',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"11434"}]',
  NULL,
  '{"memory_mb":4096,"disk_mb":8192}',
  'services:
  ollama:
    image: ollama/ollama:0.4.7
    ports:
      - "${PORT:-11434}:11434"
    volumes:
      - ollama-data:/root/.ollama
    restart: unless-stopped

volumes:
  ollama-data:',
  'Pull a model: `curl http://<server>:${PORT}/api/pull -d ''{"name":"llama3.2"}''`. API docs at https://github.com/ollama/ollama/blob/main/docs/api.md.',
  datetime('now'),
  datetime('now')
),

-- 29. Hoppscotch
(
  'hoppscotch',
  'Hoppscotch',
  'Open-source API development platform and Postman alternative',
  '2024.12.0',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%231CE783%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%231a1a2e%22%3EH%3C%2Ftext%3E%3C%2Fsvg%3E',
  'DevTools',
  'https://hoppscotch.io',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"3000"}]',
  NULL,
  '{"memory_mb":512,"disk_mb":512}',
  'services:
  hoppscotch:
    image: hoppscotch/hoppscotch:2024.12.0
    ports:
      - "${PORT:-3000}:3000"
    environment:
      DATABASE_URL: postgresql://hoppscotch:hoppscotch@hoppscotch-db:5432/hoppscotch
      TOKEN_SALT_COMPLEXITY: 10
      MAGIC_LINK_TOKEN_VALIDITY: 3
      REFRESH_TOKEN_VALIDITY: 604800000
      ACCESS_TOKEN_VALIDITY: 86400000
      SESSION_SECRET: "${SESSION_SECRET:-replace-with-random-value}"
    depends_on:
      - hoppscotch-db
    restart: unless-stopped

  hoppscotch-db:
    image: postgres:16-alpine
    volumes:
      - hoppscotch-db-data:/var/lib/postgresql/data
    environment:
      POSTGRES_DB: hoppscotch
      POSTGRES_USER: hoppscotch
      POSTGRES_PASSWORD: hoppscotch
    restart: unless-stopped

volumes:
  hoppscotch-db-data:',
  'Open `http://<server>:${PORT}` to start testing APIs. Create collections and share them with your team.',
  datetime('now'),
  datetime('now')
),

-- 30. Listmonk
(
  'listmonk',
  'Listmonk',
  'High-performance mailing list and newsletter manager',
  '4.0.0',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23FF5722%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3ELm%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Communication',
  'https://listmonk.app',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"9000"},{"key":"LISTMONK_admin__username","label":"Admin username","type":"text","placeholder":"admin"},{"key":"LISTMONK_admin__password","label":"Admin password","type":"password","placeholder":""}]',
  NULL,
  '{"memory_mb":128,"disk_mb":256}',
  'services:
  listmonk:
    image: listmonk/listmonk:v4.0.0
    ports:
      - "${PORT:-9000}:9000"
    environment:
      LISTMONK_app__address: 0.0.0.0:9000
      LISTMONK_db__host: listmonk-db
      LISTMONK_db__port: 5432
      LISTMONK_db__user: listmonk
      LISTMONK_db__password: listmonk
      LISTMONK_db__database: listmonk
      LISTMONK_admin__username: "${LISTMONK_admin__username}"
      LISTMONK_admin__password: "${LISTMONK_admin__password}"
    depends_on:
      - listmonk-db
    restart: unless-stopped

  listmonk-db:
    image: postgres:16-alpine
    volumes:
      - listmonk-db-data:/var/lib/postgresql/data
    environment:
      POSTGRES_DB: listmonk
      POSTGRES_USER: listmonk
      POSTGRES_PASSWORD: listmonk
    restart: unless-stopped

volumes:
  listmonk-db-data:',
  'Open `http://<server>:${PORT}` and sign in. Run `--install` on first start to initialize the database. Configure SMTP under Settings to start sending.',
  datetime('now'),
  datetime('now')
),

-- 31. PrivateBin
(
  'privatebin',
  'PrivateBin',
  'Zero-knowledge encrypted pastebin for sharing secrets and logs',
  '1.7.5',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23FFD600%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%231a1a2e%22%3EPB%3C%2Ftext%3E%3C%2Fsvg%3E',
  'DevTools',
  'https://privatebin.info',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"8080"}]',
  NULL,
  '{"memory_mb":64,"disk_mb":128}',
  'services:
  privatebin:
    image: privatebin/nginx-fpm-alpine:1.7.5
    ports:
      - "${PORT:-8080}:8080"
    volumes:
      - privatebin-data:/srv/data
    restart: unless-stopped

volumes:
  privatebin-data:',
  'Open `http://<server>:${PORT}` and paste. Content is encrypted in the browser — the server never sees plaintext. Share the link with automatic expiry.',
  datetime('now'),
  datetime('now')
),

-- 32. CrowdSec
(
  'crowdsec',
  'CrowdSec',
  'Collaborative security engine with community threat intelligence',
  '1.6.4',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%232B3990%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3ECS%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Security',
  'https://www.crowdsec.net',
  '[{"key":"API_PORT","label":"API port","type":"number","placeholder":"8080"},{"key":"PROMETHEUS_PORT","label":"Prometheus metrics port","type":"number","placeholder":"6060"}]',
  '{"COLLECTIONS":"crowdsecurity/linux crowdsecurity/nginx"}',
  '{"memory_mb":128,"disk_mb":256}',
  'services:
  crowdsec:
    image: crowdsecurity/crowdsec:v1.6.4
    ports:
      - "${API_PORT:-8080}:8080"
      - "${PROMETHEUS_PORT:-6060}:6060"
    volumes:
      - crowdsec-data:/var/lib/crowdsec/data
      - crowdsec-config:/etc/crowdsec
      - /var/log:/var/log:ro
    environment:
      COLLECTIONS: "${COLLECTIONS}"
    restart: unless-stopped

volumes:
  crowdsec-data:
  crowdsec-config:',
  'Enroll with CrowdSec Console: `docker exec crowdsec cscli console enroll <key>`. Install a bouncer (firewall/nginx) to block detected threats.',
  datetime('now'),
  datetime('now')
),

-- 33. Open WebUI
(
  'open-webui',
  'Open WebUI',
  'ChatGPT-like web interface for Ollama and OpenAI-compatible APIs',
  '0.4.8',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23000000%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EOW%3C%2Ftext%3E%3C%2Fsvg%3E',
  'AI/ML',
  'https://openwebui.com',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"3000"},{"key":"OLLAMA_BASE_URL","label":"Ollama API URL","type":"url","placeholder":"http://ollama:11434"}]',
  NULL,
  '{"memory_mb":512,"disk_mb":512}',
  'services:
  open-webui:
    image: ghcr.io/open-webui/open-webui:v0.4.8
    ports:
      - "${PORT:-3000}:8080"
    volumes:
      - open-webui-data:/app/backend/data
    environment:
      OLLAMA_BASE_URL: "${OLLAMA_BASE_URL}"
    restart: unless-stopped

volumes:
  open-webui-data:',
  'Open `http://<server>:${PORT}` and create your account. Connect to a running Ollama instance to start chatting with local LLMs.',
  datetime('now'),
  datetime('now')
),

-- 34. Gatus
(
  'gatus',
  'Gatus',
  'Developer-oriented health dashboard and status page',
  '5.13.1',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%2341B883%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EGa%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Monitoring',
  'https://gatus.io',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"8080"}]',
  NULL,
  '{"memory_mb":64,"disk_mb":128}',
  'services:
  gatus:
    image: twinproduction/gatus:v5.13.1
    ports:
      - "${PORT:-8080}:8080"
    volumes:
      - gatus-data:/data
      - ./gatus-config.yaml:/config/config.yaml:ro
    restart: unless-stopped

volumes:
  gatus-data:',
  'Mount a `config.yaml` with your endpoints. Dashboard at `http://<server>:${PORT}`. See https://gatus.io/docs for config reference.',
  datetime('now'),
  datetime('now')
),

-- 35. Outline
(
  'outline',
  'Outline',
  'Beautiful team wiki and knowledge base with a Slack-like editor',
  '0.81.1',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%230366D6%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EOl%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Productivity',
  'https://www.getoutline.com',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"3000"},{"key":"SECRET_KEY","label":"Secret key (min 32 chars)","type":"password","placeholder":""},{"key":"UTILS_SECRET","label":"Utils secret (min 32 chars)","type":"password","placeholder":""},{"key":"URL","label":"Public URL","type":"url","placeholder":"https://docs.example.com"}]',
  '{"FORCE_HTTPS":"false"}',
  '{"memory_mb":512,"disk_mb":1024}',
  'services:
  outline:
    image: outlinewiki/outline:0.81.1
    ports:
      - "${PORT:-3000}:3000"
    environment:
      SECRET_KEY: "${SECRET_KEY}"
      UTILS_SECRET: "${UTILS_SECRET}"
      URL: "${URL}"
      DATABASE_URL: postgresql://outline:outline@outline-db:5432/outline
      REDIS_URL: redis://outline-redis:6379
      FORCE_HTTPS: "${FORCE_HTTPS}"
      FILE_STORAGE: local
      FILE_STORAGE_LOCAL_ROOT_DIR: /var/lib/outline/data
    volumes:
      - outline-data:/var/lib/outline/data
    depends_on:
      - outline-db
      - outline-redis
    restart: unless-stopped

  outline-db:
    image: postgres:16-alpine
    volumes:
      - outline-db-data:/var/lib/postgresql/data
    environment:
      POSTGRES_DB: outline
      POSTGRES_USER: outline
      POSTGRES_PASSWORD: outline
    restart: unless-stopped

  outline-redis:
    image: valkey/valkey:8-alpine
    restart: unless-stopped

volumes:
  outline-data:
  outline-db-data:',
  'Open `${URL}` and configure an authentication provider (OIDC, SAML, or Slack). Outline requires SSO — pair with Authentik if you need a provider.',
  datetime('now'),
  datetime('now')
),

-- 36. Shlink
(
  'shlink',
  'Shlink',
  'Self-hosted URL shortener with REST API and visit analytics',
  '4.3.0',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%234696E5%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3ESh%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Productivity',
  'https://shlink.io',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"8080"},{"key":"DEFAULT_DOMAIN","label":"Short URL domain","type":"text","placeholder":"s.example.com"},{"key":"GEOLITE_LICENSE_KEY","label":"GeoLite2 key (optional)","type":"text","placeholder":""}]',
  '{"IS_HTTPS_ENABLED":"false"}',
  '{"memory_mb":128,"disk_mb":256}',
  'services:
  shlink:
    image: shlinkio/shlink:4.3.0
    ports:
      - "${PORT:-8080}:8080"
    environment:
      DEFAULT_DOMAIN: "${DEFAULT_DOMAIN}"
      IS_HTTPS_ENABLED: "${IS_HTTPS_ENABLED}"
      GEOLITE_LICENSE_KEY: "${GEOLITE_LICENSE_KEY}"
    restart: unless-stopped

volumes: {}',
  'Generate an API key: `docker exec shlink shlink api-key:generate`. Use the REST API or install the shlink-web-client for a management UI.',
  datetime('now'),
  datetime('now')
),

-- 37. Verdaccio
(
  'verdaccio',
  'Verdaccio',
  'Lightweight private npm registry for JavaScript and TypeScript packages',
  '6.0.5',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%234B5E40%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3EV%3C%2Ftext%3E%3C%2Fsvg%3E',
  'DevTools',
  'https://verdaccio.org',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"4873"}]',
  NULL,
  '{"memory_mb":128,"disk_mb":1024}',
  'services:
  verdaccio:
    image: verdaccio/verdaccio:6.0.5
    ports:
      - "${PORT:-4873}:4873"
    volumes:
      - verdaccio-storage:/verdaccio/storage
      - verdaccio-plugins:/verdaccio/plugins
    restart: unless-stopped

volumes:
  verdaccio-storage:
  verdaccio-plugins:',
  'Set registry: `npm set registry http://<server>:${PORT}`. Create a user: `npm adduser --registry http://<server>:${PORT}`. Publish with `npm publish`.',
  datetime('now'),
  datetime('now')
),

-- 38. Cal.com
(
  'calcom',
  'Cal.com',
  'Open-source scheduling infrastructure and Calendly alternative',
  '4.7.2',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23292929%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3ECa%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Productivity',
  'https://cal.com',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"3000"},{"key":"NEXTAUTH_SECRET","label":"NextAuth secret","type":"password","placeholder":""},{"key":"CALENDSO_ENCRYPTION_KEY","label":"Encryption key","type":"password","placeholder":""}]',
  '{"NEXT_PUBLIC_WEBAPP_URL":"http://localhost:3000","NEXT_PUBLIC_LICENSE_CONSENT":"agree"}',
  '{"memory_mb":1024,"disk_mb":1024}',
  'services:
  calcom:
    image: calcom/cal.com:v4.7.2
    ports:
      - "${PORT:-3000}:3000"
    environment:
      DATABASE_URL: postgresql://calcom:calcom@calcom-db:5432/calcom
      NEXTAUTH_SECRET: "${NEXTAUTH_SECRET}"
      CALENDSO_ENCRYPTION_KEY: "${CALENDSO_ENCRYPTION_KEY}"
      NEXT_PUBLIC_WEBAPP_URL: "${NEXT_PUBLIC_WEBAPP_URL}"
      NEXT_PUBLIC_LICENSE_CONSENT: "${NEXT_PUBLIC_LICENSE_CONSENT}"
    depends_on:
      - calcom-db
    restart: unless-stopped

  calcom-db:
    image: postgres:16-alpine
    volumes:
      - calcom-db-data:/var/lib/postgresql/data
    environment:
      POSTGRES_DB: calcom
      POSTGRES_USER: calcom
      POSTGRES_PASSWORD: calcom
    restart: unless-stopped

volumes:
  calcom-db-data:',
  'Open `http://<server>:${PORT}` to create your account. Connect your Google Calendar or Outlook to start accepting bookings.',
  datetime('now'),
  datetime('now')
),

-- 39. Adminer
(
  'adminer',
  'Adminer',
  'Lightweight universal database admin supporting MySQL, Postgres, SQLite, and MongoDB',
  '4.8.1',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23007FBF%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EAd%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Database',
  'https://www.adminer.org',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"8080"}]',
  '{"ADMINER_DEFAULT_SERVER":""}',
  '{"memory_mb":64,"disk_mb":64}',
  'services:
  adminer:
    image: adminer:4.8.1
    ports:
      - "${PORT:-8080}:8080"
    environment:
      ADMINER_DEFAULT_SERVER: "${ADMINER_DEFAULT_SERVER}"
    restart: unless-stopped',
  'Open `http://<server>:${PORT}` and connect to any database. Supports MySQL, PostgreSQL, SQLite, MS SQL, Oracle, and MongoDB.',
  datetime('now'),
  datetime('now')
),

-- 40. Vikunja
(
  'vikunja',
  'Vikunja',
  'Open-source task and project management with kanban boards',
  '0.24.4',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%237F23DC%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EVk%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Productivity',
  'https://vikunja.io',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"3456"}]',
  '{"VIKUNJA_SERVICE_TIMEZONE":"UTC"}',
  '{"memory_mb":256,"disk_mb":512}',
  'services:
  vikunja:
    image: vikunja/vikunja:0.24.4
    ports:
      - "${PORT:-3456}:3456"
    volumes:
      - vikunja-files:/app/vikunja/files
      - vikunja-db:/app/vikunja/db
    environment:
      VIKUNJA_SERVICE_TIMEZONE: "${VIKUNJA_SERVICE_TIMEZONE}"
    restart: unless-stopped

volumes:
  vikunja-files:
  vikunja-db:',
  'Open `http://<server>:${PORT}` and register your first account. The first user becomes the admin.',
  datetime('now'),
  datetime('now')
),

-- 41. Nextcloud
(
  'nextcloud',
  'Nextcloud',
  'Self-hosted file sync, collaboration, and productivity suite',
  '30.0.2',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%230082C9%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3ENc%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Storage',
  'https://nextcloud.com',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"8080"},{"key":"NEXTCLOUD_ADMIN_USER","label":"Admin username","type":"text","placeholder":"admin"},{"key":"NEXTCLOUD_ADMIN_PASSWORD","label":"Admin password","type":"password","placeholder":""}]',
  '{"SQLITE_DATABASE":"nextcloud"}',
  '{"memory_mb":512,"disk_mb":4096}',
  'services:
  nextcloud:
    image: nextcloud:30.0.2-apache
    ports:
      - "${PORT:-8080}:80"
    volumes:
      - nextcloud-data:/var/www/html
    environment:
      SQLITE_DATABASE: "${SQLITE_DATABASE}"
      NEXTCLOUD_ADMIN_USER: "${NEXTCLOUD_ADMIN_USER}"
      NEXTCLOUD_ADMIN_PASSWORD: "${NEXTCLOUD_ADMIN_PASSWORD}"
    restart: unless-stopped

volumes:
  nextcloud-data:',
  'Sign in at `http://<server>:${PORT}` with the admin credentials you set. Install recommended apps from the app store.',
  datetime('now'),
  datetime('now')
),

-- 42. Appsmith
(
  'appsmith',
  'Appsmith',
  'Low-code platform for building internal tools and dashboards',
  '1.42',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23F36C2D%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EAs%3C%2Ftext%3E%3C%2Fsvg%3E',
  'DevTools',
  'https://www.appsmith.com',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"8080"}]',
  NULL,
  '{"memory_mb":2048,"disk_mb":2048}',
  'services:
  appsmith:
    image: appsmith/appsmith-ee:v1.42
    ports:
      - "${PORT:-8080}:80"
    volumes:
      - appsmith-stacks:/appsmith-stacks
    restart: unless-stopped

volumes:
  appsmith-stacks:',
  'Open `http://<server>:${PORT}` and complete the setup wizard to create your admin account.',
  datetime('now'),
  datetime('now')
),

-- 43. PicoShare
(
  'picoshare',
  'PicoShare',
  'Minimal file sharing with no sign-up required for recipients',
  '1.4.3',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23795548%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EPs%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Storage',
  'https://github.com/mtlynch/picoshare',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"4001"},{"key":"PS_SHARED_SECRET","label":"Passphrase","type":"password","placeholder":""}]',
  NULL,
  '{"memory_mb":64,"disk_mb":1024}',
  'services:
  picoshare:
    image: mtlynch/picoshare:1.4.3
    ports:
      - "${PORT:-4001}:4001"
    volumes:
      - picoshare-data:/data
    environment:
      PORT: 4001
      PS_SHARED_SECRET: "${PS_SHARED_SECRET}"
    restart: unless-stopped

volumes:
  picoshare-data:',
  'Open `http://<server>:${PORT}` and sign in with your passphrase. Upload files and share links with automatic expiry.',
  datetime('now'),
  datetime('now')
),

-- 44. Duplicati
(
  'duplicati',
  'Duplicati',
  'Encrypted backup to cloud storage with scheduling and versioning',
  '2.1.0.2',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23333840%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%2367C8FF%22%3EDu%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Storage',
  'https://www.duplicati.com',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"8200"},{"key":"BACKUP_SOURCE","label":"Directory to back up","type":"text","placeholder":"/source"}]',
  NULL,
  '{"memory_mb":256,"disk_mb":512}',
  'services:
  duplicati:
    image: lscr.io/linuxserver/duplicati:v2.1.0.2
    ports:
      - "${PORT:-8200}:8200"
    volumes:
      - duplicati-config:/config
      - "${BACKUP_SOURCE}:/source:ro"
    environment:
      PUID: 1000
      PGID: 1000
    restart: unless-stopped

volumes:
  duplicati-config:',
  'Open `http://<server>:${PORT}` and configure a backup job. Supports S3, B2, Google Drive, OneDrive, and more as destinations.',
  datetime('now'),
  datetime('now')
),

-- 45. Rallly
(
  'rallly',
  'Rallly',
  'Meeting scheduling tool and self-hosted Doodle alternative',
  '3.11.2',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%236366F1%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3ER%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Productivity',
  'https://rallly.co',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"3000"},{"key":"SECRET_PASSWORD","label":"Secret password","type":"password","placeholder":""}]',
  '{"NEXT_PUBLIC_BASE_URL":"http://localhost:3000","SUPPORT_EMAIL":"support@example.com"}',
  '{"memory_mb":256,"disk_mb":256}',
  'services:
  rallly:
    image: lukevella/rallly:v3.11.2
    ports:
      - "${PORT:-3000}:3000"
    environment:
      DATABASE_URL: postgresql://rallly:rallly@rallly-db:5432/rallly
      SECRET_PASSWORD: "${SECRET_PASSWORD}"
      NEXT_PUBLIC_BASE_URL: "${NEXT_PUBLIC_BASE_URL}"
      SUPPORT_EMAIL: "${SUPPORT_EMAIL}"
    depends_on:
      - rallly-db
    restart: unless-stopped

  rallly-db:
    image: postgres:16-alpine
    volumes:
      - rallly-db-data:/var/lib/postgresql/data
    environment:
      POSTGRES_DB: rallly
      POSTGRES_USER: rallly
      POSTGRES_PASSWORD: rallly
    restart: unless-stopped

volumes:
  rallly-db-data:',
  'Open `http://<server>:${PORT}` and create a poll. Share the link with participants to find the best time.',
  datetime('now'),
  datetime('now')
),

-- 46. Actual Budget
(
  'actual-budget',
  'Actual Budget',
  'Privacy-focused local-first personal finance app',
  '24.11.0',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%235B21B6%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EAB%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Productivity',
  'https://actualbudget.org',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"5006"}]',
  '{"ACTUAL_UPLOAD_LIMIT":"20"}',
  '{"memory_mb":256,"disk_mb":512}',
  'services:
  actual-budget:
    image: actualbudget/actual-server:24.11.0
    ports:
      - "${PORT:-5006}:5006"
    volumes:
      - actual-data:/data
    environment:
      ACTUAL_UPLOAD_FILE_SYNC_SIZE_LIMIT_MB: "${ACTUAL_UPLOAD_LIMIT}"
    restart: unless-stopped

volumes:
  actual-data:',
  'Access the UI at `http://<server>:${PORT}` and create your first budget.',
  datetime('now'),
  datetime('now')
),

-- 47. dbgate
(
  'dbgate',
  'DbGate',
  'Modern web-based database GUI for MySQL, Postgres, MongoDB, SQLite, and Redis',
  '5.5.5',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23339AF0%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EDb%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Database',
  'https://dbgate.org',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"3000"}]',
  NULL,
  '{"memory_mb":128,"disk_mb":128}',
  'services:
  dbgate:
    image: dbgate/dbgate:5.5.5
    ports:
      - "${PORT:-3000}:3000"
    volumes:
      - dbgate-data:/root/.dbgate
    restart: unless-stopped

volumes:
  dbgate-data:',
  'Open `http://<server>:${PORT}` and add your database connections. Supports query editor, visual table editor, data export, and ER diagrams.',
  datetime('now'),
  datetime('now')
),

-- 48. Forgejo
(
  'forgejo',
  'Forgejo',
  'Community-governed Git forge with issue tracking and packages',
  '9.0.3',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%23FB923C%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2214%22%20fill%3D%22%23fff%22%3EF%3C%2Ftext%3E%3C%2Fsvg%3E',
  'DevTools',
  'https://forgejo.org',
  '[{"key":"HTTP_PORT","label":"HTTP port","type":"number","placeholder":"3000"},{"key":"SSH_PORT","label":"SSH port","type":"number","placeholder":"2222"}]',
  '{"FORGEJO__server__ROOT_URL":"http://localhost:3000"}',
  '{"memory_mb":256,"disk_mb":1024}',
  'services:
  forgejo:
    image: codeberg.org/forgejo/forgejo:9.0.3
    ports:
      - "${HTTP_PORT:-3000}:3000"
      - "${SSH_PORT:-2222}:22"
    volumes:
      - forgejo-data:/data
    environment:
      FORGEJO__server__ROOT_URL: "${FORGEJO__server__ROOT_URL}"
      FORGEJO__database__DB_TYPE: sqlite3
    restart: unless-stopped

volumes:
  forgejo-data:',
  'Open `http://<server>:${HTTP_PORT}` to complete the installation. The first registered user becomes admin. Drop-in Gitea replacement.',
  datetime('now'),
  datetime('now')
),

-- 49. Immich
(
  'immich',
  'Immich',
  'Self-hosted photo and video backup with mobile apps and ML features',
  '1.121.0',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%234250AF%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EIm%3C%2Ftext%3E%3C%2Fsvg%3E',
  'Media',
  'https://immich.app',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"2283"}]',
  '{"UPLOAD_LOCATION":"/usr/src/app/upload","DB_PASSWORD":"immich"}',
  '{"memory_mb":2048,"disk_mb":8192}',
  'services:
  immich-server:
    image: ghcr.io/immich-app/immich-server:v1.121.0
    ports:
      - "${PORT:-2283}:2283"
    volumes:
      - immich-upload:/usr/src/app/upload
    environment:
      DB_HOSTNAME: immich-db
      DB_USERNAME: immich
      DB_PASSWORD: "${DB_PASSWORD}"
      DB_DATABASE_NAME: immich
      REDIS_HOSTNAME: immich-redis
    depends_on:
      - immich-db
      - immich-redis
    restart: unless-stopped

  immich-machine-learning:
    image: ghcr.io/immich-app/immich-machine-learning:v1.121.0
    volumes:
      - immich-ml-cache:/cache
    restart: unless-stopped

  immich-db:
    image: tensorchord/pgvecto-rs:pg16-v0.3.0
    volumes:
      - immich-db-data:/var/lib/postgresql/data
    environment:
      POSTGRES_DB: immich
      POSTGRES_USER: immich
      POSTGRES_PASSWORD: "${DB_PASSWORD}"
    restart: unless-stopped

  immich-redis:
    image: valkey/valkey:8-alpine
    restart: unless-stopped

volumes:
  immich-upload:
  immich-ml-cache:
  immich-db-data:',
  'Open `http://<server>:${PORT}` and create your admin account. Install the Immich mobile app (iOS/Android) to start backing up photos.',
  datetime('now'),
  datetime('now')
),

-- 50. Gitness
(
  'gitness',
  'Gitness',
  'Git hosting and CI pipelines in one platform by Harness',
  '3.0.0',
  'data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2032%2032%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20rx%3D%226%22%20fill%3D%22%2300ADE6%22%2F%3E%3Ctext%20x%3D%2216%22%20y%3D%2221%22%20text-anchor%3D%22middle%22%20font-family%3D%22system-ui%2Csans-serif%22%20font-weight%3D%22700%22%20font-size%3D%2211%22%20fill%3D%22%23fff%22%3EGn%3C%2Ftext%3E%3C%2Fsvg%3E',
  'DevTools',
  'https://gitness.com',
  '[{"key":"PORT","label":"Host port","type":"number","placeholder":"3000"}]',
  NULL,
  '{"memory_mb":512,"disk_mb":1024}',
  'services:
  gitness:
    image: harness/gitness:3.0.0
    ports:
      - "${PORT:-3000}:3000"
    volumes:
      - gitness-data:/data
    restart: unless-stopped

volumes:
  gitness-data:',
  'Open `http://<server>:${PORT}` and register. Built-in CI pipelines run alongside your repositories — no separate CI server needed.',
  datetime('now'),
  datetime('now')
);
