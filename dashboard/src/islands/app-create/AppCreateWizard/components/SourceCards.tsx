import { GitBranch, Container, Layers } from 'lucide-preact';
import styles from '../app-create.module.css';

type DeploySource = 'git' | 'image' | 'compose';

type Props = {
  onSelect: (source: DeploySource) => void;
};

export default function SourceCards({ onSelect }: Props) {
  return (
    <div class={styles.sourceGrid}>
      <button
        type="button"
        class={styles.sourceCard}
        onClick={() => onSelect('git')}
        aria-label="Deploy from Git repository"
      >
        <GitBranch size={28} aria-hidden="true" />
        <span class={styles.sourceCardTitle}>Deploy from Git</span>
        <span class={styles.sourceCardDescription}>
          Connect a repository, build from source, and deploy automatically.
        </span>
      </button>
      <button
        type="button"
        class={styles.sourceCard}
        onClick={() => onSelect('image')}
        aria-label="Deploy a Docker image"
      >
        <Container size={28} aria-hidden="true" />
        <span class={styles.sourceCardTitle}>Deploy Docker Image</span>
        <span class={styles.sourceCardDescription}>
          Pull a pre-built image from a registry and deploy it directly.
        </span>
      </button>
      <button
        type="button"
        class={styles.sourceCard}
        onClick={() => onSelect('compose')}
        aria-label="Deploy from Docker Compose"
      >
        <Layers size={28} aria-hidden="true" />
        <span class={styles.sourceCardTitle}>Docker Compose</span>
        <span class={styles.sourceCardDescription}>
          Deploy a multi-container stack from a docker-compose.yml file.
        </span>
      </button>
    </div>
  );
}
