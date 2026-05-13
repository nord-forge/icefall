import { useState, useEffect } from 'preact/hooks';
import { api } from '@lib/api';
import type { User, ApiToken } from '@lib/types';
import TwoFactorSection from '@islands/settings/TwoFactorSection/TwoFactorSection';
import Skeleton from '@islands/shared/Skeleton/Skeleton';
import AccountInfoSection from './components/AccountInfoSection';
import ChangePasswordSection from './components/ChangePasswordSection';
import ChangeEmailSection from './components/ChangeEmailSection';
import ConnectedAccountsSection from './components/ConnectedAccountsSection';
import ProfileTokensSection from './components/ProfileTokensSection';
import ActiveSessionsSection from './components/ActiveSessionsSection';
import PreferencesSection from './components/PreferencesSection';
import DangerZoneSection from './components/DangerZoneSection';
import styles from './profile-page.module.css';

type SessionEntry = {
  id: string;
  created_at: string;
  expires_at: string;
  is_current: boolean;
};

type OAuthIdentity = {
  id: string;
  provider: string;
  provider_email: string | null;
  created_at: string;
};

export default function ProfilePage() {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);

  // OAuth identities
  const [identities, setIdentities] = useState<OAuthIdentity[]>([]);
  const [enabledProviders, setEnabledProviders] = useState<{ github: boolean; google: boolean }>({ github: false, google: false });

  // API Tokens
  const [tokens, setTokens] = useState<ApiToken[]>([]);

  // Sessions
  const [sessions, setSessions] = useState<SessionEntry[]>([]);
  const [sessionsLoading, setSessionsLoading] = useState(false);
  const [sessionsMsg, setSessionsMsg] = useState('');

  // Preferences
  const [preferences, setPreferences] = useState<Record<string, unknown>>({});

  useEffect(() => {
    Promise.all([
      api.getMe().then(({ data }) => setUser(data)).catch(() => {}),
      api.listOAuthIdentities().then(({ data }) => setIdentities(data)).catch(() => {}),
      api.getEnabledOAuthProviders().then(({ data }) => setEnabledProviders(data)).catch(() => {}),
      api.listTokens().then(({ data }) => setTokens(data)).catch(() => {}),
      api.listSessions().then(({ data }) => setSessions(data)).catch(() => {}),
      api.getPreferences().then(({ data }) => setPreferences(data)).catch(() => {}),
    ]).finally(() => setLoading(false));
  }, []);

  // --- Password ---

  async function handleChangePassword(currentPassword: string, newPassword: string): Promise<string> {
    await api.changePassword(currentPassword, newPassword);
    return 'Password changed successfully. Other sessions have been signed out.';
  }

  function handlePasswordChanged() {
    // Refresh sessions list after password change
    api.listSessions().then(({ data }) => setSessions(data)).catch(() => {});
  }

  // --- Email ---

  async function handleChangeEmail(newEmail: string, password: string): Promise<string> {
    const { data } = await api.changeEmail(newEmail, password);
    setUser(prev => prev ? { ...prev, email: data.email } : prev);
    return 'Email updated successfully';
  }

  // --- Tokens ---

  async function handleCreateToken(name: string): Promise<string> {
    const { data } = await api.createToken(name);
    const { data: refreshed } = await api.listTokens();
    setTokens(refreshed);
    return data.token;
  }

  async function handleRevokeToken(tokenId: string) {
    await api.revokeToken(tokenId);
    setTokens(prev => prev.filter(t => t.id !== tokenId));
  }

  // --- OAuth ---

  async function handleUnlinkProvider(provider: string) {
    await api.unlinkOAuthProvider(provider);
    setIdentities(prev => prev.filter(i => i.provider !== provider));
  }

  // --- Sessions ---

  async function handleRevokeAllSessions() {
    setSessionsLoading(true);
    setSessionsMsg('');
    try {
      await api.revokeAllSessions();
      setSessionsMsg('All other sessions have been signed out.');
      const { data } = await api.listSessions();
      setSessions(data);
    } catch (e: any) {
      setSessionsMsg(e.message || 'Failed to revoke sessions');
    }
    setSessionsLoading(false);
  }

  // --- Preferences ---

  async function handleUpdatePreference(update: Record<string, unknown>) {
    setPreferences(prev => ({ ...prev, ...update }));
    await api.updatePreferences(update);
  }

  // --- Delete Account ---

  async function handleDeleteAccount(password: string) {
    await api.deleteAccount(password);
    window.location.href = '/login';
  }

  if (loading) {
    return (
      <div class={styles.page}>
        <Skeleton width="200px" height="2rem" />
        <Skeleton width="100%" height="200px" variant="rect" />
        <Skeleton width="100%" height="200px" variant="rect" />
        <Skeleton width="100%" height="160px" variant="rect" />
      </div>
    );
  }

  return (
    <div class={styles.page}>
      <div class={styles.pageHeader}>
        <h1 class={styles.pageTitle}>Account</h1>
        <p class={styles.pageSubtitle}>Manage your account settings and security</p>
      </div>

      {/* --- Account Info --- */}
      <AccountInfoSection user={user} />

      {/* --- Change Password --- */}
      <ChangePasswordSection
        onChangePassword={handleChangePassword}
        onPasswordChanged={handlePasswordChanged}
      />

      {/* --- Change Email --- */}
      <ChangeEmailSection onChangeEmail={handleChangeEmail} />

      {/* --- Two-Factor Authentication --- */}
      <TwoFactorSection />

      {/* --- Connected Accounts --- */}
      <ConnectedAccountsSection
        identities={identities}
        enabledProviders={enabledProviders}
        onUnlink={handleUnlinkProvider}
      />

      {/* --- API Tokens --- */}
      <ProfileTokensSection
        tokens={tokens}
        onCreateToken={handleCreateToken}
        onRevokeToken={handleRevokeToken}
      />

      {/* --- Active Sessions --- */}
      <ActiveSessionsSection
        sessions={sessions}
        sessionsLoading={sessionsLoading}
        sessionsMsg={sessionsMsg}
        onRevokeAllSessions={handleRevokeAllSessions}
      />

      {/* --- Preferences --- */}
      <PreferencesSection
        preferences={preferences}
        onUpdatePreference={handleUpdatePreference}
      />

      {/* --- Danger Zone --- */}
      <DangerZoneSection onDeleteAccount={handleDeleteAccount} />
    </div>
  );
}
