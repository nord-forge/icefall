import { useEffect, useState } from 'preact/hooks';
import { useStore } from '@nanostores/preact';
import { $users, $tokens, $usersLoaded } from '@stores/users';
import { api } from '@lib/api';
import type { User, ApiToken, RegistrationSettings } from '@lib/types';
import { SkeletonTable } from '@islands/shared/Skeleton/Skeleton';
import RegistrationSettingsSection from './components/RegistrationSettings';
import TeamMembersSection from './components/TeamMembersSection';
import ResetPasswordModal from './components/ResetPasswordModal';
import Reset2faModal from './components/Reset2faModal';
import ApiTokensSection from './components/ApiTokensSection';
import styles from './users-page.module.css';

export default function UsersPage() {
  const cachedUsers = useStore($users);
  const cachedTokens = useStore($tokens);
  const wasLoaded = useStore($usersLoaded);
  const [users, setUsers] = useState<User[]>(cachedUsers);
  const [tokens, setTokens] = useState<ApiToken[]>(cachedTokens);
  const [loading, setLoading] = useState(!wasLoaded);
  const [newTokenValue, setNewTokenValue] = useState('');

  const [regSettings, setRegSettings] = useState<RegistrationSettings>({
    allow_registration: false,
    allowed_domains: null,
    default_role: 'viewer',
  });
  const [regLoading, setRegLoading] = useState(true);
  const [regSaving, setRegSaving] = useState(false);
  const [domainsInput, setDomainsInput] = useState('');

  const [resetPasswordModal, setResetPasswordModal] = useState<{
    userId: string;
    email: string;
    tempPassword?: string;
  } | null>(null);
  const [resettingPassword, setResettingPassword] = useState(false);

  const [reset2faConfirm, setReset2faConfirm] = useState<{
    userId: string;
    email: string;
  } | null>(null);
  const [resetting2fa, setResetting2fa] = useState(false);

  useEffect(() => {
    Promise.all([
      api.listUsers().then(({ data }) => { setUsers(data); $users.set(data); }).catch(() => {}),
      api.listTokens().then(({ data }) => { setTokens(data); $tokens.set(data); }).catch(() => {}),
      api.getRegistrationSettings().then(({ data }) => {
        setRegSettings(data);
        setDomainsInput(data.allowed_domains || '');
      }).catch(() => {}),
    ]).then(() => {
      setLoading(false);
      setRegLoading(false);
      $usersLoaded.set(true);
    });
  }, []);

  async function handleInvite(email: string, role: string) {
    try {
      await api.inviteUser(email, role);
      const { data } = await api.listUsers();
      setUsers(data);
    } catch {}
  }

  async function handleChangeRole(userId: string, role: string) {
    try {
      await api.changeRole(userId, role);
      setUsers(prev => prev.map(u => u.id === userId ? { ...u, role: role as User['role'] } : u));
    } catch {}
  }

  async function handleDeactivate(userId: string) {
    try {
      await api.deactivateUser(userId);
      setUsers(prev => prev.map(u => u.id === userId ? { ...u, is_active: false } : u));
    } catch {}
  }

  async function handleResetPassword() {
    if (!resetPasswordModal) return;
    setResettingPassword(true);
    try {
      const { data } = await api.resetUserPassword(resetPasswordModal.userId);
      setResetPasswordModal({
        ...resetPasswordModal,
        tempPassword: data.temporary_password,
      });
    } catch {}
    setResettingPassword(false);
  }

  async function handleReset2fa() {
    if (!reset2faConfirm) return;
    setResetting2fa(true);
    try {
      await api.resetUser2fa(reset2faConfirm.userId);
      setUsers(prev =>
        prev.map(u =>
          u.id === reset2faConfirm.userId ? { ...u, totp_enabled: false } : u,
        ),
      );
      setReset2faConfirm(null);
    } catch {}
    setResetting2fa(false);
  }

  async function handleSaveRegSettings() {
    setRegSaving(true);
    try {
      const { data } = await api.updateRegistrationSettings({
        allow_registration: regSettings.allow_registration,
        allowed_domains: domainsInput.trim() || undefined,
        default_role: regSettings.default_role,
      });
      setRegSettings(data);
      setDomainsInput(data.allowed_domains || '');
    } catch {}
    setRegSaving(false);
  }

  async function handleCreateToken(name: string) {
    try {
      const { data } = await api.createToken(name);
      setNewTokenValue(data.token);
      const { data: refreshed } = await api.listTokens();
      setTokens(refreshed);
    } catch {}
  }

  async function handleRevokeToken(tokenId: string) {
    try {
      await api.revokeToken(tokenId);
      setTokens(prev => prev.filter(t => t.id !== tokenId));
    } catch {}
  }

  return (
    <div>
      <div class={styles.pageHeader}>
        <h1 class={styles.pageTitle}>Users & Tokens</h1>
      </div>

      {loading ? (
        <SkeletonTable rows={5} columns={6} />
      ) : (
        <>
          <RegistrationSettingsSection
            settings={regSettings}
            domainsInput={domainsInput}
            loading={regLoading}
            saving={regSaving}
            onSettingsChange={setRegSettings}
            onDomainsChange={setDomainsInput}
            onSave={handleSaveRegSettings}
          />

          <TeamMembersSection
            users={users}
            onChangeRole={handleChangeRole}
            onDeactivate={handleDeactivate}
            onResetPassword={(userId, email) => setResetPasswordModal({ userId, email })}
            onReset2fa={(userId, email) => setReset2faConfirm({ userId, email })}
            onInvite={handleInvite}
          />

          {resetPasswordModal && (
            <ResetPasswordModal
              userId={resetPasswordModal.userId}
              email={resetPasswordModal.email}
              tempPassword={resetPasswordModal.tempPassword}
              resetting={resettingPassword}
              onConfirm={handleResetPassword}
              onClose={() => setResetPasswordModal(null)}
            />
          )}

          {reset2faConfirm && (
            <Reset2faModal
              email={reset2faConfirm.email}
              resetting={resetting2fa}
              onConfirm={handleReset2fa}
              onClose={() => setReset2faConfirm(null)}
            />
          )}

          <ApiTokensSection
            tokens={tokens}
            newTokenValue={newTokenValue}
            onCreateToken={handleCreateToken}
            onRevokeToken={handleRevokeToken}
            onDismissNewToken={() => setNewTokenValue('')}
          />
        </>
      )}
    </div>
  );
}
