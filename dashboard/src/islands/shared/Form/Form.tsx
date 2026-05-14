import styles from './form.module.css';
import type { ComponentChildren } from 'preact';

type Props = {
  onSubmit: () => void;
  children: ComponentChildren;
  loading?: boolean;
};

export default function Form({ onSubmit, children, loading }: Props) {
  return (
    <form
      class={styles.form}
      onSubmit={(e) => { e.preventDefault(); if (!loading) onSubmit(); }}
    >
      <fieldset disabled={loading} class={styles.fieldset}>
        {children}
      </fieldset>
    </form>
  );
}
