import styles from './timeline.module.css';
import type { ComponentChildren } from 'preact';

type TimelineItem = {
  id: string;
  time: string;
  content: ComponentChildren;
  icon?: ComponentChildren;
};

type Props = {
  items: TimelineItem[];
};

export default function Timeline({ items }: Props) {
  return (
    <ol class={styles.timeline} role="list">
      {items.map((item) => (
        <li key={item.id} class={styles.item}>
          <div class={styles.marker}>{item.icon || <span class={styles.dot} />}</div>
          <div class={styles.content}>
            <time class={styles.time}>{item.time}</time>
            <div>{item.content}</div>
          </div>
        </li>
      ))}
    </ol>
  );
}
