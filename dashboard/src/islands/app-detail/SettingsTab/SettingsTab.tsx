import { useEffect, useState, useRef } from 'preact/hooks';
import type { App, Project, Server, DeployMode } from '@lib/types';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Button from '@islands/shared/Button/Button';
import { Save } from 'lucide-preact';
import VolumeBrowser from '@islands/app-detail/VolumeBrowser/VolumeBrowser';
import GeneralSettingsCard from './components/GeneralSettingsCard';
import DeployModeCard from './components/DeployModeCard';
import TagsCard from './components/TagsCard';
import ResourceLimitsCard from './components/ResourceLimitsCard';
import AutoDeployCard from './components/AutoDeployCard';
import PreviewDeploymentsCard from './components/PreviewDeploymentsCard';
import PersistentStorageCard from './components/PersistentStorageCard';
import ServerPlacementCard from './components/ServerPlacementCard';
import EnvironmentAssignmentCard from './components/EnvironmentAssignmentCard';
import DeployApprovalCard from './components/DeployApprovalCard';
import TunnelCard from './components/TunnelCard';
import CanaryProbeCard from './components/CanaryProbeCard';
import ExportBundleCard from './components/ExportBundleCard';
import DangerZoneCard from './components/DangerZoneCard';
import styles from './settings-tab.module.css';

type Props = {
  app: App;
  servers?: Server[];
}

const TAG_PATTERN = /^[a-z0-9][a-z0-9-]*$/;
const TAG_MAX_LENGTH = 30;

function parseTags(raw: string | null): string[] {
  if (!raw) return [];
  return raw.split(',').map((t) => t.trim()).filter(Boolean);
}

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

function parseVolumes(raw: string | null): VolumeEntry[] {
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.map((v: any): VolumeEntry => {
      if (v.type === 's3') {
        return {
          type: 's3',
          bucket: v.bucket || '',
          endpoint: v.endpoint || '',
          access_key: v.access_key || '',
          secret_key: v.secret_key || '',
          region: v.region || 'auto',
          target: v.target || '',
          read_only: !!v.read_only,
        };
      }
      // Legacy entries without a type field are treated as local.
      return {
        type: 'local',
        source: v.source || '',
        target: v.target || '',
        read_only: !!v.read_only,
      };
    });
  } catch { return []; }
}

function parseResourceLimits(raw: string | null): { memoryMb: string; cpuShares: string } {
  if (!raw) return { memoryMb: '', cpuShares: '' };
  try {
    const parsed = JSON.parse(raw);
    const memoryMb = parsed.memory_bytes ? String(Math.round(parsed.memory_bytes / (1024 * 1024))) : '';
    const cpuShares = parsed.cpu_shares ? String(parsed.cpu_shares) : '';
    return { memoryMb, cpuShares };
  } catch { return { memoryMb: '', cpuShares: '' }; }
}

