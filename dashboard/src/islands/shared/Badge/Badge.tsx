import styles from './badge.module.css';

type Props = {
  label: string;
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info';
  size?: 'sm' | 'md';
};

export default function Badge({ label, variant = 'default', size = 'sm' }: Props) {
  return (
    <span class={`${styles.badge} ${styles[variant]} ${styles[size]}`}>
      {label}
    </span>
  );
}
