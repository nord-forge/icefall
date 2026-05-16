# BE-269: Image transfer Podman verification

**Phase:** 32
**Priority:** High
**Size:** S
**Dependencies:** BE-265

## Description

Phase 31 added a chunked image-transfer pipeline: the control plane runs
`export_image` (`docker save`), gzips and chunks the tar, streams it to a
remote agent, which reassembles it and runs `import_image` (`docker load`).
This must be verified — and fixed if needed — against Podman, where the
exporting side, the receiving side, or both may be Podman.

## Changes

- Verify `DockerClient::export_image` produces an archive Podman's
  `import_image` accepts, and vice versa (Docker-built image loaded into
  Podman and Podman-built image loaded into Docker).
- Podman `export_image` may default to OCI format; ensure the format both
  ends agree on (force `docker`-format export if needed via bollard options).
- Confirm multi-tag archives load correctly on Podman (`import_image` edge
  case noted in the Phase 32 investigation).
- If a mixed Docker↔Podman transfer is not safe, restrict multi-instance
  deploys to a homogeneous runtime across the target servers and document it.

## Acceptance Criteria

- Given a Docker control plane and a Podman target, when a multi-instance
  deploy runs, then the image transfers and the container starts on the target.
- Given a Podman control plane and a Podman target, when a deploy runs, then
  the image transfers and loads correctly.
- Given a multi-tag image, when transferred, then all tags survive the load.

## Out of Scope

Private registry-based distribution (a separate future optimization, already
out of scope for Phase 31).
