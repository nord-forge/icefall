import { useState, useRef } from 'preact/hooks';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import type { App } from '@lib/types';
import { Upload, FileJson, X } from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import styles from './import-bundle-step.module.css';

type Props = {
  onImport: (app: App) => void;
};

type BundlePreview = {
  name?: string;
  git_repo?: string;
  framework?: string;
  deploy_mode?: string;
  [key: string]: unknown;
};

export default function ImportBundleStep({ onImport }: Props) {
  const [preview, setPreview] = useState<BundlePreview | null>(null);
  const [rawBundle, setRawBundle] = useState<Record<string, unknown> | null>(null);
  const [error, setError] = useState('');
  const [importing, setImporting] = useState(false);
  const [dragActive, setDragActive] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);

  function parseFile(file: File) {
    setError('');
    setPreview(null);
    setRawBundle(null);

    const reader = new FileReader();
    reader.onload = () => {
      try {
        const parsed = JSON.parse(reader.result as string);
        if (typeof parsed !== 'object' || parsed === null) {
          setError('Invalid bundle: expected a JSON object');
          return;
        }
        setPreview(parsed as BundlePreview);
        setRawBundle(parsed as Record<string, unknown>);
      } catch {
        setError('Invalid JSON file. Please select a valid .icefall bundle.');
      }
    };
    reader.onerror = () => {
      setError('Failed to read file');
    };
    reader.readAsText(file);
  }

  function handleFileChange(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (file) parseFile(file);
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    setDragActive(false);
    const file = e.dataTransfer?.files?.[0];
    if (file) parseFile(file);
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    setDragActive(true);
  }

  function handleDragLeave() {
    setDragActive(false);
  }

  function clearPreview() {
    setPreview(null);
    setRawBundle(null);
    setError('');
    if (fileRef.current) fileRef.current.value = '';
  }

  async function handleImport() {
    if (!rawBundle) return;
    setImporting(true);
    try {
      const { data } = await api.importBundle(rawBundle);
      addToast('success', `App "${data.name}" imported successfully`);
      onImport(data);
    } catch (err: any) {
      addToast('error', err.message || 'Failed to import bundle');
    }
    setImporting(false);
  }

  function settingsSummary(): string {
    const parts: string[] = [];
    if (preview?.framework) parts.push(preview.framework);
    if (preview?.deploy_mode) parts.push(preview.deploy_mode);
    return parts.length > 0 ? parts.join(', ') : 'Default settings';
  }

  return (
    <div class={styles.container}>
      {/* a11y [WCAG 1.3.1]: label associated to hidden file input */}
      <label
        htmlFor="bundle-file-input"
        class={`${styles.dropZone} ${dragActive ? styles.dropZoneActive : ''}`}
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
      >
        <Upload size={24} aria-hidden="true" />
        <span class={styles.dropText}>
          Drop an .icefall bundle here, or click to select a file
        </span>
        <input
          ref={fileRef}
          id="bundle-file-input"
          type="file"
          accept=".json"
          class={styles.fileInput}
          onChange={handleFileChange}
        />
      </label>

      {error && (
        <p class={styles.errorMessage} role="alert">
          {error}
        </p>
      )}

      {preview && (
        <div class={styles.preview}>
          <div class={styles.previewHeader}>
            <FileJson size={18} aria-hidden="true" />
            <span class={styles.previewTitle}>
              {preview.name || 'Unnamed app'}
            </span>
            <button
              type="button"
              class={styles.clearButton}
              onClick={clearPreview}
              aria-label="Clear selected file"
            >
              <X size={14} />
            </button>
          </div>
          {preview.git_repo && (
            <p class={styles.previewMeta}>Repository: {preview.git_repo}</p>
          )}
          <p class={styles.previewMeta}>Settings: {settingsSummary()}</p>
          <div class={styles.previewActions}>
            <Button variant="ghost" onClick={clearPreview}>
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={handleImport}
              loading={importing}
            >
              Import app
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
