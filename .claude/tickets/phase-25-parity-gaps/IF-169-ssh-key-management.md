# IF-169: SSH key management

**Phase:** 25 — Parity Gaps
**Priority:** Medium
**Estimate:** M

## Description

Allow users to generate or import SSH keys for authenticating with git providers and servers. Keys are stored encrypted in the database and used for private repo cloning and server management.

## Acceptance Criteria

### Database
- [ ] New `ssh_keys` table: `id`, `user_id` (FK), `name`, `public_key`, `private_key` (encrypted), `fingerprint`, `key_type` (ed25519/rsa), `created_at`, `last_used_at`
- [ ] Private keys encrypted at rest using existing AES-256-GCM encryption

### Key Operations
- [ ] Generate new Ed25519 keypair (preferred) or RSA 4096 keypair
- [ ] Import existing private key (paste or file upload)
- [ ] View public key (for adding to GitHub/GitLab)
- [ ] Delete key (with confirmation)
- [ ] Copy public key to clipboard

### Git Integration
- [ ] When cloning a private repo: use the configured SSH key for authentication
- [ ] App settings: optional "SSH key" dropdown to select which key to use for this app's repo
- [ ] Support both `git@github.com:user/repo.git` and `https://` URLs (SSH key only used for git@ URLs)

### Dashboard UI
- [ ] User profile page: "SSH Keys" section (between API Tokens and Sessions)
- [ ] Key list: name, fingerprint, key type, created date, last used
- [ ] Generate key dialog: name input, key type selector (Ed25519 recommended, RSA optional)
- [ ] Import key dialog: name input, private key textarea
- [ ] After generation: show public key with copy button and instruction text for adding to git providers

### API Endpoints
- [ ] `GET /user/ssh-keys` — list user's keys (public key + metadata only)
- [ ] `POST /user/ssh-keys/generate` — generate new keypair
- [ ] `POST /user/ssh-keys/import` — import existing private key
- [ ] `DELETE /user/ssh-keys/{id}` — delete key
- [ ] `GET /user/ssh-keys/{id}/public` — get public key text

## Technical Notes

- Use `ssh-key` or `russh-keys` crate for Ed25519/RSA key generation
- Private keys should be zeroized from memory after encryption and storage
- When used for git clone: write the key to a temp file, use `GIT_SSH_COMMAND` with `-i` flag, delete after clone
- For multi-server: SSH keys are user-scoped, not server-scoped (agent enrollment uses its own X25519 keys)

## Out of Scope

- SSH key deployment to remote servers (keys for connecting TO servers — that's agent enrollment)
- SSH agent forwarding
- Key rotation automation

## Dependencies

- IF-083 (User profile page — UI location)
- IF-002 (Database encryption)
