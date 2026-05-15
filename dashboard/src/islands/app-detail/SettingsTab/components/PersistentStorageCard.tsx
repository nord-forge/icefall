import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
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
                    <Input
                      label="Source"
                      name={`vol-source-${index}`}
                      id={`vol-source-${index}`}
                      mono
                      value={vol.source}
                      onChange={(v) => onUpdateVolume(index, 'source', v)}
                      placeholder="myapp-data"
                      helpText="Volume name or host path"
                    />
                    <Input
                      label="Container Path"
                      name={`vol-target-${index}`}
                      id={`vol-target-${index}`}
                      mono
                      value={vol.target}
                      onChange={(v) => onUpdateVolume(index, 'target', v)}
                      placeholder="/app/data"
                      error={volumeErrors[index]}
                      helpText={!volumeErrors[index] ? 'Must start with /' : undefined}
                    />
                  </>
                ) : (
                  <>
                    <Input
                      label="Bucket Name"
                      name={`vol-bucket-${index}`}
                      id={`vol-bucket-${index}`}
                      mono
                      value={vol.bucket}
                      onChange={(v) => onUpdateVolume(index, 'bucket', v)}
                      placeholder="my-bucket"
                    />
                    <Input
                      label="Endpoint URL"
                      name={`vol-endpoint-${index}`}
                      id={`vol-endpoint-${index}`}
                      mono
                      value={vol.endpoint}
                      onChange={(v) => onUpdateVolume(index, 'endpoint', v)}
                      placeholder="https://s3.amazonaws.com"
                      helpText="S3-compatible endpoint (R2, MinIO, etc.)"
                    />
                    <Input
                      label="Access Key"
                      name={`vol-accesskey-${index}`}
                      id={`vol-accesskey-${index}`}
                      mono
                      value={vol.access_key}
                      onChange={(v) => onUpdateVolume(index, 'access_key', v)}
                      placeholder="AKIA..."
                    />
                    <Input
                      label="Secret Key"
                      name={`vol-secretkey-${index}`}
                      id={`vol-secretkey-${index}`}
                      mono
                      type="password"
                      revealable
                      value={vol.secret_key}
                      onChange={(v) => onUpdateVolume(index, 'secret_key', v)}
                      placeholder="Secret key"
                    />
                    <Input
                      label="Region"
                      name={`vol-region-${index}`}
                      id={`vol-region-${index}`}
                      mono
                      value={vol.region}
                      onChange={(v) => onUpdateVolume(index, 'region', v)}
                      placeholder="auto"
                      helpText={'Use "auto" for most S3-compatible providers.'}
                    />
                    <Input
                      label="Container Path"
                      name={`vol-target-${index}`}
                      id={`vol-target-${index}`}
                      mono
                      value={vol.target}
                      onChange={(v) => onUpdateVolume(index, 'target', v)}
                      placeholder="/app/s3"
                      error={volumeErrors[index]}
                      helpText={!volumeErrors[index] ? 'Must start with /' : undefined}
                    />
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
