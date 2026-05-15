import { AlertTriangle, Info, CheckCircle, XCircle } from 'lucide-preact';
import styles from './alert.module.css';
import type { ComponentChildren } from 'preact';

type Props = {
  variant: 'info' | 'warning' | 'error' | 'success';
  children: ComponentChildren;
  actions?: ComponentChildren;
};

const icons = { info: Info, warning: AlertTriangle, error: XCircle, success: CheckCircle };

export default function Alert({ variant, children, actions }: Props) {
  const Icon = icons[variant];
  return (
    <div class={`${styles.alert} ${styles[variant]}`} role="alert">
      <div class={styles.content}>
        <Icon size={16} aria-hidden="true" />
        <span>{children}</span>
      </div>
      {actions && <div class={styles.actions}>{actions}</div>}
    </div>
  );
}
