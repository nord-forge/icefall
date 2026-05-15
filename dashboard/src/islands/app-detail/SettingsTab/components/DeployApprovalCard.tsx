import { useState } from 'preact/hooks';
import type { App } from '@lib/types';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Toggle from '@islands/shared/Toggle/Toggle';
import styles from '../settings-tab.module.css';

type Props = {
  app: App;
};

export default function DeployApprovalCard({ app }: Props) {
  const [enabled, setEnabled] = useState(app.require_deploy_approval);
  const [saving, setSaving] = useState(false);

  async function handleToggle(checked: boolean) {
    setSaving(true);
    try {
      await api.updateApp(app.id, { require_deploy_approval: checked });
      setEnabled(checked);
      addToast('success', checked ? 'Deploy approval required' : 'Deploy approval disabled');
    } catch (err: any) {
      addToast('error', err.message || 'Failed to update deploy approval setting');
    }
    setSaving(false);
  }

  return (
    <div class={styles.card}>
      <h2 class={styles.sectionTitle}>Deploy Approval</h2>
      <p class={styles.settingsDescription}>
        Require manual approval before deploys go live. Pending deploys will wait for a team member to review and approve them.
      </p>
      <Toggle
        label="Require deploy approval"
        description="New deploys will be held until explicitly approved or rejected"
        checked={enabled}
        disabled={saving}
        onChange={handleToggle}
      />
    </div>
  );
}
