# Release Signing Keys

This directory contains the public keys used to verify Icefall release artifacts.

## Current Key

- **ID**: `icefall-release-2026`
- **Fingerprint**: `sha256:cd5a5845d0ccce1f`
- **Public key**: `icefall-release.pub`
- **Active since**: 2026-01-01

## Generating a Signing Keypair

Run this on a secure, offline machine:

```bash
openssl genpkey -algorithm ED25519 -out icefall-release.pem
openssl pkey -in icefall-release.pem -pubout -out icefall-release.pub
```

Then:

1. Copy `icefall-release.pub` to this directory
2. Base64-encode the private key and store it as the `ICEFALL_RELEASE_SIGNING_KEY`
   GitHub Actions secret
3. Update `src/update/keys.rs` with the real public key PEM

## Key Rotation

See the rotation procedure in `.claude/design/self-update-system.md`, section 4.
