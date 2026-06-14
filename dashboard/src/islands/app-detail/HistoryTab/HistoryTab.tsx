import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import Card from '@islands/shared/Card/Card';
import Timeline from '@islands/shared/Timeline/Timeline';
import styles from './history-tab.module.css';

type HistoryEntry = { id: string; field: string; old_value: string | null; new_value: string | null; changed_by: string | null; changed_at: string; };

export default function HistoryTab({ appId }: { appId: string }) {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.request<{ data: HistoryEntry[] }>(`/apps/${appId}/config-history`)
      .then(({ data }) => setEntries(data))
      .finally(() => setLoading(false));
  }, [appId]);

  if (loading) return <p class={styles.loading}>Loading history...</p>;
  if (entries.length === 0) return <p class={styles.empty}>No configuration changes recorded yet.</p>;

  const items = entries.map(e => ({
    id: e.id,
    time: new Date(e.changed_at).toLocaleString(),
    content: (
      <div>
        <strong class={styles.field}>{e.field}</strong>
        {e.old_value && <span class={styles.old}>{e.old_value}</span>}
        <span class={styles.arrow}> → </span>
        {e.new_value && <span class={styles.new}>{e.new_value}</span>}
        {e.changed_by && <span class={styles.author}> by {e.changed_by}</span>}
      </div>
    ),
  }));

  return (
    <Card title="Configuration history">
      <Timeline items={items} />
    </Card>
  );
}
