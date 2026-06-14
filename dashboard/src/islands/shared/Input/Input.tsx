import { useState } from 'preact/hooks';
import { Eye, EyeOff } from 'lucide-preact';
import styles from './input.module.css';

type Props = {
  label?: string;
  name: string;
  id?: string;
  type?: 'text' | 'email' | 'password' | 'number' | 'url';
  value?: string;
  placeholder?: string;
  helpText?: string;
  error?: string;
  required?: boolean;
  disabled?: boolean;
  revealable?: boolean;
  mono?: boolean;
  className?: string;
  min?: string | number;
  max?: string | number;
  step?: string | number;
  readOnly?: boolean;
  onChange?: (value: string) => void;
};

export default function Input({
  label, name, id: idProp, type = 'text', value = '', placeholder, helpText, error,
  required, disabled, revealable, mono, className, min, max, step, readOnly, onChange,
}: Props) {
  const [revealed, setRevealed] = useState(false);
  const inputType = revealable && revealed ? 'text' : type;
  const id = idProp || `input-${name}`;

  return (
    <div class={`${styles.field} ${className || ''}`}>
      {label && (
        <label htmlFor={id} class={styles.label}>
          {label}
          {required && <span class={styles.required} aria-hidden="true">*</span>}
        </label>
      )}
      <div class={styles.inputWrapper}>
        <input
          id={id}
          name={name}
          type={inputType}
          value={value}
          placeholder={placeholder}
          required={required}
          disabled={disabled}
          readOnly={readOnly}
          min={min}
          max={max}
          step={step}
          class={`${styles.input} ${mono ? styles.mono : ''} ${error ? styles.inputError : ''}`}
          aria-describedby={helpText ? `${id}-help` : error ? `${id}-error` : undefined}
          aria-invalid={!!error}
          onInput={(e) => onChange?.((e.target as HTMLInputElement).value)}
        />
        {revealable && (
          <button
            type="button"
            class={styles.revealBtn}
            onClick={() => setRevealed(!revealed)}
            aria-label={revealed ? 'Hide value' : 'Show value'}
          >
            {revealed ? <EyeOff size={16} /> : <Eye size={16} />}
          </button>
        )}
      </div>
      {helpText && <p id={`${id}-help`} class={styles.helpText}>{helpText}</p>}
      {error && <p id={`${id}-error`} class={styles.error} role="alert">{error}</p>}
    </div>
  );
}
