import { useState, useEffect } from 'preact/hooks';
import type { App, ProjectEnvironment } from '@lib/types';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Select from '@islands/shared/Select/Select';
import { Layers } from 'lucide-preact';
import styles from '../settings-tab.module.css';

type Props = {
  app: App;
  projectId: string | null;
};

export default function EnvironmentAssignmentCard({ app, projectId }: Props) {
  const [environments, setEnvironments] = useState<ProjectEnvironment[]>([]);
  const [selectedEnvId, setSelectedEnvId] = useState<string>(app.project_environment_id || '');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!projectId) return;
    api.listProjectEnvironments(projectId)
      .then(({ data }) => setEnvironments(data))
      .catch(() => {});
  }, [projectId]);

  if (!projectId) return null;

  async function handleChange(envId: string) {
    setSelectedEnvId(envId);
    setSaving(true);
    try {
      await api.updateApp(app.id, {
        project_environment_id: envId || null,
      } as any);
      const envName = environments.find((e) => e.id === envId)?.name;
      addToast('success', envId ? `Assigned to ${envName} environment` : 'Environment assignment removed');
    } catch {
      addToast('error', 'Failed to update environment assignment');
      setSelectedEnvId(app.project_environment_id || '');
    }
    setSaving(false);
  }

  return (
    <div class={styles.card}>
      <h2 class={styles.sectionTitle}>
        <Layers size={18} aria-hidden="true" /> Environment
      </h2>
      <p class={styles.settingsDescription}>
        Assign this app to a project environment to inherit shared variables and organize deployments.
      </p>
      {/* a11y [WCAG 4.1.2]: select has associated label via id */}
      <label htmlFor="env-assignment-select" class="sr-only">
        Project environment
      </label>
      <Select
        id="env-assignment-select"
        options={[
          { value: '', label: 'No environment' },
          ...environments.map((e) => ({
            value: e.id,
            label: e.name,
          })),
        ]}
        value={selectedEnvId}
        onChange={handleChange}
        disabled={saving}
        fullWidth
      />
      <span class={styles.fieldHint}>
        {selectedEnvId
          ? `Variables from this environment will be available at deploy time.`
          : 'Choose an environment to share configuration across apps.'}
      </span>
      {/* a11y [WCAG 4.1.3]: announce save result to AT */}
      <span role="status" aria-live="polite" class="sr-only">
        {saving ? 'Saving environment assignment...' : ''}
      </span>
    </div>
  );
}
