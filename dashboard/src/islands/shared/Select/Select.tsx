import { useState, useRef, useEffect, useCallback } from 'preact/hooks';
import type { ComponentChildren } from 'preact';
import styles from './select.module.css';

export type SelectOption = {
  value: string;
  label: string;
  icon?: ComponentChildren;
};

type Props = {
  options: SelectOption[];
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  size?: 'sm' | 'md';
  id?: string;
  'aria-label'?: string;
  disabled?: boolean;
  fullWidth?: boolean;
};

export default function Select({
  options,
  value,
  onChange,
  placeholder = 'Select...',
  size = 'md',
  id,
  'aria-label': ariaLabel,
  disabled = false,
  fullWidth = false,
}: Props) {
  const [open, setOpen] = useState(false);
  const [focusIdx, setFocusIdx] = useState(-1);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  const selected = options.find((o) => o.value === value);

  const close = useCallback(() => {
    setOpen(false);
    setFocusIdx(-1);
    triggerRef.current?.focus();
  }, []);

  useEffect(() => {
    if (!open) return;
    function handleOutside(e: MouseEvent) {
      const target = e.target as Node;
      if (!triggerRef.current?.contains(target) && !listRef.current?.contains(target)) {
        setOpen(false);
        setFocusIdx(-1);
      }
    }
    document.addEventListener('mousedown', handleOutside);
    return () => document.removeEventListener('mousedown', handleOutside);
  }, [open]);

  useEffect(() => {
    if (!open || focusIdx < 0) return;
    const items = listRef.current?.querySelectorAll<HTMLLIElement>('[role="option"]');
    items?.[focusIdx]?.scrollIntoView({ block: 'nearest' });
  }, [focusIdx, open]);

  function handleTriggerClick() {
    if (disabled) return;
    if (open) {
      close();
    } else {
      setOpen(true);
      const idx = options.findIndex((o) => o.value === value);
      setFocusIdx(idx >= 0 ? idx : 0);
    }
  }

  function handleSelect(opt: SelectOption) {
    onChange(opt.value);
    close();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (disabled) return;

    if (!open) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        setOpen(true);
        const idx = options.findIndex((o) => o.value === value);
        setFocusIdx(idx >= 0 ? idx : 0);
      }
      return;
    }

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setFocusIdx((i) => (i < options.length - 1 ? i + 1 : 0));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setFocusIdx((i) => (i > 0 ? i - 1 : options.length - 1));
        break;
      case 'Home':
        e.preventDefault();
        setFocusIdx(0);
        break;
      case 'End':
        e.preventDefault();
        setFocusIdx(options.length - 1);
        break;
      case 'Enter':
      case ' ':
        e.preventDefault();
        if (focusIdx >= 0) handleSelect(options[focusIdx]);
        break;
      case 'Escape':
        e.preventDefault();
        close();
        break;
      case 'Tab':
        close();
        break;
    }
  }

  const listboxId = id ? `${id}-listbox` : undefined;
  const sizeClass = size === 'sm' ? styles.triggerSm : styles.triggerMd;

  return (
    <div class={`${styles.wrapper} ${fullWidth ? styles.fullWidth : ''}`} onKeyDown={handleKeyDown}>
      {/* a11y [WCAG 4.1.2]: combobox pattern with aria-expanded, aria-activedescendant */}
      <button
        ref={triggerRef}
        type="button"
        id={id}
        role="combobox"
        aria-expanded={open}
        aria-haspopup="listbox"
        aria-controls={listboxId}
        aria-activedescendant={open && focusIdx >= 0 ? `${id}-opt-${focusIdx}` : undefined}
        aria-label={ariaLabel}
        class={`${styles.trigger} ${sizeClass} ${open ? styles.triggerOpen : ''} ${disabled ? styles.triggerDisabled : ''}`}
        onClick={handleTriggerClick}
        disabled={disabled}
      >
        <span class={styles.triggerLabel}>
          {selected?.icon && <span class={styles.optionIcon}>{selected.icon}</span>}
          {selected?.label || placeholder}
        </span>
        <svg class={styles.chevron} width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="m6 9 6 6 6-6" />
        </svg>
      </button>

      {open && (
        <ul
          ref={listRef}
          id={listboxId}
          role="listbox"
          aria-label={ariaLabel}
          class={styles.listbox}
          tabIndex={-1}
        >
          {options.map((opt, i) => (
            <li
              key={opt.value}
              id={`${id}-opt-${i}`}
              role="option"
              aria-selected={opt.value === value}
              class={`${styles.option} ${i === focusIdx ? styles.optionFocused : ''} ${opt.value === value ? styles.optionSelected : ''}`}
              onClick={() => handleSelect(opt)}
              onMouseEnter={() => setFocusIdx(i)}
            >
              {opt.icon && <span class={styles.optionIcon}>{opt.icon}</span>}
              <span class={styles.optionLabel}>{opt.label}</span>
              {opt.value === value && (
                <svg class={styles.checkIcon} width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                  <path d="M20 6 9 17l-5-5" />
                </svg>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
