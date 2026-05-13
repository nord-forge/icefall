import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import type { EnvVar } from '@lib/types';
import Button from '@islands/shared/Button/Button';
import Select from '@islands/shared/Select/Select';
import { Plus, Eye, EyeOff, Trash2, Upload, Save } from 'lucide-preact';
import styles from './env-var-editor.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  appId: string;
}

export default function EnvVarEditor({ appId }: Props) {
  const [vars, setVars] = useState<EnvVar[]>([]);
  const [loading, setLoading] = useState(true);
  const [revealed, setRevealed] = useState<Set<string>>(new Set());
  const [revealedValues, setRevealedValues] = useState<Record<string, string>>({});
  const [adding, setAdding] = useState(false);
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');
  const [newScope, setNewScope] = useState('shared');
  const [showImport, setShowImport] = useState(false);
  const [importContent, setImportContent] = useState('');

  useEffect(() => {
    api.listEnvVars(appId).then(({ data }) => { setVars(data); setLoading(false); }).catch(() => setLoading(false));
  }, [appId]);

  async function handleReveal(varId: string) {
    if (revealed.has(varId)) {
      setRevealed((prev) => { const s = new Set(prev); s.delete(varId); return s; });
      return;
    }
    const { data } = await api.listEnvVars(appId, true);
    const found = data.find((v) => v.id === varId);
    if (found) {
      setRevealedValues((prev) => ({ ...prev, [varId]: found.value }));
      setRevealed((prev) => new Set(prev).add(varId));
    }
  }

  async function handleAdd() {
    if (!newKey.trim()) return;
    try {
      await api.setEnvVar(appId, { key: newKey.trim(), value: newValue, scope: newScope });
      const { data } = await api.listEnvVars(appId);
      setVars(data);
      setNewKey('');
      setNewValue('');
      setAdding(false);
    } catch { /* show error */ }
  }

  async function handleDelete(varId: string) {
    await api.deleteEnvVar(appId, varId);
    setVars((prev) => prev.filter((v) => v.id !== varId));
  }

  async function handleImport() {
    if (!importContent.trim()) return;
    await api.importEnv(appId, importContent, 'shared');
    const { data } = await api.listEnvVars(appId);
    setVars(data);
    setShowImport(false);
    setImportContent('');
  }

  if (loading) return <p class={styles.loadingText}>Loading environment variables...</p>;

  return (
    <div>
      <div class={styles.header}>
        <p class={styles.description}>
          Manage environment variables for this application.
        </p>
        <div class={styles.headerActions}>
          <Button variant="secondary" onClick={() => setShowImport(true)}>
            <Upload size={14} /> Import .env
          </Button>
          <Button variant="primary" onClick={() => setAdding(true)}>
            <Plus size={14} /> Add Variable
          </Button>
        </div>
      </div>

      {showImport && (
        <div class={styles.importCard}>
          <h4 class={styles.importTitle}>Import .env file</h4>
          <label htmlFor="import-env-content" class={formStyles.label}>File content</label>
          <textarea
            id="import-env-content"
            value={importContent}
            onInput={(e) => setImportContent((e.target as HTMLTextAreaElement).value)}
            placeholder="KEY=value&#10;ANOTHER_KEY=another_value"
            rows={8}
            class={formStyles.textarea}
          />
          <div class={styles.importActions}>
            <Button variant="ghost" onClick={() => setShowImport(false)}>Cancel</Button>
            <Button variant="primary" onClick={handleImport}>Import</Button>
          </div>
        </div>
      )}

      <div class={styles.tableCard}>
        <table class={styles.table}>
          <thead>
            <tr class={styles.tableRow}>
              {['Key', 'Value', 'Scope', 'Actions'].map((h) => (
                <th key={h} class={styles.th}>
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {vars.map((v) => (
              <tr key={v.id} class={styles.tableRow}>
                <td class={`${styles.td} ${styles.keyCell}`}>
                  {v.key}
                </td>
                <td class={`${styles.td} ${styles.valueCell}`}>
                  {revealed.has(v.id) ? revealedValues[v.id] || v.value : '••••••••••••••••'}
                </td>
                <td class={styles.td}>
                  <span class={v.scope === 'production' ? styles.scopeProduction : styles.scopeShared}>
                    {v.scope === 'shared' ? 'All' : v.scope.charAt(0).toUpperCase() + v.scope.slice(1)}
                  </span>
                </td>
                <td class={styles.tdActions}>
                  {/* a11y [4.1.2]: aria-label on icon-only button */}
                  <button onClick={() => handleReveal(v.id)} class={styles.iconButton} aria-label={revealed.has(v.id) ? 'Hide value' : 'Show value'}>
                    {revealed.has(v.id) ? <EyeOff size={14} /> : <Eye size={14} />}
                  </button>
                  {/* a11y [4.1.2]: aria-label on icon-only button */}
                  <button onClick={() => handleDelete(v.id)} class={styles.iconButton} aria-label="Delete variable">
                    <Trash2 size={14} />
                  </button>
                </td>
              </tr>
            ))}
            {adding && (
              <tr>
                <td class={styles.tdAdd}>
                  <label htmlFor="new-env-key" class="sr-only">Key</label>
                  <input id="new-env-key" class={formStyles.inputMono} placeholder="KEY" value={newKey} onInput={(e) => setNewKey((e.target as HTMLInputElement).value)} />
                </td>
                <td class={styles.tdAdd}>
                  <label htmlFor="new-env-value" class="sr-only">Value</label>
                  <input id="new-env-value" class={formStyles.inputMono} placeholder="Value" value={newValue} onInput={(e) => setNewValue((e.target as HTMLInputElement).value)} />
                </td>
                <td class={styles.tdAdd}>
                  <label htmlFor="new-env-scope" class="sr-only">Scope</label>
                  <Select
                    id="new-env-scope"
                    options={[
                      { value: 'shared', label: 'All' },
                      { value: 'production', label: 'Production' },
                      { value: 'preview', label: 'Preview' },
                    ]}
                    value={newScope}
                    onChange={setNewScope}
                    size="sm"
                  />
                </td>
                <td class={styles.tdAddActions}>
                  <Button variant="ghost" size="sm" onClick={() => setAdding(false)}>Cancel</Button>
                  <Button variant="primary" size="sm" onClick={handleAdd}>Add</Button>
                </td>
              </tr>
            )}
          </tbody>
        </table>
        {vars.length === 0 && !adding && (
          <div class={styles.emptyState}>
            No environment variables yet.
          </div>
        )}
      </div>

      <p class={styles.footnote}>
        Variables marked as secret are encrypted at rest.
      </p>
    </div>
  );
}
