import { useState, useEffect } from 'preact/hooks';
import type { GitHubInstallation } from '@lib/types';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Button from '@islands/shared/Button/Button';
import ConfirmDialog from '@islands/shared/ConfirmDialog/ConfirmDialog';
import { GitBranch, Plus, Trash2 } from 'lucide-preact';
import styles from '../settings-page.module.css';

type Props = {
  onSaveMessage: (msg: string) => void;
};

export default function GitIntegrationsSection({ onSaveMessage }: Props) {
  const [installations, setInstallations] = useState<GitHubInstallation[]>([]);
  const [loading, setLoading] = useState(true);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [disconnecting, setDisconnecting] = useState(false);
  const [connecting, setConnecting] = useState(false);

  useEffect(() => {
    api.listGitSources()
      .then(({ data }) => {
        setInstallations(data);
        setLoading(false);
      })
      .catch(() => {
        addToast('error', 'Failed to load Git integrations');
        setLoading(false);
      });
  }, []);

  async function handleConnect() {
    setConnecting(true);
    try {
      const setup = await api.getGitHubSetup();
      const form = document.createElement('form');
      form.method = 'POST';
      form.action = setup.form_action;
      form.target = '_self';
      const input = document.createElement('input');
      input.type = 'hidden';
      input.name = 'manifest';
      input.value = JSON.stringify(setup.manifest);
      form.appendChild(input);
      document.body.appendChild(form);
      form.submit();
    } catch (err: any) {
      addToast('error', err.message || 'Failed to start GitHub setup');
      setConnecting(false);
    }
  }

  async function handleDisconnect(id: string) {
    const installation = installations.find((i) => i.id === id);
    try {
      await api.deleteGitSource(id);
      setInstallations((prev) => prev.filter((i) => i.id !== id));
      setConfirmDeleteId(null);
      onSaveMessage(`Disconnected ${installation?.account_name}`);
      addToast('success', `Disconnected ${installation?.account_name}`);
    } catch {
      addToast('error', 'Failed to disconnect Git source');
    }
  }

  return (
    <div class={styles.section}>
      <div class={styles.sectionHeaderRow}>
        <h2 class={styles.sectionHeading}>
          <GitBranch size={18} aria-hidden="true" /> Git Integrations
        </h2>
        <Button variant="secondary" onClick={handleConnect} loading={connecting}>
          <Plus size={14} aria-hidden="true" /> Connect GitHub
        </Button>
      </div>

      {loading ? (
        <p class={styles.emptyText}>Loading integrations...</p>
      ) : installations.length === 0 ? (
        <p class={styles.emptyText}>
          No Git sources connected. Connect a GitHub account to deploy from repositories.
        </p>
      ) : (
        <div class={styles.itemList}>
          {installations.map((inst) => (
            <div key={inst.id} class={styles.itemRow}>
              <div class={styles.itemInfo}>
                <span class={styles.itemLabel}>{inst.account_name}</span>
                <span class={styles.itemMeta}>
                  {inst.account_type === 'organization' ? 'Organization' : 'User'}
                  {' · '}
                  {inst.repo_count} {inst.repo_count === 1 ? 'repository' : 'repositories'}
                  {' · '}
                  {inst.status}
                </span>
              </div>
              <div class={styles.itemActions}>
                <button
                  type="button"
                  class={styles.iconButton}
                  onClick={() => setConfirmDeleteId(inst.id)}
                  aria-label={`Disconnect ${inst.account_name}`}
                >
                  <Trash2 size={14} aria-hidden="true" />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      <ConfirmDialog
        open={confirmDeleteId !== null}
        title="Disconnect GitHub integration?"
        description={`This will disconnect "${installations.find((i) => i.id === confirmDeleteId)?.account_name ?? 'this account'}" and revoke access to its repositories. Existing deploys will not be affected, but future deploys from this source will fail.`}
        confirmLabel="Disconnect"
        variant="danger"
        loading={disconnecting}
        onConfirm={async () => {
          if (!confirmDeleteId) return;
          setDisconnecting(true);
          try {
            await handleDisconnect(confirmDeleteId);
          } finally {
            setDisconnecting(false);
            setConfirmDeleteId(null);
          }
        }}
        onCancel={() => setConfirmDeleteId(null)}
      />
    </div>
  );
}
