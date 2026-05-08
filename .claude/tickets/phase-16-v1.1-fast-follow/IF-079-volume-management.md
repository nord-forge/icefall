# IF-079: Volume management & browsing

**Phase:** 16 — v1.1 Fast Follow
**Priority:** Medium
**Estimate:** L

## Description

Extend the persistent storage system (IF-064) with advanced volume management: a file browser UI, volume size limits, volume backups, and the ability to share volumes between apps.

## Acceptance Criteria

### Volume Browser / File Manager
- [ ] New "Browse" button per volume mount in the app settings or a dedicated tab
- [ ] File tree view showing directory contents inside the volume
- [ ] File preview for text files (read-only)
- [ ] Download individual files or folders (as zip)
- [ ] Upload files to a volume path
- [ ] Delete files with confirmation
- [ ] Breadcrumb navigation within the volume

### Volume Size Limits
- [ ] Optional max size field per volume mount (e.g., 1 GB, 5 GB)
- [ ] Display current usage vs limit
- [ ] Warning notification when volume approaches limit (80%, 90%)

### Volume Backups
- [ ] Scheduled backup of named volumes to S3 (separate from database backups)
- [ ] Manual backup trigger per volume
- [ ] Restore volume from backup
- [ ] Backup history and download

### Shared Volumes
- [ ] Option to share a named volume between multiple apps
- [ ] Volume picker when configuring mounts: show existing named volumes
- [ ] Warning when deleting an app that has shared volumes

## Technical Notes

- Volume browsing requires `docker exec` to list files (or a sidecar approach)
- Volume size can be tracked via `docker system df -v` or by exec'ing `du` inside a container
- Volume backups: tar the volume contents via a temporary container, upload to S3
- Shared volumes: Docker named volumes are inherently shareable — the UI just needs to expose existing volumes

## Dependencies

- IF-064 (persistent storage UI)
