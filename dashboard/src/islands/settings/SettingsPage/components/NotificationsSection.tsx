import { useState, useEffect } from 'preact/hooks';
import { useStore } from '@nanostores/preact';
import { $channels, $settingsLoaded } from '@stores/settings';
import Button from '@islands/shared/Button/Button';
import Select from '@islands/shared/Select/Select';
import { Bell, Plus, Trash2, Send } from 'lucide-preact';
import styles from '../settings-page.module.css';
import formStyles from '@styles/form.module.css';

export type NotificationChannel = {
  id: string;
  channel_type: string;
  config: Record<string, string>;
  created_at: string;
};

const CHANNEL_TYPES = [
  { value: 'webhook', label: 'Webhook' },
  { value: 'smtp', label: 'Email (SMTP)' },
  { value: 'slack', label: 'Slack' },
  { value: 'discord', label: 'Discord' },
];

function configFieldsForType(type: string) {
  switch (type) {
    case 'webhook': return [{ key: 'url', label: 'Webhook URL', placeholder: 'https://hooks.slack.com/...' }];
    case 'smtp': return [
      { key: 'host', label: 'SMTP Host', placeholder: 'smtp.example.com' },
      { key: 'port', label: 'Port', placeholder: '587' },
      { key: 'username', label: 'Username', placeholder: '' },
      { key: 'password', label: 'Password', placeholder: '' },
      { key: 'from', label: 'From Address', placeholder: 'alerts@example.com' },
      { key: 'to', label: 'To Address', placeholder: 'team@example.com' },
    ];
    case 'slack': return [{ key: 'url', label: 'Slack Webhook URL', placeholder: 'https://hooks.slack.com/services/...' }];
    case 'discord': return [{ key: 'url', label: 'Discord Webhook URL', placeholder: 'https://discord.com/api/webhooks/...' }];
    default: return [{ key: 'url', label: 'URL', placeholder: '' }];
  }
}

export function channelLabel(ch: NotificationChannel) {
  const type = CHANNEL_TYPES.find(t => t.value === ch.channel_type);
  return type?.label || ch.channel_type;
}

function channelSummary(ch: NotificationChannel) {
  if (ch.channel_type === 'webhook') return ch.config.url || '';
  if (ch.channel_type === 'smtp') return ch.config.host ? `${ch.config.host}:${ch.config.port || '587'}` : '';
  if (ch.channel_type === 'slack') return ch.config.channel || ch.config.url || '';
  if (ch.channel_type === 'discord') return ch.config.url || '';
  return JSON.stringify(ch.config);
}

type Props = {
  onSaveMessage: (msg: string) => void;
  onChannelsChange: (channels: NotificationChannel[]) => void;
};

export default function NotificationsSection({ onSaveMessage, onChannelsChange }: Props) {
  const cachedChannels = useStore($channels);
  const [channels, setChannels] = useState<NotificationChannel[]>(cachedChannels);
  const [showAddChannel, setShowAddChannel] = useState(false);
  const [newChannelType, setNewChannelType] = useState('webhook');
  const [newChannelConfig, setNewChannelConfig] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState('');

  useEffect(() => {
    fetch('/api/v1/notifications/channels', { credentials: 'same-origin' }).then(r => r.json()).then(d => {
      const data = d.data || [];
      setChannels(data);
      $channels.set(data);
      $settingsLoaded.set(true);
      onChannelsChange(data);
    }).catch(() => {});
  }, []);

  function updateChannels(data: NotificationChannel[]) {
    setChannels(data);
    onChannelsChange(data);
  }

  async function addChannel() {
    setSaving(true);
    try {
      await fetch('/api/v1/notifications/channels', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ channel_type: newChannelType, config: newChannelConfig }),
      });
      const res = await fetch('/api/v1/notifications/channels');
      const d = await res.json();
      updateChannels(d.data || []);
      setShowAddChannel(false);
      setNewChannelType('webhook');
      setNewChannelConfig({});
      onSaveMessage('Channel added');
    } catch { onSaveMessage('Failed to add channel'); }
    setSaving(false);
  }

  async function deleteChannel(id: string) {
    try {
      await fetch(`/api/v1/notifications/channels/${id}`, { method: 'DELETE' });
      updateChannels(channels.filter(c => c.id !== id));
    } catch {}
  }

  async function testChannel(id: string) {
    setTesting(id);
    try {
      await fetch(`/api/v1/notifications/channels/${id}/test`, { method: 'POST' });
      onSaveMessage('Test notification sent');
    } catch { onSaveMessage('Test failed'); }
    setTesting('');
  }

  return (
    <div class={styles.section}>
      <div class={styles.sectionHeaderRow}>
        <h2 class={styles.sectionHeading}><Bell size={18} aria-hidden="true" /> Notifications</h2>
        <Button variant="secondary" onClick={() => setShowAddChannel(true)}>
          <Plus size={14} aria-hidden="true" /> Add Channel
        </Button>
      </div>

      {showAddChannel && (
        <div class={styles.addCard}>
          <div class={formStyles.fieldRow}>
            <div>
              <label htmlFor="channel-type" class={formStyles.label}>Channel Type</label>
              <Select
                id="channel-type"
                options={CHANNEL_TYPES}
                value={newChannelType}
                onChange={(v) => { setNewChannelType(v); setNewChannelConfig({}); }}
                fullWidth
              />
            </div>
          </div>
          <div class={formStyles.fieldRow}>
            {configFieldsForType(newChannelType).map(f => (
              <div key={f.key}>
                <label htmlFor={`ch-${f.key}`} class={formStyles.label}>{f.label}</label>
                <input
                  id={`ch-${f.key}`}
                  class={formStyles.input}
                  type={f.key === 'password' ? 'password' : 'text'}
                  autoComplete={f.key === 'password' ? 'off' : undefined}
                  value={newChannelConfig[f.key] || ''}
                  onInput={e => setNewChannelConfig(prev => ({ ...prev, [f.key]: (e.target as HTMLInputElement).value }))}
                  placeholder={f.placeholder}
                />
              </div>
            ))}
          </div>
          <div class={styles.addCardActions}>
            <Button variant="ghost" onClick={() => { setShowAddChannel(false); setNewChannelConfig({}); }}>Cancel</Button>
            <Button variant="primary" onClick={addChannel} loading={saving}>Add Channel</Button>
          </div>
        </div>
      )}

      {channels.length === 0 && !showAddChannel ? (
        <p class={styles.emptyText}>No notification channels configured.</p>
      ) : (
        <div class={styles.itemList}>
          {channels.map(ch => (
            <div key={ch.id} class={styles.itemRow}>
              <div class={styles.itemInfo}>
                <span class={styles.itemLabel}>{channelLabel(ch)}</span>
                <span class={styles.itemMeta}>{channelSummary(ch)}</span>
              </div>
              <div class={styles.itemActions}>
                <Button variant="ghost" size="sm" onClick={() => testChannel(ch.id)} loading={testing === ch.id}>
                  <Send size={12} aria-hidden="true" /> Test
                </Button>
                <button type="button" class={styles.iconButton} onClick={() => deleteChannel(ch.id)} aria-label={`Delete ${channelLabel(ch)} channel`}>
                  <Trash2 size={14} aria-hidden="true" />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
