import { useEffect, useState } from 'preact/hooks';
import { api, request } from '@lib/api';
import { addToast } from '@stores/toast';
import Card from '@islands/shared/Card/Card';
import Input from '@islands/shared/Input/Input';
import Toggle from '@islands/shared/Toggle/Toggle';
import Button from '@islands/shared/Button/Button';
import Tabs from '@islands/shared/Tabs/Tabs';
import styles from './shared-variables-page.module.css';

type SharedVar = {
  id: string;
  scope: string;
  scope_id: string;
  key: string;
  value: string;
  is_sensitive: boolean;
  created_at: string;
};

type Project = { id: string; name: string };
type Server = { id: string; name: string };

export default function SharedVariablesPage() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [servers, setServers] = useState<Server[]>([]);
  const [selectedScope, setSelectedScope] = useState<'project' | 'server'>('project');
  const [selectedId, setSelectedId] = useState('');
  const [vars, setVars] = useState<SharedVar[]>([]);
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');
  const [isSensitive, setIsSensitive] = useState(false);

  useEffect(() => {
    request<{ data: Project[] }>('/projects').then(({ data }) => {
      setProjects(data);
      if (data.length > 0 && !selectedId) setSelectedId(data[0].id);
    }).catch(() => {});
    request<{ data: Server[] }>('/servers').then(({ data }) => setServers(data)).catch(() => {});
  }, []);

  useEffect(() => {
    if (!selectedId) return;
    request<{ data: SharedVar[] }>(`/shared-variables/${selectedScope}/${selectedId}`)
      .then(({ data }) => setVars(data))
      .catch(() => setVars([]));
  }, [selectedScope, selectedId]);

  async function handleAdd() {
    if (!newKey.trim() || !selectedId) return;
    try {
      await request(`/shared-variables/${selectedScope}/${selectedId}`, {
        method: 'POST',
        body: JSON.stringify({ key: newKey, value: newValue, is_sensitive: isSensitive }),
      });
      setNewKey('');
      setNewValue('');
      const { data } = await request<{ data: SharedVar[] }>(`/shared-variables/${selectedScope}/${selectedId}`);
      setVars(data);
      addToast('success', 'Variable added');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to add variable');
    }
  }

  async function handleDelete(varId: string) {
    try {
      await request(`/shared-variables/${varId}`, { method: 'DELETE' });
      setVars(vars.filter((v) => v.id !== varId));
      addToast('info', 'Variable removed');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to delete');
    }
  }

  const scopeItems = selectedScope === 'project' ? projects : servers;

  return (
    <div class={styles.page}>
      <h1 class={styles.title}>Shared variables</h1>

      <Tabs
        tabs={[
          { id: 'project', label: 'Project scope', content: null },
          { id: 'server', label: 'Server scope', content: null },
        ]}
        defaultTab={selectedScope}
      />

      <div class={styles.controls}>
        <div class={styles.scopeSelector}>
          <label class={styles.label}>
            {selectedScope === 'project' ? 'Project' : 'Server'}
          </label>
          <select
            class={styles.select}
            value={selectedId}
            onChange={(e) => setSelectedId((e.target as HTMLSelectElement).value)}
            aria-label={`Select ${selectedScope}`}
          >
            {scopeItems.map((item) => (
              <option key={item.id} value={item.id}>{item.name}</option>
            ))}
          </select>
        </div>
      </div>

      <Card title={`Variables (${vars.length})`}>
        {vars.length === 0 ? (
          <p class={styles.empty}>No shared variables for this {selectedScope}.</p>
        ) : (
          <div class={styles.varList}>
            {vars.map((v) => (
              <div key={v.id} class={styles.varRow}>
                <span class={styles.varKey}>{v.key}</span>
                <span class={styles.varValue}>{v.is_sensitive ? '••••••••' : v.value}</span>
                <button
                  type="button" class={styles.deleteBtn}
                  onClick={() => handleDelete(v.id)}
                  aria-label={`Delete ${v.key}`}
                >
                  Remove
                </button>
              </div>
            ))}
          </div>
        )}

        <div class={styles.addForm}>
          <Input label="Key" name="new-key" value={newKey} onChange={setNewKey} placeholder="VARIABLE_NAME" />
          <Input label="Value" name="new-value" value={newValue} onChange={setNewValue} placeholder="value" revealable={isSensitive} type={isSensitive ? 'password' : 'text'} />
          <Toggle label="Sensitive" checked={isSensitive} onChange={setIsSensitive} description="Mask value in UI" />
          <Button variant="primary" onClick={handleAdd}>Add variable</Button>
        </div>
      </Card>
    </div>
  );
}
