import { useState, useEffect } from 'preact/hooks';
import Badge from '@islands/shared/Badge/Badge';
import Button from '@islands/shared/Button/Button';
import Dropdown from '@islands/shared/Dropdown/Dropdown';
import { Key, MoreVertical, Plus } from 'lucide-preact';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import { formatRelativeTime } from '@lib/format';
import OAuthSetupDialog, { type OAuthProvider } from './OAuthSetupDialog';
import styles from '../settings-page.module.css';

type ProviderState = {
  clientId: string;
  hasSecret: boolean;
  enabled: boolean;
  configuredAt: string | null;
  callbackUrl: string;
};

const EMPTY: ProviderState = { clientId: '', hasSecret: false, enabled: false, configuredAt: null, callbackUrl: '' };

type Props = {
  onSaveMessage: (msg: string) => void;
};

export default function OAuthProvidersSection({ onSaveMessage }: Props) {
  const [github, setGithub] = useState<ProviderState>(EMPTY);
  const [google, setGoogle] = useState<ProviderState>(EMPTY);
  const [dialog, setDialog] = useState<OAuthProvider | null>(null);

  const load = () => {
    api.getOAuthSettings().then(d => {
      if (!d.data) return;
      setGithub({
        clientId: d.data.github_client_id || '',
        hasSecret: d.data.github_has_secret,
        enabled: d.data.github_enabled,
        configuredAt: d.data.github_configured_at,
        callbackUrl: d.data.github_callback_url,
      });
      setGoogle({
        clientId: d.data.google_client_id || '',
        hasSecret: d.data.google_has_secret,
        enabled: d.data.google_enabled,
        configuredAt: d.data.google_configured_at,
        callbackUrl: d.data.google_callback_url,
      });
    }).catch(() => {});
  };

  useEffect(load, []);

  async function disableProvider(provider: OAuthProvider) {
    try {
      await api.updateOAuthSettings(
        provider === 'github' ? { github_enabled: false } : { google_enabled: false },
      );
      onSaveMessage(`${provider === 'github' ? 'GitHub' : 'Google'} disabled`);
      load();
    } catch (e) {
      addToast('error', e instanceof Error ? e.message : 'Failed to disable provider');
    }
  }

  // lucide-preact dropped brand glyphs, so render the official Octicons
  // mark-github (MIT, github/octicons) inline.
  const githubMark = (
    <svg width="18" height="18" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
      <path d="M8 0C3.58 0 0 3.58 0 8C0 11.54 2.29 14.53 5.47 15.59C5.87 15.66 6.02 15.42 6.02 15.21C6.02 15.02 6.01 14.39 6.01 13.72C4 14.09 3.48 13.23 3.32 12.78C3.23 12.55 2.84 11.84 2.5 11.65C2.22 11.5 1.82 11.13 2.49 11.12C3.12 11.11 3.57 11.7 3.72 11.94C4.44 13.15 5.59 12.81 6.05 12.6C6.12 12.08 6.33 11.73 6.56 11.53C4.78 11.33 2.92 10.64 2.92 7.58C2.92 6.71 3.23 5.99 3.74 5.43C3.66 5.23 3.38 4.41 3.82 3.31C3.82 3.31 4.49 3.1 6.02 4.13C6.66 3.95 7.34 3.86 8.02 3.86C8.7 3.86 9.38 3.95 10.02 4.13C11.55 3.09 12.22 3.31 12.22 3.31C12.66 4.41 12.38 5.23 12.3 5.43C12.81 5.99 13.12 6.7 13.12 7.58C13.12 10.65 11.25 11.33 9.47 11.53C9.76 11.78 10.01 12.26 10.01 13.01C10.01 14.08 10 14.94 10 15.21C10 15.42 10.15 15.67 10.55 15.59C13.71 14.53 16 11.53 16 8C16 3.58 12.42 0 8 0Z" />
    </svg>
  );

  const rows: { id: OAuthProvider; name: string; state: ProviderState; icon: preact.JSX.Element }[] = [
    { id: 'github', name: 'GitHub', state: github, icon: githubMark },
    { id: 'google', name: 'Google', state: google, icon: <span class={styles.oauthGoogleMark} aria-hidden="true">G</span> },
  ];

  return (
    <div id="oauth" class={styles.section}>
      <h2 class={styles.sectionHeading}><Key size={18} aria-hidden="true" /> OAuth Providers</h2>
      <p class={styles.hint} style={{ marginTop: 0, marginBottom: 'var(--space-4)' }}>
        Let users sign in with GitHub or Google. Configure a provider, then test the connection end-to-end.
      </p>

      <div class={styles.oauthProviderList}>
        {rows.map(({ id, name, state, icon }) => {
          const connected = state.enabled && state.hasSecret && !!state.clientId;
          return (
            <div key={id} class={styles.oauthProviderRow}>
              <span class={styles.oauthProviderIcon}>{icon}</span>
              <div class={styles.oauthProviderInfo}>
                <span class={styles.oauthProviderName}>{name}</span>
                {connected ? (
                  <span class={styles.oauthProviderMeta}>
                    {state.configuredAt
                      ? `Connected ${formatRelativeTime(state.configuredAt)}`
                      : 'Connected'}
                  </span>
                ) : (
                  <span class={styles.oauthProviderMeta}>Not configured</span>
                )}
              </div>
              {connected ? (
                <div class={styles.oauthProviderActions}>
                  <Badge label="Connected" variant="success" />
                  <Dropdown
                    trigger={
                      <button type="button" class={styles.iconButton} aria-label={`Manage ${name}`}>
                        <MoreVertical size={16} aria-hidden="true" />
                      </button>
                    }
                    items={[
                      { label: 'Edit credentials', onClick: () => setDialog(id) },
                      { label: 'Test connection', onClick: () => setDialog(id) },
                      { label: 'Disable', onClick: () => disableProvider(id) },
                    ]}
                  />
                </div>
              ) : (
                <Button variant="secondary" onClick={() => setDialog(id)}>
                  <Plus size={14} aria-hidden="true" /> Configure {name}
                </Button>
              )}
            </div>
          );
        })}
      </div>

      {dialog && (
        <OAuthSetupDialog
          provider={dialog}
          open
          initialClientId={dialog === 'github' ? github.clientId : google.clientId}
          hasSecret={dialog === 'github' ? github.hasSecret : google.hasSecret}
          callbackUrl={dialog === 'github' ? github.callbackUrl : google.callbackUrl}
          onClose={() => setDialog(null)}
          onSaved={load}
        />
      )}
    </div>
  );
}
