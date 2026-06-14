import { useState } from 'preact/hooks';
import TwoFactorSection from '@islands/settings/TwoFactorSection/TwoFactorSection';
import UpdateSettings from '@islands/update/UpdateSettings/UpdateSettings';
import Button from '@islands/shared/Button/Button';
import GeneralSection from './components/GeneralSection';
import NotificationsSection from './components/NotificationsSection';
import type { NotificationChannel } from './components/NotificationsSection';
import EventSubscriptionsSection from './components/EventSubscriptionsSection';
import BackupLocationsSection from './components/BackupLocationsSection';
import InstanceBackupSection from './components/InstanceBackupSection';
import OAuthProvidersSection from './components/OAuthProvidersSection';
import GitIntegrationsSection from './components/GitIntegrationsSection';
import GlobalLogDrainsSection from './components/GlobalLogDrainsSection';
import CloudflareTunnelSection from './components/CloudflareTunnelSection';
import ContainerCleanupSection from './components/ContainerCleanupSection';
import McpServerSection from './components/McpServerSection';
import styles from './settings-page.module.css';

export default function SettingsPage() {
  const [saveMessage, setSaveMessage] = useState('');
  const [channels, setChannels] = useState<NotificationChannel[]>([]);

  return (
    <div>
      <h1 class={styles.pageTitle}>Platform Settings</h1>
      <p role="status" aria-live="polite" class={styles.saveStatus}>{saveMessage}</p>

      <GeneralSection onSaveMessage={setSaveMessage} />

      <NotificationsSection
        onSaveMessage={setSaveMessage}
        onChannelsChange={setChannels}
      />

      {channels.length > 0 && (
        <EventSubscriptionsSection
          channels={channels}
          onSaveMessage={setSaveMessage}
        />
      )}

      <BackupLocationsSection onSaveMessage={setSaveMessage} />

      <InstanceBackupSection onSaveMessage={setSaveMessage} />

      <OAuthProvidersSection onSaveMessage={setSaveMessage} />

      <GitIntegrationsSection onSaveMessage={setSaveMessage} />

      <GlobalLogDrainsSection onSaveMessage={setSaveMessage} />

      <CloudflareTunnelSection onSaveMessage={setSaveMessage} />

      <ContainerCleanupSection onSaveMessage={setSaveMessage} />

      <div class={styles.section}>
        <h2 class={styles.sectionHeading}>Teams</h2>
        <p class={styles.mcpDescription}>
          Manage your teams, members, and permissions.
        </p>
        <a href="/teams"><Button variant="secondary">Manage teams</Button></a>
      </div>

      <TwoFactorSection />

      <McpServerSection />

      <UpdateSettings />
    </div>
  );
}
