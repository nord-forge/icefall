import { useState, useEffect } from 'preact/hooks';
import type { LogDrain } from '@lib/types';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Button from '@islands/shared/Button/Button';
import ConfirmDialog from '@islands/shared/ConfirmDialog/ConfirmDialog';
import Toggle from '@islands/shared/Toggle/Toggle';
import { Plus, Trash2, Database, BarChart, Globe, Zap } from 'lucide-preact';
import Input from '@islands/shared/Input/Input';
import Select from '@islands/shared/Select/Select';
import Textarea from '@islands/shared/Textarea/Textarea';
import formStyles from '@styles/form.module.css';
import styles from './log-drains.module.css';

type Props = {
  appId: string;
};

type DrainType = 'loki' | 'axiom' | 'http';

const DRAIN_TYPE_LABELS: Record<DrainType, string> = {
  loki: 'Loki',
  axiom: 'Axiom',
  http: 'HTTP',
};

function DrainTypeIcon({ type }: { type: DrainType }) {
  switch (type) {
    case 'loki':
      return <Database size={14} aria-hidden="true" />;
    case 'axiom':
      return <BarChart size={14} aria-hidden="true" />;
    case 'http':
      return <Globe size={14} aria-hidden="true" />;
  }
}

function parseConfig(raw: string): Record<string, unknown> {
  try {
    return JSON.parse(raw);
  } catch {
    return {};
  }
}

