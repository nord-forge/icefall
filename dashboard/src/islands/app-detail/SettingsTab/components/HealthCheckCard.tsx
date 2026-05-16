import { useState, useEffect } from 'preact/hooks';
import type { HealthCheckResult } from '@lib/types';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import { Activity, Trash2, Plus } from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import Toggle from '@islands/shared/Toggle/Toggle';
import Select from '@islands/shared/Select/Select';
import ConfirmDialog from '@islands/shared/ConfirmDialog/ConfirmDialog';
import styles from '../settings-tab.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  appId: string;
};

const CHECK_TYPE_OPTIONS = [
  { value: 'http', label: 'HTTP' },
  { value: 'tcp', label: 'TCP' },
  { value: 'exec', label: 'Exec (command)' },
];

function statusLabel(status: string) {
  switch (status) {
    case 'healthy': return 'Healthy';
    case 'unhealthy': return 'Unhealthy';
    default: return status || 'Unknown';
  }
}

export default function HealthCheckCard({ appId }: Props) {
  const [results, setResults] = useState<HealthCheckResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editForm, setEditForm] = useState({ interval_secs: '', failure_threshold: '', auto_restart: false });
  const [savingEdit, setSavingEdit] = useState(false);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [showCreate, setShowCreate] = useState(false);
  const [createForm, setCreateForm] = useState({
    check_type: 'http',
    interval_secs: '30',
    failure_threshold: '3',
    auto_restart: true,
    config: '',
  });
  const [creating, setCreating] = useState(false);

  async function fetchChecks() {
    try {
      const { data } = await api.getHealth(appId);
      setResults(data);
    } catch {
      addToast('error', 'Failed to load health checks');
    }
    setLoading(false);
  }

  useEffect(() => { fetchChecks(); }, [appId]);

  function startEdit(r: HealthCheckResult) {
    setEditingId(r.check.id);
    setEditForm({
      interval_secs: String(r.check.interval_secs),
      failure_threshold: String(r.check.failure_threshold),
      auto_restart: r.check.auto_restart,
    });
  }

  async function saveEdit(checkId: string) {
    setSavingEdit(true);
    try {
      await api.updateHealth(appId, {
        interval_secs: parseInt(editForm.interval_secs, 10) || 30,
        failure_threshold: parseInt(editForm.failure_threshold, 10) || 3,
        auto_restart: editForm.auto_restart,
      });
      addToast('success', 'Health check updated');
      setEditingId(null);
      await fetchChecks();
    } catch (err: any) {
      addToast('error', err.message || 'Failed to update health check');
    }
    setSavingEdit(false);
  }

  async function handleDelete() {
    if (!deleteId) return;
    setDeleting(true);
    try {
      await api.deleteHealthCheck(appId, deleteId);
      setResults((prev) => prev.filter((r) => r.check.id !== deleteId));
      addToast('success', 'Health check deleted');
      setDeleteId(null);
    } catch (err: any) {
      addToast('error', err.message || 'Failed to delete health check');
    }
    setDeleting(false);
  }

  async function handleCreate() {
    setCreating(true);
    try {
      await api.createHealthCheck(appId, {
        check_type: createForm.check_type,
        interval_secs: parseInt(createForm.interval_secs, 10) || 30,
        failure_threshold: parseInt(createForm.failure_threshold, 10) || 3,
        auto_restart: createForm.auto_restart,
        config: createForm.config || undefined,
      });
      addToast('success', 'Health check created');
      setShowCreate(false);
      setCreateForm({ check_type: 'http', interval_secs: '30', failure_threshold: '3', auto_restart: true, config: '' });
      await fetchChecks();
    } catch (err: any) {
      addToast('error', err.message || 'Failed to create health check');
    }
    setCreating(false);
  }

  return (
    <div class={styles.card}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 'var(--space-4)' }}>
        <h2 class={styles.sectionTitle} style={{ marginBottom: 0 }}>
          <Activity size={18} aria-hidden="true" /> Health Checks
        </h2>
        <Button variant="secondary" onClick={() => setShowCreate(true)}>
          <Plus size={14} aria-hidden="true" /> Add health check
        </Button>
      </div>

      <p class={styles.settingsDescription}>
        Configure health checks to monitor your application. Failed checks can automatically restart the container.
      </p>

      {loading ? (
        <p class={styles.settingsNote}>Loading health checks...</p>
      ) : results.length === 0 && !showCreate ? (
        <p class={styles.settingsNote}>No health checks configured. Add one to monitor your application.</p>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-3)' }}>
          {results.map((r) => (
            <div
              key={r.check.id}
              style={{
                padding: 'var(--space-4)',
                border: '1px solid var(--color-border)',
                borderRadius: 'var(--radius-sm)',
                background: 'var(--color-surface-raised)',
              }}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: 'var(--space-3)' }}>
                <div>
                  <div style={{ fontSize: 'var(--text-sm)', fontWeight: 'var(--weight-medium)', marginBottom: 'var(--space-1)' }}>
                    {r.check.check_type.toUpperCase()} check
                  </div>
                  <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-secondary)' }}>
                    Every {r.check.interval_secs}s
                    {' · '}
                    Threshold: {r.check.failure_threshold} failures
                    {' · '}
                    Auto-restart: {r.check.auto_restart ? 'on' : 'off'}
                    {' · '}
                    Status: {statusLabel(r.current_status)}
                    {r.uptime_percent > 0 && ` · Uptime: ${r.uptime_percent.toFixed(1)}%`}
                  </div>
                </div>
                <div style={{ display: 'flex', gap: 'var(--space-2)', flexShrink: 0 }}>
                  <Button variant="ghost" size="sm" onClick={() => startEdit(r)}>
                    Edit
                  </Button>
                  <button
                    type="button"
                    class={styles.volumeRemove}
                    onClick={() => setDeleteId(r.check.id)}
                    aria-label={`Delete ${r.check.check_type} health check`}
                  >
                    <Trash2 size={14} aria-hidden="true" />
                  </button>
                </div>
              </div>

              {editingId === r.check.id && (
                <div style={{ marginTop: 'var(--space-4)', borderTop: '1px solid var(--color-border)', paddingTop: 'var(--space-4)' }}>
                  <div class={formStyles.fieldRow}>
                    <Input
                      label="Interval (seconds)"
                      name={`edit-interval-${r.check.id}`}
                      id={`edit-interval-${r.check.id}`}
                      type="number"
                      min={5}
                      value={editForm.interval_secs}
                      onChange={(v) => setEditForm((f) => ({ ...f, interval_secs: v }))}
                    />
                    <Input
                      label="Failure threshold"
                      name={`edit-threshold-${r.check.id}`}
                      id={`edit-threshold-${r.check.id}`}
                      type="number"
                      min={1}
                      value={editForm.failure_threshold}
                      onChange={(v) => setEditForm((f) => ({ ...f, failure_threshold: v }))}
                    />
                  </div>
                  <div style={{ marginTop: 'var(--space-3)' }}>
                    <Toggle
                      label="Auto-restart on failure"
                      checked={editForm.auto_restart}
                      onChange={(v) => setEditForm((f) => ({ ...f, auto_restart: v }))}
                    />
                  </div>
                  <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 'var(--space-2)', marginTop: 'var(--space-3)' }}>
                    <Button variant="ghost" onClick={() => setEditingId(null)}>Cancel</Button>
                    <Button variant="primary" onClick={() => saveEdit(r.check.id)} loading={savingEdit}>
                      Save changes
                    </Button>
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {showCreate && (
        <div
          style={{
            marginTop: 'var(--space-4)',
            padding: 'var(--space-4)',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-sm)',
            background: 'var(--color-surface-raised)',
          }}
        >
          <div class={formStyles.fieldRow}>
            <div>
              <label htmlFor="hc-check-type" class={formStyles.label}>Check type</label>
              <Select
                id="hc-check-type"
                options={CHECK_TYPE_OPTIONS}
                value={createForm.check_type}
                onChange={(v) => setCreateForm((f) => ({ ...f, check_type: v }))}
                fullWidth
              />
            </div>
            <Input
              label="Interval (seconds)"
              name="hc-interval"
              id="hc-interval"
              type="number"
              min={5}
              value={createForm.interval_secs}
              onChange={(v) => setCreateForm((f) => ({ ...f, interval_secs: v }))}
              helpText="How often to run the check."
            />
          </div>
          <div class={formStyles.fieldRow} style={{ marginTop: 'var(--space-3)' }}>
            <Input
              label="Failure threshold"
              name="hc-threshold"
              id="hc-threshold"
              type="number"
              min={1}
              value={createForm.failure_threshold}
              onChange={(v) => setCreateForm((f) => ({ ...f, failure_threshold: v }))}
              helpText="Consecutive failures before marking unhealthy."
            />
            <Input
              label="Config (JSON)"
              name="hc-config"
              id="hc-config"
              type="text"
              value={createForm.config}
              onChange={(v) => setCreateForm((f) => ({ ...f, config: v }))}
              placeholder='{"path": "/health", "port": 8080}'
              helpText="Optional JSON config for the check type."
            />
          </div>
          <div style={{ marginTop: 'var(--space-3)' }}>
            <Toggle
              label="Auto-restart on failure"
              checked={createForm.auto_restart}
              onChange={(v) => setCreateForm((f) => ({ ...f, auto_restart: v }))}
            />
          </div>
          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 'var(--space-2)', marginTop: 'var(--space-4)' }}>
            <Button variant="ghost" onClick={() => setShowCreate(false)}>Cancel</Button>
            <Button variant="primary" onClick={handleCreate} loading={creating}>
              Create health check
            </Button>
          </div>
        </div>
      )}

      <ConfirmDialog
        open={deleteId !== null}
        title="Delete health check?"
        description="This will permanently remove this health check. The application will no longer be monitored by this check."
        confirmLabel="Delete"
        variant="danger"
        loading={deleting}
        onConfirm={handleDelete}
        onCancel={() => setDeleteId(null)}
      />
    </div>
  );
}
