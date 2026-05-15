import styles from './table.module.css';
import type { ComponentChildren } from 'preact';

type Column = {
  key: string;
  header: string;
  srOnly?: boolean;
};

type Props = {
  columns: Column[];
  children: ComponentChildren;
  emptyMessage?: string;
  isEmpty?: boolean;
};

export default function Table({ columns, children, emptyMessage = 'No data', isEmpty }: Props) {
  return (
    <div class={styles.wrapper}>
      <table class={styles.table}>
        <thead>
          <tr class={styles.headerRow}>
            {columns.map((col) => (
              <th key={col.key} class={styles.th}>
                {col.srOnly ? <span class="sr-only">{col.header}</span> : col.header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {isEmpty ? (
            <tr><td colSpan={columns.length} class={styles.empty}>{emptyMessage}</td></tr>
          ) : children}
        </tbody>
      </table>
    </div>
  );
}
