import styles from './toggle.module.css';

type Props = {
  label: string;
  description?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
  /** Render only the switch (no visible label/description row). The `label`
   *  is still used as the accessible name via aria-label. Use when the caller
   *  already provides its own visible label. */
  hideLabel?: boolean;
  id?: string;
};

function SwitchButton({
  id,
  checked,
  disabled,
  onChange,
  ariaLabel,
}: {
  id?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
  ariaLabel?: string;
}) {
  return (
    <button
      id={id}
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel}
      disabled={disabled}
      class={`${styles.switch} ${checked ? styles.on : ''}`}
      onClick={() => onChange(!checked)}
    >
      <span class={styles.thumb}>
        {/* a11y [WCAG 1.4.1]: shape cue inside the thumb — state not conveyed by color alone */}
        <svg class={styles.icon} width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <path class={styles.check} d="M2.5 5 L4.5 7 L7.5 3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
          <path class={styles.cross} d="M3 3 L7 7 M7 3 L3 7" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
        </svg>
      </span>
    </button>
  );
}

export default function Toggle({
  label,
  description,
  checked,
  disabled,
  onChange,
  hideLabel,
  id,
}: Props) {
  const switchId = id ?? `toggle-${label.toLowerCase().replace(/\s+/g, '-')}`;

  if (hideLabel) {
    return (
      <SwitchButton
        id={switchId}
        checked={checked}
        disabled={disabled}
        onChange={onChange}
        ariaLabel={label}
      />
    );
  }

  return (
    <div class={styles.field}>
      <div class={styles.row}>
        <label htmlFor={switchId} class={styles.label}>{label}</label>
        <SwitchButton id={switchId} checked={checked} disabled={disabled} onChange={onChange} />
      </div>
      {description && <p class={styles.description}>{description}</p>}
    </div>
  );
}
