import { useState, useEffect } from 'preact/hooks';
import type { EnvironmentVariable } from '@lib/types';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Button from '@islands/shared/Button/Button';
import Toggle from '@islands/shared/Toggle/Toggle';
import { Plus, Trash2, EyeOff } from 'lucide-preact';
import Input from '@islands/shared/Input/Input';
import styles from './environment-vars-editor.module.css';

type Props = {
  environmentId: string;
};

export default function EnvironmentVarsEditor({ environmentId }: Props) {
  const [variables, setVariables] = useState<EnvironmentVariable[]>([]);
  const [loading, setLoading] = useState(true);
  const [showAddForm, setShowAddForm] = useState(false);
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');
  const [newIsSecret, setNewIsSecret] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setLoading(true);
    api.listEnvironmentVariables(environmentId)
      .then(({ data }) => {
        setVariables(data);
        setLoading(false);
      })
      .catch(() => {
        addToast('error', 'Failed to load environment variables');
        setLoading(false);
      });
  }, [environmentId]);

  async function handleAdd() {
    if (!newKey.trim() || !newValue.trim()) return;
    setSaving(true);
    try {
      const { data } = await api.setEnvironmentVariable(environmentId, {
        key: newKey.trim(),
        value: newValue,
        is_secret: newIsSecret,
      });
      setVariables((prev) => [...prev, data]);
      setNewKey('');
      setNewValue('');
      setNewIsSecret(false);
      setShowAddForm(false);
      addToast('success', `Variable "${data.key}" added`);
    } catch {
      addToast('error', 'Failed to add variable');
    }
    setSaving(false);
  }

  async function handleDelete(varId: string) {
    const variable = variables.find((v) => v.id === varId);
    try {
      await api.deleteEnvironmentVariable(environmentId, varId);
      setVariables((prev) => prev.filter((v) => v.id !== varId));
      addToast('success', `Variable "${variable?.key}" removed`);
    } catch {
      addToast('error', 'Failed to delete variable');
    }
  }

  if (loading) {
    return (
      <div class={styles.container}>
        <p class={styles.emptyText}>Loading variables...</p>
      </div>
    );
  }

  return (
    <div class={styles.container}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h3 style={{ fontSize: 'var(--text-base)', fontWeight: 'var(--weight-semibold)' }}>
          Environment variables
        </h3>
        <Button variant="secondary" size="sm" onClick={() => setShowAddForm(!showAddForm)}>
          <Plus size={14} aria-hidden="true" /> Add variable
        </Button>
      </div>

      {showAddForm && (
        <form
          class={styles.addForm}
          onSubmit={(e) => {
            e.preventDefault();
            handleAdd();
          }}
        >
          <div class={styles.addFormField}>
            <Input
              label="Key"
              name="env-var-key"
              id="env-var-key"
              mono
              value={newKey}
              onChange={setNewKey}
              placeholder="DATABASE_URL"
              required
            />
          </div>
          <div class={styles.addFormField}>
            <Input
              label="Value"
              name="env-var-value"
              id="env-var-value"
              mono
              value={newValue}
              onChange={setNewValue}
              placeholder="postgres://..."
              required
            />
          </div>
          <Toggle
            label="Secret"
            description="Mask value after saving"
            checked={newIsSecret}
            onChange={setNewIsSecret}
          />
          <Button variant="primary" size="sm" onClick={handleAdd} loading={saving} disabled={!newKey.trim() || !newValue.trim()}>
            Save variable
          </Button>
          <Button variant="ghost" size="sm" onClick={() => setShowAddForm(false)}>
            Cancel
          </Button>
        </form>
      )}

      {variables.length === 0 ? (
        <p class={styles.emptyText}>
          No variables defined. Add variables to share configuration across apps in this environment.
        </p>
      ) : (
        <div class={styles.varList}>
          {variables.map((v) => (
            <div key={v.id} class={styles.varRow}>
              <div class={styles.varInfo}>
                <span class={styles.varKey}>{v.key}</span>
                {v.is_secret ? (
                  <span class={`${styles.varValue} ${styles.masked}`}>
                    <EyeOff size={12} aria-hidden="true" /> ••••••••
                  </span>
                ) : (
                  <span class={styles.varValue}>{v.value}</span>
                )}
              </div>
              <div class={styles.varActions}>
                {v.is_secret && (
                  <span class={styles.secretBadge}>secret</span>
                )}
                <button
                  type="button"
                  class={styles.iconButton}
                  onClick={() => handleDelete(v.id)}
                  aria-label={`Delete variable ${v.key}`}
                >
                  <Trash2 size={14} aria-hidden="true" />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* a11y [WCAG 4.1.3]: announce changes to assistive technology */}
      <div role="status" aria-live="polite" class="sr-only" />
    </div>
  );
}
