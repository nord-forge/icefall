import styles from './toggle.module.css';

type Props = {
  label: string;
  description?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
};

export default function Toggle({ label, description, checked, disabled, onChange }: Props) {
  const id = `toggle-${label.toLowerCase().replace(/\s+/g, '-')}`;
  return (
    <div class={styles.field}>
      <div class={styles.row}>
        <label htmlFor={id} class={styles.label}>{label}</label>
        <button
          id={id} type="button" role="switch" aria-checked={checked}
          disabled={disabled}
          class={`${styles.switch} ${checked ? styles.on : ''}`}
          onClick={() => onChange(!checked)}
        >
          <span class={styles.thumb} />
        </button>
      </div>
      {description && <p class={styles.description}>{description}</p>}
    </div>
  );
}
