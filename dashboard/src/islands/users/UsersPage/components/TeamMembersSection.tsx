import { useState } from 'preact/hooks';
import type { User } from '@lib/types';
import { formatRelativeTime } from '@lib/format';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import Select from '@islands/shared/Select/Select';
import StatusDot from '@islands/shared/StatusDot/StatusDot';
import { UserPlus, Trash2, ShieldCheck, ShieldOff, RotateCcw, KeyRound } from 'lucide-preact';
import styles from '../users-page.module.css';
import formStyles from '@styles/form.module.css';

const ROLE_OPTIONS = [
  { value: 'admin', label: 'Admin' },
  { value: 'deployer', label: 'Deployer' },
  { value: 'viewer', label: 'Viewer' },
];

type Props = {
  users: User[];
  onChangeRole: (userId: string, role: string) => void;
  onDeactivate: (userId: string) => void;
  onResetPassword: (userId: string, email: string) => void;
  onReset2fa: (userId: string, email: string) => void;
  onInvite: (email: string, role: string) => Promise<void>;
};

export default function TeamMembersSection({
  users,
  onChangeRole,
  onDeactivate,
  onResetPassword,
  onReset2fa,
  onInvite,
}: Props) {
  const [showInvite, setShowInvite] = useState(false);
  const [inviteEmail, setInviteEmail] = useState('');
  const [inviteRole, setInviteRole] = useState('deployer');
  const [submitting, setSubmitting] = useState(false);

  async function handleInvite() {
    if (!inviteEmail.trim()) return;
    setSubmitting(true);
    await onInvite(inviteEmail.trim(), inviteRole);
    setInviteEmail('');
    setShowInvite(false);
    setSubmitting(false);
  }

  return (
    <section class={styles.section}>
      <div class={styles.sectionHeader}>
        <h2 class={styles.sectionTitle}>Team Members</h2>
        <Button variant="primary" onClick={() => setShowInvite(true)}>
          <UserPlus size={14} aria-hidden="true" /> Invite
        </Button>
      </div>

      {showInvite && (
        <div class={styles.card}>
          <div class={formStyles.fieldRow}>
            <Input
              label="Email"
              name="invite-email"
              id="invite-email"
              type="email"
              value={inviteEmail}
              onChange={setInviteEmail}
              placeholder="colleague@example.com"
            />
            <div>
              <label htmlFor="invite-role" class={formStyles.label}>Role</label>
              <Select
                id="invite-role"
                options={ROLE_OPTIONS}
                value={inviteRole}
                onChange={setInviteRole}
                fullWidth
              />
            </div>
          </div>
          <div class={styles.cardActions}>
            <Button variant="ghost" onClick={() => setShowInvite(false)}>Cancel</Button>
            <Button variant="primary" onClick={handleInvite} loading={submitting} disabled={!inviteEmail.trim()}>Send Invite</Button>
          </div>
        </div>
      )}

      <div class={styles.tableCard}>
        <table class={styles.table}>
          <thead>
            <tr class={styles.tableRow}>
              <th class={styles.th}>Email</th>
              <th class={styles.th}>Role</th>
              <th class={styles.th}>2FA</th>
              <th class={styles.th}>Status</th>
              <th class={styles.th}>Last Login</th>
              <th class={styles.th}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {users.map(u => (
              <tr key={u.id} class={styles.tableRow}>
                <td class={styles.td}>{u.email}</td>
                <td class={styles.td}>
                  <Select
                    options={ROLE_OPTIONS}
                    value={u.role}
                    onChange={(role) => onChangeRole(u.id, role)}
                    aria-label={`Role for ${u.email}`}
                    size="sm"
                    id={`role-${u.id}`}
                  />
                </td>
                <td class={styles.td}>
                  {/* a11y [1.1.1]: non-text content has accessible name via aria-label */}
                  {u.totp_enabled ? (
                    <span class={styles.badge2faOn} aria-label="2FA enabled" title="2FA enabled">
                      <ShieldCheck size={14} aria-hidden="true" /> On
                    </span>
                  ) : (
                    <span class={styles.badge2faOff} aria-label="2FA disabled" title="2FA disabled">
                      <ShieldOff size={14} aria-hidden="true" /> Off
                    </span>
                  )}
                </td>
                <td class={styles.td}>
                  <StatusDot status={u.is_active ? 'online' : 'stopped'} />
                </td>
                <td class={styles.tdMuted}>{u.last_login_at ? formatRelativeTime(u.last_login_at) : 'Never'}</td>
                <td class={styles.td}>
                  <div class={styles.actionRow}>
                    <button
                      type="button"
                      onClick={() => onResetPassword(u.id, u.email)}
                      class={styles.iconButton}
                      aria-label={`Reset password for ${u.email}`}
                      title="Reset password"
                    >
                      <KeyRound size={14} aria-hidden="true" />
                    </button>
                    {u.totp_enabled && (
                      <button
                        type="button"
                        onClick={() => onReset2fa(u.id, u.email)}
                        class={styles.iconButton}
                        aria-label={`Reset 2FA for ${u.email}`}
                        title="Reset 2FA"
                      >
                        <RotateCcw size={14} aria-hidden="true" />
                      </button>
                    )}
                    {u.is_active && (
                      <button
                        type="button"
                        onClick={() => onDeactivate(u.id)}
                        class={styles.iconButton}
                        aria-label={`Deactivate ${u.email}`}
                        title="Deactivate user"
                      >
                        <Trash2 size={14} aria-hidden="true" />
                      </button>
                    )}
                  </div>
                </td>
              </tr>
            ))}
            {users.length === 0 && (
              <tr>
                <td class={styles.emptyRow} colSpan={6}>No users yet.</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}
