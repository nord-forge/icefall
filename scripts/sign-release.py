#!/usr/bin/env python3
"""
Build and sign the Icefall release manifest.

Reads SHA-256 checksum files from the build artifacts, assembles a JSON manifest,
signs it with the Ed25519 release key, and writes the manifest + signature files.

Environment variables:
    SIGNING_KEY_B64  - Base64-encoded Ed25519 private key (PEM)
    VERSION          - Release version without 'v' prefix (e.g., "1.2.0")
    REPO             - GitHub repository (e.g., "owner/icefall")
"""

import base64
import hashlib
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

try:
    from nacl.signing import SigningKey
except ImportError:
    print("ERROR: pynacl not installed. Run: pip install pynacl")
    sys.exit(1)


def load_signing_key() -> SigningKey:
    key_b64 = os.environ.get("SIGNING_KEY_B64", "")
    if not key_b64:
        print("ERROR: SIGNING_KEY_B64 environment variable not set")
        sys.exit(1)

    pem_bytes = base64.b64decode(key_b64)
    pem_text = pem_bytes.decode("utf-8", errors="replace")

    # Extract the base64 body from PEM
    lines = [
        line
        for line in pem_text.strip().splitlines()
        if not line.startswith("-----")
    ]
    der_bytes = base64.b64decode("".join(lines))

    # Ed25519 private key in PKCS#8: 16-byte header + 34 bytes (2-byte length prefix + 32-byte key)
    # The raw 32-byte seed starts at offset 16+2 = 18 in most Ed25519 PKCS#8 encodings
    if len(der_bytes) == 48:
        seed = der_bytes[16:]
        # The seed might have a 2-byte ASN.1 octet string wrapper
        if seed[0] == 0x04 and seed[1] == 0x20:
            seed = seed[2:]
    elif len(der_bytes) == 32:
        seed = der_bytes
    else:
        # Try extracting the last 32 bytes as a fallback
        seed = der_bytes[-32:]

    if len(seed) != 32:
        print(f"ERROR: Could not extract 32-byte Ed25519 seed (got {len(seed)} bytes)")
        sys.exit(1)

    return SigningKey(seed)


def read_checksum(artifact_name: str) -> tuple[str, int]:
    """Read SHA-256 and file size from build artifacts."""
    # Look in the downloaded artifacts directory structure
    for pattern_dir in Path("artifacts").glob("release-*"):
        checksum_file = pattern_dir / f"{artifact_name}.sha256"
        tarball_file = pattern_dir / artifact_name
        if checksum_file.exists() and tarball_file.exists():
            sha256 = checksum_file.read_text().strip().split()[0]
            size = tarball_file.stat().st_size
            return sha256, size

    print(f"ERROR: Could not find checksum for {artifact_name}")
    sys.exit(1)


def compute_key_fingerprint(signing_key: SigningKey) -> str:
    """Compute SHA-256 fingerprint of the public key."""
    pub_bytes = bytes(signing_key.verify_key)
    digest = hashlib.sha256(pub_bytes).hexdigest()[:16]
    return f"sha256:{digest}"


def main():
    version = os.environ.get("VERSION", "")
    repo = os.environ.get("REPO", "")

    if not version:
        print("ERROR: VERSION environment variable not set")
        sys.exit(1)
    if not repo:
        print("ERROR: REPO environment variable not set")
        sys.exit(1)

    signing_key = load_signing_key()
    fingerprint = compute_key_fingerprint(signing_key)
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    base_url = f"https://github.com/{repo}/releases/download/v{version}"

    # Determine channel from version string
    channel = "stable"
    if "beta" in version or "rc" in version:
        channel = "beta"
    elif "nightly" in version:
        channel = "nightly"

    artifacts = {}
    agent_artifacts = {}
    for arch_label, artifact_suffix in [
        ("x86_64-linux", "x86_64-linux"),
        ("aarch64-linux", "aarch64-linux"),
    ]:
        tarball_name = f"icefall-v{version}-{artifact_suffix}.tar.gz"
        try:
            sha256, size = read_checksum(tarball_name)
            artifacts[arch_label] = {
                "url": f"{base_url}/{tarball_name}",
                "sha256": sha256,
                "size_bytes": size,
            }
        except SystemExit:
            print(f"WARNING: Skipping control plane {arch_label} (artifact not found)")
            continue

        agent_tarball = f"icefall-agent-v{version}-{artifact_suffix}.tar.gz"
        try:
            agent_sha256, agent_size = read_checksum(agent_tarball)
            agent_artifacts[arch_label] = {
                "url": f"{base_url}/{agent_tarball}",
                "sha256": agent_sha256,
                "size_bytes": agent_size,
            }
        except SystemExit:
            print(f"WARNING: Skipping agent {arch_label} (artifact not found)")
            continue

    if not artifacts:
        print("ERROR: No artifacts found to include in manifest")
        sys.exit(1)

    manifest = {
        "schema_version": 1,
        "version": version,
        "release_date": now,
        "min_supported_version": "0.1.0",
        "channel": channel,
        "requires_migration": False,
        "breaking": False,
        "breaking_changes": None,
        "changelog_url": f"https://github.com/{repo}/releases/tag/v{version}",
        "artifacts": dict(sorted(artifacts.items())),
        "agent_artifacts": dict(sorted(agent_artifacts.items())),
        "signed_by": fingerprint,
        "signature_timestamp": now,
    }

    # Canonicalize: sorted keys, no trailing whitespace, consistent formatting.
    # The verifier checks the signature against the manifest file bytes verbatim,
    # so the bytes we sign MUST be the exact bytes we write to disk — do not
    # pretty-print the file separately (that mismatch breaks verification).
    manifest_json = json.dumps(manifest, sort_keys=True, separators=(",", ":"))
    manifest_bytes = manifest_json.encode("utf-8")

    # Sign the canonical manifest bytes
    signed = signing_key.sign(manifest_bytes)
    signature_b64 = base64.b64encode(signed.signature).decode("ascii")

    # Write the SAME canonical bytes that were signed.
    manifest_file = f"icefall-v{version}-manifest.json"
    with open(manifest_file, "wb") as f:
        f.write(manifest_bytes)

    # Write signature
    sig_file = f"icefall-v{version}-manifest.json.sig"
    with open(sig_file, "w") as f:
        f.write(signature_b64)

    # Self-check: verify the signature against the bytes actually written to the
    # manifest file. The client verifies the downloaded file verbatim, so if the
    # written bytes differ from the signed bytes the release is unverifiable.
    # Fail the build here rather than ship a manifest no client can trust.
    written_bytes = Path(manifest_file).read_bytes()
    try:
        signing_key.verify_key.verify(written_bytes, signed.signature)
    except Exception as exc:  # noqa: BLE001
        print(f"ERROR: self-verification failed — written manifest bytes do not "
              f"match signed bytes: {exc}")
        sys.exit(1)

    print(f"Manifest written to {manifest_file}")
    print(f"Signature written to {sig_file}")
    print(f"Signed by key: {fingerprint}")
    print(f"Control plane artifacts: {', '.join(artifacts.keys())}")
    print(f"Agent artifacts: {', '.join(agent_artifacts.keys())}")


if __name__ == "__main__":
    main()
