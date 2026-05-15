import styles from './textarea.module.css';

type Props = {
  label?: string;
  name: string;
  id?: string;
  value?: string;
  placeholder?: string;
  helpText?: string;
  error?: string;
  rows?: number;
  required?: boolean;
  disabled?: boolean;
  mono?: boolean;
  className?: string;
  readOnly?: boolean;
  onChange?: (value: string) => void;
};

export default function Textarea({
  label, name, id: idProp, value = '', placeholder, helpText, error,
  rows = 4, required, disabled, mono, className, readOnly, onChange,
}: Props) {
  const id = idProp || `textarea-${name}`;
  return (
    <div class={`${styles.field} ${className || ''}`}>
      {label && (
        <label htmlFor={id} class={styles.label}>
          {label}{required && <span class={styles.required} aria-hidden="true">*</span>}
        </label>
      )}
      <textarea
        id={id} name={name} value={value} placeholder={placeholder}
        rows={rows} required={required} disabled={disabled} readOnly={readOnly}
        class={`${styles.textarea} ${mono ? styles.mono : ''} ${error ? styles.error : ''}`}
        aria-describedby={helpText ? `${id}-help` : error ? `${id}-err` : undefined}
        aria-invalid={!!error}
        onInput={(e) => onChange?.((e.target as HTMLTextAreaElement).value)}
      />
      {helpText && <p id={`${id}-help`} class={styles.helpText}>{helpText}</p>}
      {error && <p id={`${id}-err`} class={styles.errorText} role="alert">{error}</p>}
    </div>
  );
}
