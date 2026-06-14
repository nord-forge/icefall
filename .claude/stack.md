# Icefall Stack

## Backend (Daemon + CLI)
- **Language:** Rust (edition 2021, stable toolchain)
- **Async runtime:** Tokio
- **HTTP framework:** Axum 0.8
- **CLI:** Clap 4 (derive)
- **Database:** SQLite via sqlx 0.8 (WAL mode, Postgres-compatible SQL)
- **Docker:** Bollard 0.18 (Docker Engine API via Unix socket)
- **Reverse proxy:** Caddy (admin API via reqwest)
- **Encryption:** AES-256-GCM (aes-gcm 0.10)
- **Password hashing:** Argon2 (argon2 0.5)
- **Error handling:** thiserror 2
- **Tracing:** tracing + tracing-subscriber
- **OpenAPI:** utoipa 5 + utoipa-swagger-ui 9
- **Config format:** TOML

## Frontend (Dashboard)
- **Framework:** Astro + Preact
- **Styling:** CSS Modules
- **State:** Preact Signals + Nanostores
- **Icons:** Lucide (`lucide-preact`)

## Infrastructure
- **Process management:** systemd
- **Reverse proxy:** Caddy (auto HTTPS via Let's Encrypt)
- **Container runtime:** Docker Engine
