import { useState, useEffect } from 'preact/hooks';
import type { ProjectEnvironment } from '@lib/types';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Button from '@islands/shared/Button/Button';
import { Plus, Trash2 } from 'lucide-preact';
import Input from '@islands/shared/Input/Input';
import formStyles from '@styles/form.module.css';
import styles from './environment-tabs.module.css';

type Props = {
  projectId: string;
  onFilterChange: (envId: string | null) => void;
};

export default function EnvironmentTabs({ projectId, onFilterChange }: Props) {
  const [environments, setEnvironments] = useState<ProjectEnvironment[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [showAddForm, setShowAddForm] = useState(false);
  const [newName, setNewName] = useState('');
  const [newColor, setNewColor] = useState('#3b82f6');
  const [adding, setAdding] = useState(false);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);

  useEffect(() => {
    api.listProjectEnvironments(projectId)
      .then(({ data }) => setEnvironments(data))
      .catch(() => addToast('error', 'Failed to load environments'));
  }, [projectId]);

  function selectTab(envId: string | null) {
    setActiveId(envId);
    onFilterChange(envId);
  }

  async function handleAdd() {
    if (!newName.trim()) return;
    setAdding(true);
    try {
      const { data } = await api.createProjectEnvironment(projectId, {
        name: newName.trim(),
        color: newColor,
      });
      setEnvironments((prev) => [...prev, data]);
      setNewName('');
      setNewColor('#3b82f6');
      setShowAddForm(false);
      addToast('success', `Environment "${data.name}" created`);
    } catch {
      addToast('error', 'Failed to create environment');
    }
    setAdding(false);
  }

  async function handleDelete(envId: string) {
    const env = environments.find((e) => e.id === envId);
    try {
      await api.deleteProjectEnvironment(projectId, envId);
      setEnvironments((prev) => prev.filter((e) => e.id !== envId));
      if (activeId === envId) {
        setActiveId(null);
        onFilterChange(null);
      }
      setConfirmDeleteId(null);
      addToast('success', `Environment "${env?.name}" deleted`);
    } catch {
      addToast('error', 'Failed to delete environment');
    }
  }

  return (
    <div>
      <div class={styles.tabBar} role="tablist" aria-label="Environment filters">
        <button
          type="button"
          role="tab"
          aria-selected={activeId === null}
          class={`${styles.tab} ${activeId === null ? styles.tabActive : ''}`}
          onClick={() => selectTab(null)}
        >
          All
        </button>

        {environments.map((env) => (
          <div key={env.id} class={styles.envRow}>
            <button
              type="button"
              role="tab"
              aria-selected={activeId === env.id}
              class={`${styles.tab} ${activeId === env.id ? styles.tabActive : ''}`}
              onClick={() => selectTab(env.id)}
            >
              {env.color && (
                <span
                  class={styles.colorDot}
                  style={{ backgroundColor: env.color }}
                  aria-hidden="true"
                />
              )}
              {env.name}
            </button>

            {confirmDeleteId === env.id ? (
              <>
                <Button
                  variant="danger"
                  size="sm"
                  onClick={() => handleDelete(env.id)}
                >
                  Confirm
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setConfirmDeleteId(null)}
                >
                  Cancel
                </Button>
              </>
            ) : (
              <button
                type="button"
                class={styles.deleteButton}
                onClick={() => setConfirmDeleteId(env.id)}
                aria-label={`Delete ${env.name} environment`}
              >
                <Trash2 size={14} aria-hidden="true" />
              </button>
            )}
          </div>
        ))}

        <Button
          variant="ghost"
          size="sm"
          onClick={() => setShowAddForm(!showAddForm)}
        >
          <Plus size={14} aria-hidden="true" /> Add environment
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
              label="Name"
              name="new-env-name"
              id="new-env-name"
              value={newName}
              onChange={setNewName}
              placeholder="staging"
              required
            />
          </div>

          <div class={styles.addFormField}>
            <label htmlFor="new-env-color" class={formStyles.label}>
              Color
            </label>
            {/* a11y [WCAG 1.4.1]: color is decorative, not sole indicator */}
            <input
              id="new-env-color"
              type="color"
              class={styles.colorPicker}
              value={newColor}
              onInput={(e) => setNewColor((e.target as HTMLInputElement).value)}
            />
          </div>

          <Button variant="primary" size="sm" onClick={handleAdd} loading={adding} disabled={!newName.trim()}>
            Create
          </Button>
          <Button variant="ghost" size="sm" onClick={() => setShowAddForm(false)}>
            Cancel
          </Button>
        </form>
      )}

      {/* a11y [WCAG 4.1.3]: announce filter changes to assistive technology */}
      <div role="status" aria-live="polite" class="sr-only">
        {activeId
          ? `Showing apps in ${environments.find((e) => e.id === activeId)?.name} environment`
          : 'Showing all apps'}
      </div>
    </div>
  );
}
