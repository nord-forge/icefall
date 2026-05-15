import { useState, useEffect } from 'preact/hooks';
import type { LogDrain } from '@lib/types';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import Textarea from '@islands/shared/Textarea/Textarea';
import Toggle from '@islands/shared/Toggle/Toggle';
import { Plus, Trash2, Database, BarChart, Globe, Zap } from 'lucide-preact';
import formStyles from '@styles/form.module.css';
import styles from '../settings-page.module.css';

type Props = {
  onSaveMessage: (msg: string) => void;
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

function formatDate(iso: string | null): string {
  if (!iso) return 'never';
  return new Date(iso).toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export default function GlobalLogDrainsSection({ onSaveMessage }: Props) {
  const [drains, setDrains] = useState<LogDrain[]>([]);
  const [loading, setLoading] = useState(true);
  const [showAddForm, setShowAddForm] = useState(false);
  const [newType, setNewType] = useState<DrainType>('loki');
  const [newName, setNewName] = useState('');
  const [newConfig, setNewConfig] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState<string | null>(null);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);

  useEffect(() => {
    api.listGlobalLogDrains()
      .then(({ data }) => {
        setDrains(data);
        setLoading(false);
      })
      .catch(() => {
        addToast('error', 'Failed to load global log drains');
        setLoading(false);
      });
  }, []);

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
      // Global drains use the same endpoint but with a placeholder app_id handled server-side
      const { data } = await api.createLogDrain('_global', {
        name: newName.trim(),
        drain_type: newType,
        config: newConfig,
        enabled: true,
      });
      setDrains((prev) => [...prev, data]);
      resetForm();
      onSaveMessage(`Global log drain "${data.name}" created`);
      addToast('success', `Global log drain "${data.name}" created`);
    } catch {
      addToast('error', 'Failed to create global log drain');
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
      onSaveMessage(`${data.name} ${data.enabled ? 'enabled' : 'disabled'}`);
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
      onSaveMessage(`Global log drain "${drain?.name}" deleted`);
      addToast('success', `Global log drain "${drain?.name}" deleted`);
    } catch {
      addToast('error', 'Failed to delete log drain');
    }
  }

  function renderConfigFields() {
    switch (newType) {
      case 'loki':
        return (
          <>
            <div class={formStyles.fieldRow}>
              <Input
                label="Loki URL"
                name="gld-loki-url"
                id="gld-loki-url"
                mono
                value={newConfig.url || ''}
                onChange={(v) => updateConfig('url', v)}
                placeholder="https://loki.example.com/loki/api/v1/push"
              />
              <Input
                label="Tenant ID"
                name="gld-loki-tenant"
                id="gld-loki-tenant"
                value={newConfig.tenant_id || ''}
                onChange={(v) => updateConfig('tenant_id', v)}
                placeholder="default"
              />
            </div>
            <div class={formStyles.fieldRow}>
              <Input
                label="Username"
                name="gld-loki-user"
                id="gld-loki-user"
                value={newConfig.username || ''}
                onChange={(v) => updateConfig('username', v)}
              />
              <Input
                label="Password"
                name="gld-loki-pass"
                id="gld-loki-pass"
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
              name="gld-axiom-dataset"
              id="gld-axiom-dataset"
              value={newConfig.dataset || ''}
              onChange={(v) => updateConfig('dataset', v)}
              placeholder="my-app-logs"
            />
            <Input
              label="API token"
              name="gld-axiom-token"
              id="gld-axiom-token"
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
                name="gld-http-url"
                id="gld-http-url"
                mono
                value={newConfig.url || ''}
                onChange={(v) => updateConfig('url', v)}
                placeholder="https://logs.example.com/ingest"
              />
              <div>
                <label htmlFor="gld-http-method" class={formStyles.label}>Method</label>
                <select
                  id="gld-http-method"
                  class={formStyles.select}
                  value={newConfig.method || 'POST'}
                  onChange={(e) => updateConfig('method', (e.target as HTMLSelectElement).value)}
                >
                  <option value="POST">POST</option>
                  <option value="PUT">PUT</option>
                </select>
              </div>
            </div>
            <Textarea
              label="Headers (JSON)"
              name="gld-http-headers"
              id="gld-http-headers"
              rows={3}
              value={newConfig.headers || ''}
              onChange={(v) => updateConfig('headers', v)}
              placeholder='{"Authorization": "Bearer ..."}'
            />
            <div>
              <label htmlFor="gld-http-format" class={formStyles.label}>Format</label>
              <select
                id="gld-http-format"
                class={formStyles.select}
                value={newConfig.format || 'json'}
                onChange={(e) => updateConfig('format', (e.target as HTMLSelectElement).value)}
              >
                <option value="json">JSON</option>
                <option value="text">Plain text</option>
              </select>
            </div>
          </>
        );
    }
  }

  return (
    <div class={styles.section}>
      <div class={styles.sectionHeaderRow}>
        <h2 class={styles.sectionHeading}>
          <Zap size={18} aria-hidden="true" /> Global Log Drains
        </h2>
        <Button variant="secondary" onClick={() => setShowAddForm(!showAddForm)}>
          <Plus size={14} aria-hidden="true" /> Add drain
        </Button>
      </div>

      {showAddForm && (
        <div class={styles.addCard}>
          <Input
            label="Name"
            name="gld-name"
            id="gld-name"
            value={newName}
            onChange={setNewName}
            placeholder="All-app centralized logging"
          />

          <div>
            <span class={formStyles.label}>Type</span>
            <div style={{ display: 'flex', gap: 'var(--space-2)', flexWrap: 'wrap' }} role="radiogroup" aria-label="Log drain type">
              {(['loki', 'axiom', 'http'] as DrainType[]).map((type) => {
                const isActive = newType === type;
                return (
                  <button
                    key={type}
                    type="button"
                    role="radio"
                    aria-checked={isActive}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: 'var(--space-2)',
                      padding: 'var(--space-2) var(--space-4)',
                      border: `1px solid ${isActive ? 'var(--color-primary)' : 'var(--color-border)'}`,
                      borderRadius: 'var(--radius-sm)',
                      background: isActive ? 'oklch(from var(--color-primary) l c h / 0.08)' : 'var(--color-surface)',
                      color: 'var(--color-text)',
                      fontSize: 'var(--text-sm)',
                      fontWeight: 'var(--weight-medium)',
                      cursor: 'pointer',
                    }}
                    onClick={() => handleTypeChange(type)}
                  >
                    <DrainTypeIcon type={type} />
                    {DRAIN_TYPE_LABELS[type]}
                  </button>
                );
              })}
            </div>
          </div>

          {renderConfigFields()}

          <div class={styles.addCardActions}>
            <Button variant="ghost" onClick={resetForm}>
              Cancel
            </Button>
            <Button variant="primary" onClick={handleAdd} loading={saving} disabled={!newName.trim()}>
              Create drain
            </Button>
          </div>
        </div>
      )}

      {loading ? (
        <p class={styles.emptyText}>Loading global log drains...</p>
      ) : drains.length === 0 && !showAddForm ? (
        <p class={styles.emptyText}>
          No global log drains configured. Global drains forward logs from all apps to a central destination.
        </p>
      ) : (
        <div class={styles.itemList}>
          {drains.map((drain) => (
            <div key={drain.id} class={styles.itemRow}>
              <div class={styles.itemInfo}>
                <span class={styles.itemLabel}>
                  <span
                    style={{
                      display: 'inline-block',
                      width: '8px',
                      height: '8px',
                      borderRadius: '50%',
                      marginRight: 'var(--space-2)',
                      background: drain.enabled ? 'var(--color-success)' : 'var(--color-text-muted)',
                    }}
                    aria-label={drain.enabled ? 'Active' : 'Inactive'}
                  />
                  {drain.name}
                </span>
                <span class={styles.itemMeta}>
                  {DRAIN_TYPE_LABELS[drain.drain_type]} · Last sent: {formatDate(drain.last_sent_at)}
                </span>
              </div>
              <div class={styles.itemActions}>
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
                {confirmDeleteId === drain.id ? (
                  <>
                    <Button variant="danger" size="sm" onClick={() => handleDelete(drain.id)}>
                      Confirm
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => setConfirmDeleteId(null)}>
                      Cancel
                    </Button>
                  </>
                ) : (
                  <button
                    type="button"
                    class={styles.iconButton}
                    onClick={() => setConfirmDeleteId(drain.id)}
                    aria-label={`Delete ${drain.name} drain`}
                  >
                    <Trash2 size={14} aria-hidden="true" />
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
