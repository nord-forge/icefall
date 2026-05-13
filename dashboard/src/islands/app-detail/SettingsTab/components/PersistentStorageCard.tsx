import Button from '@islands/shared/Button/Button';
import Select from '@islands/shared/Select/Select';
import { HardDrive, Cloud, X, Plus, Search } from 'lucide-preact';
import styles from '../settings-tab.module.css';
import formStyles from '@styles/form.module.css';

type LocalVolume = {
  type: 'local';
  source: string;
  target: string;
  read_only: boolean;
};

type S3Volume = {
  type: 's3';
  bucket: string;
  endpoint: string;
  access_key: string;
  secret_key: string;
  region: string;
  target: string;
  read_only: boolean;
};

type VolumeEntry = LocalVolume | S3Volume;

type Props = {
  volumes: VolumeEntry[];
  volumeErrors: Record<number, string>;
  onAddVolume: (type: 'local' | 's3') => void;
  onRemoveVolume: (index: number) => void;
  onUpdateVolume: (index: number, field: string, value: string | boolean) => void;
  onSwitchVolumeType: (index: number, newType: 'local' | 's3') => void;
  onBrowseVolume: (index: number) => void;
};

export default function PersistentStorageCard({
  volumes,
  volumeErrors,
  onAddVolume,
  onRemoveVolume,
  onUpdateVolume,
  onSwitchVolumeType,
  onBrowseVolume,
}: Props) {
  return (
    <div class={styles.card}>
      <h2 class={styles.sectionTitle}>
        <HardDrive size={18} /> Persistent Storage
      </h2>
      <p class={styles.settingsDescription}>
        Mount named volumes, host paths, or S3-compatible object storage into your container. Data in these mounts persists across deployments.
      </p>

      {volumes.length > 0 && (
        <div class={styles.volumeList} role="list" aria-label="Volume mounts">
          {volumes.map((vol, index) => (
            <div key={index} class={styles.volumeRow} role="listitem">
              <div class={styles.volumeFields}>
                {/* a11y [WCAG 4.1.2]: select has associated label via htmlFor/id */}
                <div class={styles.volumeTypeRow}>
                  <label htmlFor={`vol-type-${index}`} class={formStyles.label}>Type</label>
                  <Select
                    id={`vol-type-${index}`}
                    options={[
                      { value: 'local', label: 'Local Volume' },
                      { value: 's3', label: 'S3 Mount' },
                    ]}
                    value={vol.type}
                    onChange={(v) => onSwitchVolumeType(index, v as 'local' | 's3')}
                    fullWidth
                  />
                </div>

                {vol.type === 'local' ? (
                  <>
                    <div>
                      <label htmlFor={`vol-source-${index}`} class={formStyles.label}>Source</label>
                      <input
                        id={`vol-source-${index}`}
                        class={formStyles.inputMono}
                        value={vol.source}
                        onInput={(e) => onUpdateVolume(index, 'source', (e.target as HTMLInputElement).value)}
                        placeholder="myapp-data"
                      />
                      <span class={styles.fieldHint}>Volume name or host path</span>
                    </div>
                    <div>
                      <label htmlFor={`vol-target-${index}`} class={formStyles.label}>Container Path</label>
                      <input
                        id={`vol-target-${index}`}
                        class={formStyles.inputMono}
                        value={vol.target}
                        onInput={(e) => onUpdateVolume(index, 'target', (e.target as HTMLInputElement).value)}
                        placeholder="/app/data"
                        aria-invalid={!!volumeErrors[index]}
                        aria-describedby={volumeErrors[index] ? `vol-error-${index}` : undefined}
                      />
                      {volumeErrors[index] ? (
                        <span id={`vol-error-${index}`} class={styles.volumeError} role="alert">{volumeErrors[index]}</span>
                      ) : (
                        <span class={styles.fieldHint}>Must start with /</span>
                      )}
                    </div>
                  </>
                ) : (
                  <>
                    <div>
                      <label htmlFor={`vol-bucket-${index}`} class={formStyles.label}>
                        <Cloud size={14} style={{ verticalAlign: 'text-bottom', marginRight: '4px' }} />
                        Bucket Name
                      </label>
                      <input
                        id={`vol-bucket-${index}`}
                        class={formStyles.inputMono}
                        value={vol.bucket}
                        onInput={(e) => onUpdateVolume(index, 'bucket', (e.target as HTMLInputElement).value)}
                        placeholder="my-bucket"
                      />
                    </div>
                    <div>
                      <label htmlFor={`vol-endpoint-${index}`} class={formStyles.label}>Endpoint URL</label>
                      <input
                        id={`vol-endpoint-${index}`}
                        class={formStyles.inputMono}
                        value={vol.endpoint}
                        onInput={(e) => onUpdateVolume(index, 'endpoint', (e.target as HTMLInputElement).value)}
                        placeholder="https://s3.amazonaws.com"
                      />
                      <span class={styles.fieldHint}>S3-compatible endpoint (R2, MinIO, etc.)</span>
                    </div>
                    <div>
                      <label htmlFor={`vol-accesskey-${index}`} class={formStyles.label}>Access Key</label>
                      <input
                        id={`vol-accesskey-${index}`}
                        class={formStyles.inputMono}
                        value={vol.access_key}
                        onInput={(e) => onUpdateVolume(index, 'access_key', (e.target as HTMLInputElement).value)}
                        placeholder="AKIA..."
                      />
                    </div>
                    <div>
                      <label htmlFor={`vol-secretkey-${index}`} class={formStyles.label}>Secret Key</label>
                      <input
                        id={`vol-secretkey-${index}`}
                        class={formStyles.inputMono}
                        type="password"
                        value={vol.secret_key}
                        onInput={(e) => onUpdateVolume(index, 'secret_key', (e.target as HTMLInputElement).value)}
                        placeholder="Secret key"
                        autocomplete="off"
                      />
                    </div>
                    <div>
                      <label htmlFor={`vol-region-${index}`} class={formStyles.label}>Region</label>
                      <input
                        id={`vol-region-${index}`}
                        class={formStyles.inputMono}
                        value={vol.region}
                        onInput={(e) => onUpdateVolume(index, 'region', (e.target as HTMLInputElement).value)}
                        placeholder="auto"
                      />
                      <span class={styles.fieldHint}>Use "auto" for most S3-compatible providers.</span>
                    </div>
                    <div>
                      <label htmlFor={`vol-target-${index}`} class={formStyles.label}>Container Path</label>
                      <input
                        id={`vol-target-${index}`}
                        class={formStyles.inputMono}
                        value={vol.target}
                        onInput={(e) => onUpdateVolume(index, 'target', (e.target as HTMLInputElement).value)}
                        placeholder="/app/s3"
                        aria-invalid={!!volumeErrors[index]}
                        aria-describedby={volumeErrors[index] ? `vol-error-${index}` : undefined}
                      />
                      {volumeErrors[index] ? (
                        <span id={`vol-error-${index}`} class={styles.volumeError} role="alert">{volumeErrors[index]}</span>
                      ) : (
                        <span class={styles.fieldHint}>Must start with /</span>
                      )}
                    </div>
                  </>
                )}
              </div>
              <div class={styles.volumeActions}>
                <label class={styles.toggleLabel}>
                  <input
                    type="checkbox"
                    class={formStyles.checkbox}
                    checked={vol.read_only}
                    onChange={(e) => onUpdateVolume(index, 'read_only', (e.target as HTMLInputElement).checked)}
                  />
                  Read-only
                </label>
                {vol.type !== 's3' && vol.target && (
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => onBrowseVolume(index)}
                    aria-label={`Browse volume ${vol.type === 'local' ? vol.source : ''} mounted at ${vol.target}`}
                  >
                    <Search size={14} /> Browse
                  </Button>
                )}
                {/* a11y [WCAG 4.1.2]: button has accessible name via aria-label */}
                <button
                  type="button"
                  class={styles.volumeRemove}
                  onClick={() => onRemoveVolume(index)}
                  aria-label={`Remove volume mount ${vol.type === 'local' ? (vol.source || 'unnamed') : (vol.bucket || 'unnamed bucket')} to ${vol.target || 'unset'}`}
                >
                  <X size={14} />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      <div class={styles.volumeAddRow} style={{ marginTop: volumes.length > 0 ? 'var(--space-3)' : '0' }}>
        <Button variant="secondary" onClick={() => onAddVolume('local')}>
          <Plus size={14} /> Add Volume
        </Button>
        <Button variant="secondary" onClick={() => onAddVolume('s3')}>
          <Cloud size={14} /> Add S3 Mount
        </Button>
      </div>
      <p class={styles.settingsNote}>Changes take effect on next deployment. S3 mounts use an rclone sidecar container with FUSE.</p>
    </div>
  );
}
