# IF-080: S3 / object storage mounts

**Phase:** 16 — v1.1 Fast Follow
**Priority:** Low
**Estimate:** M

## Description

Allow apps to mount S3-compatible object storage as a filesystem path using FUSE-based tools (s3fs, goofys, or rclone mount). This enables apps to read/write to S3 buckets as if they were local directories.

## Acceptance Criteria

- [ ] New volume type option: "S3 Mount" alongside existing "Host Path" and "Named Volume"
- [ ] S3 mount configuration:
  - Bucket name
  - Endpoint URL (for S3-compatible providers like R2, MinIO)
  - Access key and secret key (stored encrypted)
  - Region
  - Mount path inside the container
  - Read-only toggle
- [ ] S3 credentials reuse: option to pick from existing backup locations configured in settings
- [ ] Mount implemented via a sidecar container or init container with s3fs/rclone
- [ ] Health check: verify mount is accessible after container start
- [ ] Unmount on container stop

## Technical Notes

- Options: `s3fs-fuse` (most compatible), `goofys` (faster), `rclone mount` (most providers)
- Requires `--privileged` or `--cap-add SYS_ADMIN --device /dev/fuse` on the sidecar container
- Alternative: use rclone as a Docker volume plugin (`rclone/docker-volume-rclone`)
- Consider performance implications — object storage has higher latency than local disk

## Dependencies

- IF-064 (persistent storage UI)