export default function LogDrains({ appId }: Props) {
  const [drains, setDrains] = useState<LogDrain[]>([]);
  const [loading, setLoading] = useState(true);
  const [showAddForm, setShowAddForm] = useState(false);
  const [newType, setNewType] = useState<DrainType>('loki');
  const [newName, setNewName] = useState('');
  const [newConfig, setNewConfig] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState<string | null>(null);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    api.listLogDrains(appId)
      .then(({ data }) => {
        setDrains(data);
        setLoading(false);
      })
      .catch(() => {
        addToast('error', 'Failed to load log drains');
        setLoading(false);
      });
  }, [appId]);

  function resetForm() {
    setNewName('');
    setNewType('loki');
    setNewConfig({});
    setShowAddForm(false);
  }

  function handleTypeChange(type: DrainType) {
    setNewType(type);
    setNewConfig({});
  }

  function updateConfig(key: string, value: string) {
    setNewConfig((prev) => ({ ...prev, [key]: value }));
  }

  async function handleAdd() {
    if (!newName.trim()) return;
    setSaving(true);
    try {
      const { data } = await api.createLogDrain(appId, {
        name: newName.trim(),
        drain_type: newType,
        config: newConfig,
        enabled: true,
      });
      setDrains((prev) => [...prev, data]);
      resetForm();
      addToast('success', `Log drain "${data.name}" created`);
    } catch {
      addToast('error', 'Failed to create log drain');
    }
    setSaving(false);
  }

  async function handleToggle(drain: LogDrain) {
    try {
      const { data } = await api.updateLogDrain(drain.id, {
        name: drain.name,
        drain_type: drain.drain_type,
        config: parseConfig(drain.config),
        enabled: !drain.enabled,
      });
      setDrains((prev) => prev.map((d) => (d.id === data.id ? data : d)));
      addToast('success', `${data.name} ${data.enabled ? 'enabled' : 'disabled'}`);
    } catch {
      addToast('error', 'Failed to update log drain');
    }
  }

  async function handleTest(drainId: string) {
    setTesting(drainId);
    try {
      const { data } = await api.testLogDrain(drainId);
      if (data.success) {
        addToast('success', 'Connection test passed');
      } else {
        addToast('error', `Connection test failed: ${data.message}`);
      }
    } catch {
      addToast('error', 'Connection test failed');
    }
    setTesting(null);
  }

  async function handleDelete(drainId: string) {
    const drain = drains.find((d) => d.id === drainId);
    try {
      await api.deleteLogDrain(drainId);
      setDrains((prev) => prev.filter((d) => d.id !== drainId));
      setConfirmDeleteId(null);
      addToast('success', `Log drain "${drain?.name}" deleted`);
    } catch {
      addToast('error', 'Failed to delete log drain');
    }
  }

  function formatDate(iso: string | null): string {
    if (!iso) return 'never';
    return new Date(iso).toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function renderConfigFields() {
    switch (newType) {
      case 'loki':
        return (
          <>
            <div class={formStyles.fieldRow}>
              <Input
                label="Loki URL"
                name="drain-loki-url"
                id="drain-loki-url"
                mono
                value={newConfig.url || ''}
                onChange={(v) => updateConfig('url', v)}
                placeholder="https://loki.example.com/loki/api/v1/push"
              />
              <Input
                label="Tenant ID"
                name="drain-loki-tenant"
                id="drain-loki-tenant"
                value={newConfig.tenant_id || ''}
                onChange={(v) => updateConfig('tenant_id', v)}
                placeholder="default"
              />
            </div>
            <div class={formStyles.fieldRow}>
              <Input
                label="Username"
                name="drain-loki-user"
                id="drain-loki-user"
                value={newConfig.username || ''}
                onChange={(v) => updateConfig('username', v)}
              />
              <Input
                label="Password"
                name="drain-loki-pass"
                id="drain-loki-pass"
                type="password"
                value={newConfig.password || ''}
                onChange={(v) => updateConfig('password', v)}
              />
            </div>
          </>
        );

      case 'axiom':
        return (
          <div class={formStyles.fieldRow}>
            <Input
              label="Dataset name"
              name="drain-axiom-dataset"
              id="drain-axiom-dataset"
              value={newConfig.dataset || ''}
              onChange={(v) => updateConfig('dataset', v)}
              placeholder="my-app-logs"
            />
            <Input
              label="API token"
              name="drain-axiom-token"
              id="drain-axiom-token"
              type="password"
              mono
              value={newConfig.api_token || ''}
              onChange={(v) => updateConfig('api_token', v)}
            />
          </div>
        );

      case 'http':
        return (
          <>
            <div class={formStyles.fieldRow}>
              <Input
                label="Endpoint URL"
                name="drain-http-url"
                id="drain-http-url"
                mono
                value={newConfig.url || ''}
                onChange={(v) => updateConfig('url', v)}
                placeholder="https://logs.example.com/ingest"
              />
              <div>
                <label htmlFor="drain-http-method" class={formStyles.label}>Method</label>
                <Select
                  id="drain-http-method"
                  value={newConfig.method || 'POST'}
                  onChange={(v) => updateConfig('method', v)}
                  options={[
                    { value: 'POST', label: 'POST' },
                    { value: 'PUT', label: 'PUT' },
                  ]}
                />
              </div>
            </div>
            <Textarea
              label="Headers (JSON)"
              name="drain-http-headers"
              id="drain-http-headers"
              rows={3}
              value={newConfig.headers || ''}
              onChange={(v) => updateConfig('headers', v)}
              placeholder='{"Authorization": "Bearer ..."}'
              mono
            />
            <div>
              <label htmlFor="drain-http-format" class={formStyles.label}>Format</label>
              <Select
                id="drain-http-format"
                value={newConfig.format || 'json'}
                onChange={(v) => updateConfig('format', v)}
                options={[
                  { value: 'json', label: 'JSON' },
                  { value: 'text', label: 'Plain text' },
                ]}
              />
            </div>
          </>
        );
    }
  }

  if (loading) {
    return (
      <div class={styles.container}>
        <p class={styles.emptyText}>Loading log drains...</p>
      </div>
    );
  }

  return (
    <div class={styles.container}>
      <div class={styles.header}>
        <h2 class={styles.heading}>
          <Zap size={18} aria-hidden="true" /> Log drains
        </h2>
        <Button variant="secondary" onClick={() => setShowAddForm(!showAddForm)}>
          <Plus size={14} aria-hidden="true" /> Add drain
        </Button>
      </div>

      {showAddForm && (
        <div class={styles.addForm}>
          <Input
            label="Name"
            name="drain-name"
            id="drain-name"
            value={newName}
            onChange={setNewName}
            placeholder="Production logs"
          />

          <div>
            <span class={formStyles.label}>Type</span>
            <div class={styles.typeSelector} role="radiogroup" aria-label="Log drain type">
              {(['loki', 'axiom', 'http'] as DrainType[]).map((type) => (
                <button
                  key={type}
                  type="button"
                  role="radio"
                  aria-checked={newType === type}
                  class={`${styles.typeOption} ${newType === type ? styles.typeOptionActive : ''}`}
                  onClick={() => handleTypeChange(type)}
                >
                  <DrainTypeIcon type={type} />
                  {DRAIN_TYPE_LABELS[type]}
                </button>
              ))}
            </div>
          </div>

          {renderConfigFields()}

          <div class={styles.formActions}>
            <Button variant="ghost" onClick={resetForm}>
              Cancel
            </Button>
            <Button variant="primary" onClick={handleAdd} loading={saving} disabled={!newName.trim()}>
              Create drain
            </Button>
          </div>
        </div>
      )}

      {drains.length === 0 && !showAddForm ? (
        <p class={styles.emptyText}>
          No log drains configured. Add a drain to forward logs to an external service.
        </p>
      ) : (
        <div class={styles.drainList}>
          {drains.map((drain) => (
            <div key={drain.id} class={styles.drainRow}>
              <span
                class={`${styles.statusDot} ${drain.enabled ? styles.statusActive : styles.statusInactive}`}
                aria-label={drain.enabled ? 'Active' : 'Inactive'}
              />
              <div class={styles.drainInfo}>
                <span class={styles.drainName}>{drain.name}</span>
                <span class={styles.drainMeta}>
                  Last sent: {formatDate(drain.last_sent_at)}
                </span>
              </div>
              <span class={styles.typeBadge}>
                <DrainTypeIcon type={drain.drain_type} />
                {DRAIN_TYPE_LABELS[drain.drain_type]}
              </span>
              <div class={styles.drainActions}>
                <Toggle
                  label={`${drain.enabled ? 'Disable' : 'Enable'} ${drain.name}`}
                  checked={drain.enabled}
                  onChange={() => handleToggle(drain)}
                />
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => handleTest(drain.id)}
                  loading={testing === drain.id}
                >
                  Test
                </Button>
                <button
                  type="button"
                  class={styles.iconButton}
                  onClick={() => setConfirmDeleteId(drain.id)}
                  aria-label={`Delete ${drain.name} drain`}
                >
                  <Trash2 size={14} aria-hidden="true" />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* a11y [WCAG 4.1.3]: announce changes to AT */}
      <div role="status" aria-live="polite" class="sr-only" />

      <ConfirmDialog
        open={confirmDeleteId !== null}
        title="Delete log drain?"
        description={`This will permanently remove "${drains.find((d) => d.id === confirmDeleteId)?.name ?? 'this drain'}" and stop forwarding logs to it. This action cannot be undone.`}
        confirmLabel="Delete"
        variant="danger"
        loading={deleting}
        onConfirm={async () => {
          if (!confirmDeleteId) return;
          setDeleting(true);
          try {
            await handleDelete(confirmDeleteId);
          } finally {
            setDeleting(false);
            setConfirmDeleteId(null);
          }
        }}
        onCancel={() => setConfirmDeleteId(null)}
      />
    </div>
  );
}
