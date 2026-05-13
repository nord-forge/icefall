import type { Server } from '@lib/types';
import { parseComposeServices } from './ComposeStep';
import styles from '../app-create.module.css';
import formStyles from '@styles/form.module.css';

type DeploySource = 'git' | 'image' | 'compose';

type Props = {
  deploySource: DeploySource | null;
  form: {
    name: string;
    git_repo: string;
    git_branch: string;
    build_command: string;
    image_ref: string;
    port: string;
    compose_content: string;
    envContent: string;
  };
  hasMultipleServers: boolean;
  selectedServerId: string | null;
  servers: Server[];
};

export default function ReviewStep({
  deploySource,
  form,
  hasMultipleServers,
  selectedServerId,
  servers,
}: Props) {
  const isImage = deploySource === 'image';
  const isCompose = deploySource === 'compose';
  const services = isCompose ? parseComposeServices(form.compose_content) : [];

  return (
    <div class={formStyles.fieldGroup}>
      <h3 class={styles.reviewTitle}>Review</h3>
      <div class={styles.reviewGrid}>
        <span class={styles.reviewLabel}>Name</span>
        <span class={styles.reviewValue}>{form.name || '-'}</span>

        <span class={styles.reviewLabel}>Deploy Type</span>
        <span class={styles.reviewValue}>
          {isCompose ? 'Docker Compose' : isImage ? 'Docker Image' : 'Git Repository'}
        </span>

        {isCompose ? (
          <>
            <span class={styles.reviewLabel}>Services</span>
            <span class={styles.reviewValue}>
              {services.length > 0 ? services.join(', ') : '-'}
            </span>
          </>
        ) : isImage ? (
          <>
            <span class={styles.reviewLabel}>Image</span>
            <span class={styles.reviewMono}>{form.image_ref}</span>
            <span class={styles.reviewLabel}>Port</span>
            <span class={styles.reviewMono}>{form.port}</span>
          </>
        ) : (
          <>
            <span class={styles.reviewLabel}>Repository</span>
            <span class={styles.reviewMono}>{form.git_repo || '-'}</span>
            <span class={styles.reviewLabel}>Branch</span>
            <span class={styles.reviewMono}>{form.git_branch}</span>
            {form.build_command && <>
              <span class={styles.reviewLabel}>Build</span>
              <span class={styles.reviewMono}>{form.build_command}</span>
            </>}
          </>
        )}

        {hasMultipleServers && selectedServerId && <>
          <span class={styles.reviewLabel}>Server</span>
          <span class={styles.reviewValue}>
            {servers.find((s) => s.id === selectedServerId)?.name || selectedServerId}
          </span>
        </>}

        {form.envContent && <>
          <span class={styles.reviewLabel}>Env vars</span>
          <span>{form.envContent.split('\n').filter((l) => l.trim() && !l.startsWith('#')).length} variable(s)</span>
        </>}
      </div>
    </div>
  );
}
