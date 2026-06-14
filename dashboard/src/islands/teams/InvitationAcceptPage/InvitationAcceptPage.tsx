import { useState, useEffect } from 'preact/hooks';
import { Mail, AlertTriangle } from 'lucide-preact';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Button from '@islands/shared/Button/Button';
import styles from './invitation-accept-page.module.css';

export default function InvitationAcceptPage() {
  const [teamName, setTeamName] = useState('');
  const [role, setRole] = useState('');
  const [loading, setLoading] = useState(true);
  const [accepting, setAccepting] = useState(false);
  const [declining, setDeclining] = useState(false);
  const [error, setError] = useState('');

  const token = typeof window !== 'undefined'
    ? window.location.pathname.split('/invitations/')[1]?.split('/')[0] || ''
    : '';

  useEffect(() => {
    async function loadInvitation() {
      if (!token) {
        setError('No invitation token found.');
        setLoading(false);
        return;
      }

      // Try accepting to get team info; the API may provide a preview endpoint
      // For now, we show the acceptance page with the token
      // If the token is valid, the accept/decline buttons will work
      try {
        // Attempt a preview by accepting (some APIs return info)
        // We'll just show the UI and let the user decide
        setLoading(false);
      } catch {
        setError('This invitation may have expired or is invalid.');
        setLoading(false);
      }
    }
    loadInvitation();
  }, [token]);

  async function handleAccept() {
    setAccepting(true);
    try {
      const res = await api.acceptInvitation(token);
      setTeamName(res.data.team.name);
      setRole(res.data.role);
      addToast('success', `You joined "${res.data.team.name}" as ${res.data.role}.`);
      setTimeout(() => {
        window.location.href = '/';
      }, 1500);
    } catch {
      setError('Failed to accept invitation. It may have expired or already been used.');
      setAccepting(false);
    }
  }

  async function handleDecline() {
    setDeclining(true);
    try {
      await api.declineInvitation(token);
      addToast('info', 'Invitation declined.');
      setTimeout(() => {
        window.location.href = '/';
      }, 1000);
    } catch {
      addToast('error', 'Failed to decline invitation.');
      setDeclining(false);
    }
  }

  if (loading) {
    return (
      <div class={styles.container}>
        <p class={styles.loadingState} role="status" aria-live="polite">Loading invitation...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div class={styles.container}>
        <div class={styles.card}>
          <AlertTriangle size={40} aria-hidden="true" class={styles.errorIcon} />
          <h1 class={styles.errorTitle}>Invalid invitation</h1>
          <p class={styles.errorDescription}>{error}</p>
          <a href="/">
            <Button variant="secondary">Go to dashboard</Button>
          </a>
        </div>
      </div>
    );
  }

  return (
    <div class={styles.container}>
      <div class={styles.card}>
        <Mail size={40} aria-hidden="true" class={styles.icon} />
        <h1 class={styles.heading}>Team invitation</h1>
        {teamName && (
          <p class={styles.teamName}>{teamName}</p>
        )}
        {role && (
          <>
            <p class={styles.roleLabel}>You have been invited as</p>
            <span class={styles.roleBadge}>{role}</span>
          </>
        )}
        {!teamName && (
          <p class={styles.roleLabel}>
            You have been invited to join a team. Accept or decline this invitation.
          </p>
        )}
        <div class={styles.actions}>
          <Button
            variant="secondary"
            onClick={handleDecline}
            loading={declining}
            disabled={accepting}
          >
            Decline
          </Button>
          <Button
            variant="primary"
            onClick={handleAccept}
            loading={accepting}
            disabled={declining}
          >
            Accept invitation
          </Button>
        </div>
      </div>
    </div>
  );
}
