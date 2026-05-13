import { Webhook, Copy, Check } from 'lucide-preact';
import styles from '../settings-tab.module.css';
import formStyles from '@styles/form.module.css';

type Props = {
  webhookBaseUrl: string;
  webhookSecret: string | null | undefined;
  hasWebhookSecret: boolean;
  gitBranch: string;
  copied: string;
  onCopy: (text: string, label: string) => void;
};

export default function AutoDeployCard({
  webhookBaseUrl,
  webhookSecret,
  hasWebhookSecret,
  gitBranch,
  copied,
  onCopy,
}: Props) {
  return (
    <div class={styles.card}>
      <h2 class={styles.sectionTitle}>
        <Webhook size={18} /> Auto-Deploy
      </h2>
      <p class={styles.settingsDescription}>
        Automatically deploy when you push to the configured branch. Configure the webhook URL in your Git provider's <a href="/settings" data-astro-prefetch="hover">settings</a>.
      </p>

      {hasWebhookSecret ? (
        <div class={styles.webhookInfo}>
          <div class={styles.webhookRow}>
            <label class={formStyles.label}>GitHub Webhook URL</label>
            <div class={styles.copyRow}>
              <code class={styles.codeBlock}>{webhookBaseUrl}/github</code>
              <button type="button" class={styles.copyButton} onClick={() => onCopy(webhookBaseUrl + '/github', 'github')} aria-label="Copy GitHub webhook URL">
                {copied === 'github' ? <Check size={14} /> : <Copy size={14} />}
              </button>
            </div>
          </div>
          <div class={styles.webhookRow}>
            <label class={formStyles.label}>GitLab Webhook URL</label>
            <div class={styles.copyRow}>
              <code class={styles.codeBlock}>{webhookBaseUrl}/gitlab</code>
              <button type="button" class={styles.copyButton} onClick={() => onCopy(webhookBaseUrl + '/gitlab', 'gitlab')} aria-label="Copy GitLab webhook URL">
                {copied === 'gitlab' ? <Check size={14} /> : <Copy size={14} />}
              </button>
            </div>
          </div>
          <div class={styles.webhookRow}>
            <label class={formStyles.label}>Webhook Secret</label>
            <div class={styles.copyRow}>
              <code class={styles.codeBlock}>{webhookSecret}</code>
              <button type="button" class={styles.copyButton} onClick={() => onCopy(webhookSecret || '', 'secret')} aria-label="Copy webhook secret">
                {copied === 'secret' ? <Check size={14} /> : <Copy size={14} />}
              </button>
            </div>
          </div>
          <p class={styles.fieldHint}>
            Deploys on push to: <code>{gitBranch}</code>
          </p>
        </div>
      ) : (
        <p class={styles.settingsNote}>
          Auto-deploy is not configured. A webhook secret will be generated when the app is deployed for the first time.
        </p>
      )}
    </div>
  );
}
