import { useCallback, useEffect, useRef, useState } from 'preact/hooks';
import { X, FolderOpen, File, Download, Trash2, Upload, ChevronRight, HardDrive, FolderUp, AlertTriangle } from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import { api } from '@lib/api';
import styles from './volume-browser.module.css';

type FileEntry = {
  name: string;
  size: number;
  modified: string;
  is_dir: boolean;
  permissions: string;
};

type VolumeMount = {
  source: string;
  target: string;
  read_only: boolean;
};

type Props = {
  appId: string;
  mountIndex: number;
  volume: VolumeMount;
  onClose: () => void;
};

const API_BASE = '/api/v1';

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const val = bytes / Math.pow(1024, i);
  return `${val.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

export default function VolumeBrowser({ appId, mountIndex, volume, onClose }: Props) {
  const [currentPath, setCurrentPath] = useState('/');
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [volumeSize, setVolumeSize] = useState<number | null>(null);
  const [showUpload, setShowUpload] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const overlayRef = useRef<HTMLDivElement>(null);

  const fetchEntries = useCallback(async (path: string) => {
    setLoading(true);
    setError('');
    try {
      const json = await api.browseVolume(appId, mountIndex, path);
      setEntries(json.data || []);
      setCurrentPath(path);
    } catch (err: any) {
      setError(err.message || 'Failed to browse volume');
      setEntries([]);
    } finally {
      setLoading(false);
    }
  }, [appId, mountIndex]);

  const fetchSize = useCallback(async () => {
    try {
      const json = await api.volumeSize(appId, mountIndex);
      setVolumeSize(json.data?.bytes_used ?? null);
    } catch {
      // Size tracking is non-critical
    }
  }, [appId, mountIndex]);

  useEffect(() => {
    fetchEntries('/');
    fetchSize();
  }, [fetchEntries, fetchSize]);

  // a11y [WCAG 2.4.3]: focus trap — keep Tab cycling within the drawer
  useEffect(() => {
    const dialog = overlayRef.current;
    if (!dialog) return;
    const focusable = dialog.querySelectorAll<HTMLElement>(
      'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])'
    );
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    function handleTab(e: KeyboardEvent) {
      if (e.key !== 'Tab') return;
      if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last?.focus(); }
      else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first?.focus(); }
    }
    dialog.addEventListener('keydown', handleTab);
    first?.focus();
    return () => dialog.removeEventListener('keydown', handleTab);
  }, [showUpload, deleteTarget, loading, entries]);

  // Close on Escape key
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        if (showUpload) {
          setShowUpload(false);
        } else if (deleteTarget) {
          setDeleteTarget(null);
        } else {
          onClose();
        }
      }
    }
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [onClose, showUpload, deleteTarget]);

  // Close on overlay click
  function handleOverlayClick(e: Event) {
    if (e.target === overlayRef.current) {
      onClose();
    }
  }

  function navigateTo(path: string) {
    fetchEntries(path);
  }

  function navigateUp() {
    if (currentPath === '/' || currentPath === '') return;
    const parts = currentPath.split('/').filter(Boolean);
    parts.pop();
    const parent = parts.length === 0 ? '/' : '/' + parts.join('/');
    navigateTo(parent);
  }

  function handleEntryClick(entry: FileEntry) {
    if (entry.is_dir) {
      const next = currentPath === '/'
        ? '/' + entry.name
        : currentPath + '/' + entry.name;
      navigateTo(next);
    }
  }

  function handleDownload(entry: FileEntry) {
    const filePath = currentPath === '/'
      ? '/' + entry.name
      : currentPath + '/' + entry.name;
    const url = `${API_BASE}/apps/${appId}/volumes/${mountIndex}/download?path=${encodeURIComponent(filePath)}`;
    const a = document.createElement('a');
    a.href = url;
    a.download = entry.name;
    a.click();
  }

  async function handleUpload() {
    if (!selectedFile) return;
    setUploading(true);
    try {
      const arrayBuffer = await selectedFile.arrayBuffer();
      await api.uploadVolumeFile(appId, mountIndex, currentPath, selectedFile.name, arrayBuffer);
      setShowUpload(false);
      setSelectedFile(null);
      fetchEntries(currentPath);
      fetchSize();
    } catch (err: any) {
      setError(err.message || 'Upload failed');
    } finally {
      setUploading(false);
    }
  }

  async function handleDelete() {
    if (!deleteTarget) return;
    setDeleting(true);
    try {
      const filePath = currentPath === '/'
        ? '/' + deleteTarget
        : currentPath + '/' + deleteTarget;
      await api.deleteVolumeFile(appId, mountIndex, filePath);
      setDeleteTarget(null);
      fetchEntries(currentPath);
      fetchSize();
    } catch (err: any) {
      setError(err.message || 'Delete failed');
    } finally {
      setDeleting(false);
    }
  }

  // Build breadcrumb segments
  const pathParts = currentPath.split('/').filter(Boolean);
  const breadcrumbs = [
    { label: volume.target, path: '/' },
    ...pathParts.map((part, i) => ({
      label: part,
      path: '/' + pathParts.slice(0, i + 1).join('/'),
    })),
  ];

  // Sort: directories first, then by name
  const sortedEntries = [...entries].sort((a, b) => {
    if (a.is_dir && !b.is_dir) return -1;
    if (!a.is_dir && b.is_dir) return 1;
    return a.name.localeCompare(b.name);
  });

  return (
    <div
      class={styles.overlay}
      ref={overlayRef}
      onClick={handleOverlayClick}
      role="dialog"
      aria-modal="true"
      aria-label={`Volume browser for ${volume.source}`}
    >
      <div class={styles.drawer}>
        {/* Header */}
        <div class={styles.header}>
          <div class={styles.headerTitle}>
            <HardDrive size={18} />
            <span>{volume.source}</span>
          </div>
          {/* a11y [WCAG 4.1.2]: button has accessible name */}
          <button
            type="button"
            class={styles.closeButton}
            onClick={onClose}
            aria-label="Close volume browser"
          >
            <X size={18} />
          </button>
        </div>

        {/* Breadcrumb toolbar */}
        <div class={styles.toolbar}>
          <nav class={styles.breadcrumbs} aria-label="Volume path">
            {currentPath !== '/' && (
              <button
                type="button"
                class={styles.breadcrumbLink}
                onClick={navigateUp}
                aria-label="Go to parent directory"
              >
                <FolderUp size={14} />
              </button>
            )}
            {breadcrumbs.map((crumb, i) => (
              <>
                {i > 0 && (
                  <span class={styles.breadcrumbSep} aria-hidden="true">
                    <ChevronRight size={12} />
                  </span>
                )}
                {i < breadcrumbs.length - 1 ? (
                  <button
                    type="button"
                    class={styles.breadcrumbLink}
                    onClick={() => navigateTo(crumb.path)}
                  >
                    {crumb.label}
                  </button>
                ) : (
                  <span class={styles.breadcrumbCurrent}>{crumb.label}</span>
                )}
              </>
            ))}
          </nav>
          <div class={styles.toolbarActions}>
            {!volume.read_only && (
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setShowUpload(true)}
                aria-label="Upload file to current directory"
              >
                <Upload size={14} /> Upload
              </Button>
            )}
          </div>
        </div>

        {/* Volume size */}
        {volumeSize !== null && (
          <div class={styles.sizeBar}>
            <HardDrive size={12} />
            Volume usage: {formatBytes(volumeSize)}
          </div>
        )}

        {/* File list */}
        <div class={styles.fileList}>
          {loading ? (
            <div class={styles.loadingState} role="status" aria-live="polite">
              Loading...
            </div>
          ) : error ? (
            <div class={styles.errorState} role="alert">
              <AlertTriangle size={24} />
              <p>{error}</p>
              <Button variant="secondary" size="sm" onClick={() => fetchEntries(currentPath)}>
                Retry
              </Button>
            </div>
          ) : sortedEntries.length === 0 ? (
            <div class={styles.emptyState}>
              <FolderOpen size={32} />
              <p>This directory is empty.</p>
            </div>
          ) : (
            <table class={styles.fileTable}>
              <thead>
                <tr>
                  <th>Name</th>
                  <th>Size</th>
                  <th>Modified</th>
                  <th>Permissions</th>
                  <th aria-label="Actions" />
                </tr>
              </thead>
              <tbody>
                {sortedEntries.map((entry) => (
                  <tr
                    key={entry.name}
                    class={entry.is_dir ? styles.fileRowClickable : styles.fileRow}
                    onClick={entry.is_dir ? () => handleEntryClick(entry) : undefined}
                  >
                    <td>
                      <span class={entry.is_dir ? styles.dirName : styles.fileName}>
                        {entry.is_dir
                          ? <FolderOpen size={14} />
                          : <File size={14} />
                        }
                        {entry.name}
                      </span>
                    </td>
                    <td class={styles.fileMeta}>
                      {entry.is_dir ? '--' : formatBytes(entry.size)}
                    </td>
                    <td class={styles.fileMeta}>{entry.modified}</td>
                    <td class={styles.fileMeta}>{entry.permissions}</td>
                    <td>
                      <div class={styles.fileActions}>
                        {!entry.is_dir && (
                          <button
                            type="button"
                            class={styles.actionButton}
                            onClick={(e) => {
                              e.stopPropagation();
                              handleDownload(entry);
                            }}
                            aria-label={`Download ${entry.name}`}
                          >
                            <Download size={14} />
                          </button>
                        )}
                        {!volume.read_only && (
                          <button
                            type="button"
                            class={`${styles.actionButton} ${styles.deleteAction}`}
                            onClick={(e) => {
                              e.stopPropagation();
                              setDeleteTarget(entry.name);
                            }}
                            aria-label={`Delete ${entry.name}`}
                          >
                            <Trash2 size={14} />
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        {/* Delete confirmation */}
        {deleteTarget && (
          <div class={styles.deleteConfirm} role="alert">
            <p class={styles.deleteConfirmText}>
              Delete <strong>{deleteTarget}</strong>? This cannot be undone.
            </p>
            <div class={styles.deleteConfirmActions}>
              <Button variant="ghost" size="sm" onClick={() => setDeleteTarget(null)}>
                Cancel
              </Button>
              <Button variant="danger" size="sm" onClick={handleDelete} loading={deleting}>
                <Trash2 size={14} /> Delete
              </Button>
            </div>
          </div>
        )}
      </div>

      {/* Upload dialog */}
      {showUpload && (
        <div
          class={styles.uploadOverlay}
          onClick={(e) => { if (e.target === e.currentTarget) setShowUpload(false); }}
          role="dialog"
          aria-modal="true"
          aria-label="Upload file"
        >
          <div class={styles.uploadDialog}>
            <h3 class={styles.uploadTitle}>Upload File</h3>
            <p style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-secondary)' }}>
              Upload to: <code>{volume.target}{currentPath === '/' ? '' : currentPath}</code>
            </p>

            {!selectedFile ? (
              <div
                class={styles.uploadDropzone}
                onClick={() => fileInputRef.current?.click()}
                onDragOver={(e) => { e.preventDefault(); }}
                onDrop={(e) => {
                  e.preventDefault();
                  const files = (e as DragEvent).dataTransfer?.files;
                  if (files && files.length > 0) {
                    setSelectedFile(files[0]);
                  }
                }}
                role="button"
                tabIndex={0}
                aria-label="Select file to upload"
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    fileInputRef.current?.click();
                  }
                }}
              >
                <Upload size={24} />
                <span>Click or drag a file here</span>
                <span style={{ fontSize: 'var(--text-xs)' }}>Maximum 50 MB</span>
              </div>
            ) : (
              <div class={styles.uploadSelectedFile}>
                <File size={14} />
                {selectedFile.name}
                <span style={{ color: 'var(--color-text-muted)', marginLeft: 'auto' }}>
                  {formatBytes(selectedFile.size)}
                </span>
              </div>
            )}

            <input
              ref={fileInputRef}
              type="file"
              class={styles.uploadHiddenInput}
              onChange={(e) => {
                const files = (e.target as HTMLInputElement).files;
                if (files && files.length > 0) {
                  setSelectedFile(files[0]);
                }
              }}
              aria-label="File input"
            />

            <div class={styles.uploadActions}>
              <Button variant="ghost" onClick={() => { setShowUpload(false); setSelectedFile(null); }}>
                Cancel
              </Button>
              <Button
                variant="primary"
                onClick={handleUpload}
                loading={uploading}
                disabled={!selectedFile}
              >
                <Upload size={14} /> Upload
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
