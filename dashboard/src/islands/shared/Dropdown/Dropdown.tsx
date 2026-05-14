import { useState, useRef, useEffect } from 'preact/hooks';
import styles from './dropdown.module.css';
import type { ComponentChildren } from 'preact';

type MenuItem = {
  label: string;
  icon?: ComponentChildren;
  onClick: () => void;
  danger?: boolean;
};

type Props = {
  trigger: ComponentChildren;
  items: MenuItem[];
};

export default function Dropdown({ trigger, items }: Props) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    }
    if (open) document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [open]);

  return (
    <div class={styles.wrapper} ref={ref}>
      <div onClick={() => setOpen(!open)}>{trigger}</div>
      {open && (
        <div class={styles.menu} role="menu">
          {items.map((item, i) => (
            <button
              key={i} type="button" role="menuitem"
              class={`${styles.item} ${item.danger ? styles.danger : ''}`}
              onClick={() => { item.onClick(); setOpen(false); }}
            >
              {item.icon && <span class={styles.icon}>{item.icon}</span>}
              {item.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
