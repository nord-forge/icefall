import { useState } from 'preact/hooks';
import Button from '@islands/shared/Button/Button';
import { Save, Filter } from 'lucide-preact';
import type { NotificationChannel } from './NotificationsSection';
import { channelLabel } from './NotificationsSection';
import styles from '../settings-page.module.css';
import formStyles from '@styles/form.module.css';

const EVENT_TYPES = [
  { key: 'deploy.success', label: 'Deploy successful' },
  { key: 'deploy.failure', label: 'Deploy failed' },
  { key: 'health.down', label: 'Health check down' },
  { key: 'health.recovered', label: 'Health check recovered' },
  { key: 'health.auto_restart', label: 'Auto-restart triggered' },
  { key: 'backup.success', label: 'Backup successful' },
  { key: 'backup.failure', label: 'Backup failed' },
];

type Props = {
  channels: NotificationChannel[];
  onSaveMessage: (msg: string) => void;
};

export default function EventSubscriptionsSection({ channels, onSaveMessage }: Props) {
  const [subscriptions, setSubscriptions] = useState<Record<string, Set<string>>>({});
  const [saving, setSaving] = useState(false);

  function isSubscribed(channelId: string, eventType: string): boolean {
    return subscriptions[channelId]?.has(eventType) || false;
  }

  function toggleSubscription(channelId: string, eventType: string) {
    setSubscriptions(prev => {
      const current = new Set(prev[channelId] || []);
      if (current.has(eventType)) {
        current.delete(eventType);
      } else {
        current.add(eventType);
      }
      return { ...prev, [channelId]: current };
    });
  }

  async function saveSubscriptions() {
    setSaving(true);
    onSaveMessage('Subscriptions saved');
    setSaving(false);
  }

  return (
    <div class={styles.section}>
      <div class={styles.sectionHeaderRow}>
        <h2 class={styles.sectionHeading}><Filter size={18} aria-hidden="true" /> Event Subscriptions</h2>
        <Button variant="primary" size="sm" onClick={saveSubscriptions} loading={saving}>
          <Save size={14} aria-hidden="true" /> Save
        </Button>
      </div>
      <p class={styles.hint}>Choose which events trigger each notification channel. Failure events are enabled by default.</p>
      <div class={styles.subscriptionTable} role="grid" aria-label="Notification subscription matrix">
        <div class={styles.subHeader}>
          <div class={styles.subEventCell} />
          {channels.map(ch => (
            <div key={ch.id} class={styles.subChannelCell}>
              {channelLabel(ch)}
            </div>
          ))}
        </div>
        {EVENT_TYPES.map(evt => (
          <div key={evt.key} class={styles.subRow}>
            <div class={styles.subEventCell}>{evt.label}</div>
            {channels.map(ch => (
              <div key={ch.id} class={styles.subCheckCell}>
                <input
                  type="checkbox"
                  class={formStyles.checkbox}
                  checked={isSubscribed(ch.id, evt.key)}
                  onChange={() => toggleSubscription(ch.id, evt.key)}
                  aria-label={`${evt.label} via ${channelLabel(ch)}`}
                />
              </div>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}
