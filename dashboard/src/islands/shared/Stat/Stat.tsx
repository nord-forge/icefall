import styles from './stat.module.css';

type Props = {
  label: string;
  value: string | number;
  trend?: 'up' | 'down' | 'flat';
  detail?: string;
};

export default function Stat({ label, value, trend, detail }: Props) {
  return (
    <div class={styles.stat}>
      <dt class={styles.label}>{label}</dt>
      <dd class={styles.value}>
        {value}
        {trend && <span class={`${styles.trend} ${styles[trend]}`} aria-label={`Trending ${trend}`} />}
      </dd>
      {detail && <p class={styles.detail}>{detail}</p>}
    </div>
  );
}
