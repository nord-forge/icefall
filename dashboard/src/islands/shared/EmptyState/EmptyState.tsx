import type { LucideIcon } from 'lucide-preact';
import Button from '@islands/shared/Button/Button';
import styles from './empty-state.module.css';

type EmptyStateProps = {
  icon?: LucideIcon;
  title: string;
  description?: string;
  action?: { label: string; onClick: () => void };
  compact?: boolean;
};

export default function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  compact = false,
}: EmptyStateProps) {
  const wrapperClass = [
    styles.wrapper,
    compact && styles.compact,
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <div class={wrapperClass}>
      {Icon && (
        <div class={styles.iconWrap} aria-hidden="true">
          <Icon size={compact ? 32 : 48} />
        </div>
      )}
      <h3 class={styles.title}>{title}</h3>
      {description && <p class={styles.description}>{description}</p>}
      {action && (
        <Button variant="primary" size="sm" onClick={action.onClick}>
          {action.label}
        </Button>
      )}
    </div>
  );
}