export default function SettingsTab({ app, servers = [] }: Props) {
  const [form, setForm] = useState(() => {
    let buildCommand = '';
    try {
      const parsed = app.build_config ? JSON.parse(app.build_config) : {};
      buildCommand = parsed.build_command || '';
    } catch { /* malformed build_config JSON */ }
    const limits = parseResourceLimits(app.resource_limits);
    return {
      name: app.name,
      git_repo: app.git_repo || '',
      git_branch: app.git_branch,
      build_command: buildCommand,
      memory_mb: limits.memoryMb,
      cpu_shares: limits.cpuShares,
      preview_enabled: app.preview_enabled,
      preview_branch_pattern: app.preview_branch_pattern || '*',
    };
  });
  const [deployMode, setDeployMode] = useState<DeployMode>(
    (app.deploy_mode as DeployMode) || 'auto'
  );
  const [tags, setTags] = useState<string[]>(() => parseTags(app.tags));
  const [tagInput, setTagInput] = useState('');
  const [tagError, setTagError] = useState('');
  const [volumes, setVolumes] = useState<VolumeEntry[]>(() => parseVolumes(app.volumes));
  const [volumeErrors, setVolumeErrors] = useState<Record<number, string>>({});
  const [saving, setSaving] = useState(false);
  const [saveMessage, setSaveMessage] = useState('');
  const [deleting, setDeleting] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [starting, setStarting] = useState(false);
  const [restarting, setRestarting] = useState(false);
  const [copied, setCopied] = useState('');
  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<string>(app.project_id || '');
  const [browsingVolume, setBrowsingVolume] = useState<number | null>(null);
  const [migrateTarget, setMigrateTarget] = useState('');
  const [showMigrateDialog, setShowMigrateDialog] = useState(false);
  const [migrateAck, setMigrateAck] = useState(false);
  const [migrating, setMigrating] = useState(false);
  const copiedTimeoutRef = useRef<number>();

  const multiServer = servers.length >= 2;
  const currentServer = servers.find((s) => s.id === app.server_id);
  const availableTargets = servers.filter((s) => s.id !== app.server_id && s.status === 'online');

  useEffect(() => {
    api.listProjects().then(({ data }) => setProjects(data)).catch(() => {});

    return () => clearTimeout(copiedTimeoutRef.current);
  }, []);

  const webhookBaseUrl = window.location.origin + '/api/v1/webhooks/' + app.id;
  const hasWebhookSecret = !!app.webhook_secret;

  async function handleSave() {
    setSaving(true);
    try {
      const resourceLimits: Record<string, number> = {};
      if (form.memory_mb) resourceLimits.memory_bytes = parseInt(form.memory_mb, 10) * 1024 * 1024;
      if (form.cpu_shares) resourceLimits.cpu_shares = parseInt(form.cpu_shares, 10);

      const validVolumes = volumes.filter((v) => {
        if (v.type === 's3') return v.bucket && v.target;
        return v.source && v.target;
      });

      await api.updateApp(app.id, {
        name: form.name,
        git_repo: form.git_repo || undefined,
        git_branch: form.git_branch,
        resource_limits: Object.keys(resourceLimits).length > 0 ? JSON.stringify(resourceLimits) : undefined,
        preview_enabled: form.preview_enabled,
        preview_branch_pattern: form.preview_enabled ? form.preview_branch_pattern : undefined,
        tags: tags.join(','),
        volumes: validVolumes.length > 0 ? JSON.stringify(validVolumes) : null,
        project_id: selectedProjectId || null,
        deploy_mode: deployMode,
      } as any);
      setSaveMessage('Saved');
    } catch {
      setSaveMessage('Save failed');
    }
    setSaving(false);
  }

  async function handleDelete() {
    setDeleting(true);
    try {
      await api.deleteApp(app.id);
      window.location.href = '/';
    } catch {
      setDeleting(false);
    }
  }

  async function handleMigrate() {
    if (!migrateTarget || !migrateAck) return;
    setMigrating(true);
    try {
      await api.migrateApp(app.id, migrateTarget, migrateAck);
      addToast('success', `Migration to ${availableTargets.find(s => s.id === migrateTarget)?.name || 'server'} started`);
      setShowMigrateDialog(false);
      window.location.href = `/apps/${app.id}/deploys`;
    } catch (err: any) {
      addToast('error', err.message || 'Migration failed');
    }
    setMigrating(false);
  }

  function copyToClipboard(text: string, label: string) {
    navigator.clipboard.writeText(text);
    setCopied(label);
    clearTimeout(copiedTimeoutRef.current);
    copiedTimeoutRef.current = window.setTimeout(() => setCopied(''), 2000);
  }

  function addTag(raw: string) {
    const value = raw.trim().toLowerCase();
    setTagError('');
    if (!value) return;
    if (value.length > TAG_MAX_LENGTH) {
      setTagError(`Tag must be ${TAG_MAX_LENGTH} characters or fewer.`);
      return;
    }
    if (!TAG_PATTERN.test(value)) {
      setTagError('Tags can only contain lowercase letters, numbers, and hyphens.');
      return;
    }
    if (tags.includes(value)) {
      setTagInput('');
      return;
    }
    setTags([...tags, value]);
    setTagInput('');
  }

  function removeTag(tag: string) {
    setTags(tags.filter((t) => t !== tag));
  }

  function handleTagKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ',') {
      e.preventDefault();
      addTag(tagInput);
    }
    if (e.key === 'Backspace' && !tagInput && tags.length > 0) {
      setTags(tags.slice(0, -1));
    }
  }

  function addVolume(volumeType: 'local' | 's3' = 'local') {
    if (volumeType === 's3') {
      setVolumes([...volumes, {
        type: 's3',
        bucket: '',
        endpoint: '',
        access_key: '',
        secret_key: '',
        region: 'auto',
        target: '',
        read_only: false,
      }]);
    } else {
      setVolumes([...volumes, { type: 'local', source: '', target: '', read_only: false }]);
    }
  }

  function removeVolume(index: number) {
    setVolumes(volumes.filter((_, i) => i !== index));
    setVolumeErrors((prev) => {
      const next = { ...prev };
      delete next[index];
      return next;
    });
  }

  function updateVolume(index: number, field: string, value: string | boolean) {
    const updated = volumes.map((v, i) => {
      if (i !== index) return v;
      return { ...v, [field]: value } as VolumeEntry;
    });
    setVolumes(updated);

    if (field === 'target') {
      const target = value as string;
      if (target && !target.startsWith('/')) {
        setVolumeErrors((prev) => ({ ...prev, [index]: 'Container path must start with /' }));
      } else {
        setVolumeErrors((prev) => {
          const next = { ...prev };
          delete next[index];
          return next;
        });
      }
    }
  }

  function switchVolumeType(index: number, newType: 'local' | 's3') {
    const updated = volumes.map((v, i): VolumeEntry => {
      if (i !== index) return v;
      if (newType === 's3') {
        return {
          type: 's3',
          bucket: '',
          endpoint: '',
          access_key: '',
          secret_key: '',
          region: 'auto',
          target: v.target,
          read_only: v.read_only,
        };
      }
      return {
        type: 'local',
        source: '',
        target: v.target,
        read_only: v.read_only,
      };
    });
    setVolumes(updated);
  }

  return (
    <div class={styles.container}>
      <GeneralSettingsCard
        name={form.name}
        gitRepo={form.git_repo}
        gitBranch={form.git_branch}
        buildCommand={form.build_command}
        projects={projects}
        selectedProjectId={selectedProjectId}
        onNameChange={(v) => setForm({ ...form, name: v })}
        onGitRepoChange={(v) => setForm({ ...form, git_repo: v })}
        onGitBranchChange={(v) => setForm({ ...form, git_branch: v })}
        onBuildCommandChange={(v) => setForm({ ...form, build_command: v })}
        onProjectChange={setSelectedProjectId}
      />

      <EnvironmentAssignmentCard app={app} projectId={selectedProjectId || null} />

      <DeployModeCard
        deployMode={deployMode}
        onDeployModeChange={setDeployMode}
      />

      <TagsCard
        tags={tags}
        tagInput={tagInput}
        tagError={tagError}
        tagMaxLength={TAG_MAX_LENGTH}
        onTagInputChange={setTagInput}
        onAddTag={addTag}
        onRemoveTag={removeTag}
        onTagKeyDown={handleTagKeyDown}
      />

      <ResourceLimitsCard
        memoryMb={form.memory_mb}
        cpuShares={form.cpu_shares}
        onMemoryMbChange={(v) => setForm({ ...form, memory_mb: v })}
        onCpuSharesChange={(v) => setForm({ ...form, cpu_shares: v })}
      />

      <AutoDeployCard
        webhookBaseUrl={webhookBaseUrl}
        webhookSecret={app.webhook_secret}
        hasWebhookSecret={hasWebhookSecret}
        gitBranch={app.git_branch}
        copied={copied}
        onCopy={copyToClipboard}
      />

      <PreviewDeploymentsCard
        previewEnabled={form.preview_enabled}
        previewBranchPattern={form.preview_branch_pattern}
        onPreviewEnabledChange={(v) => setForm({ ...form, preview_enabled: v })}
        onPreviewBranchPatternChange={(v) => setForm({ ...form, preview_branch_pattern: v })}
      />

      <DeployApprovalCard app={app} />

      <TunnelCard app={app} />

      <PersistentStorageCard
        volumes={volumes}
        volumeErrors={volumeErrors}
        onAddVolume={addVolume}
        onRemoveVolume={removeVolume}
        onUpdateVolume={updateVolume}
        onSwitchVolumeType={switchVolumeType}
        onBrowseVolume={setBrowsingVolume}
      />

      {/* Save Button */}
      <div class={styles.saveRow}>
        <Button variant="primary" onClick={handleSave} loading={saving}>
          <Save size={14} /> Save All Settings
        </Button>
        {/* a11y [WCAG 4.1.3]: announce save result to AT */}
        <span role="status" aria-live="polite">{saveMessage}</span>
      </div>

      {/* Server Placement — only in multi-server mode */}
      {multiServer && (
        <ServerPlacementCard
          appName={app.name}
          serverId={app.server_id}
          currentServer={currentServer}
          availableTargets={availableTargets}
          migrateTarget={migrateTarget}
          showMigrateDialog={showMigrateDialog}
          migrateAck={migrateAck}
          migrating={migrating}
          onMigrateTargetChange={setMigrateTarget}
          onShowMigrateDialog={() => setShowMigrateDialog(true)}
          onCloseMigrateDialog={() => setShowMigrateDialog(false)}
          onMigrateAckChange={setMigrateAck}
          onMigrate={handleMigrate}
        />
      )}

      <CanaryProbeCard app={app} />

      <ExportBundleCard app={app} />

      <DangerZoneCard
        confirmDelete={confirmDelete}
        deleting={deleting}
        stopping={stopping}
        starting={starting}
        restarting={restarting}
        onStart={async () => { setStarting(true); try { await api.startApp(app.id); } catch {} setStarting(false); }}
        onRestart={async () => { setRestarting(true); try { await api.restartApp(app.id); } catch {} setRestarting(false); }}
        onStop={async () => { setStopping(true); try { await api.stopApp(app.id); } catch {} setStopping(false); }}
        onDelete={handleDelete}
        onConfirmDeleteToggle={setConfirmDelete}
      />

      {/* Volume Browser Drawer */}
      {browsingVolume !== null && volumes[browsingVolume] && volumes[browsingVolume].type !== 's3' && (
        <VolumeBrowser
          appId={app.id}
          mountIndex={browsingVolume}
          volume={{
            source: (volumes[browsingVolume] as LocalVolume).source,
            target: volumes[browsingVolume].target,
            read_only: volumes[browsingVolume].read_only,
          }}
          onClose={() => setBrowsingVolume(null)}
        />
      )}
    </div>
  );
}
