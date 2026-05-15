import styles from './card.module.css';
import type { ComponentChildren } from 'preact';

type Props = {
  title?: string;
  children: ComponentChildren;
  actions?: ComponentChildren;
  variant?: 'default' | 'outlined' | 'elevated';
};

export default function Card({ title, children, actions, variant = 'default' }: Props) {
  return (
    <div class={`${styles.card} ${styles[variant]}`}>
      {(title || actions) && (
        <div class={styles.header}>
          {title && <h3 class={styles.title}>{title}</h3>}
          {actions && <div class={styles.actions}>{actions}</div>}
        </div>
      )}
      <div class={styles.body}>{children}</div>
    </div>
  );
}
