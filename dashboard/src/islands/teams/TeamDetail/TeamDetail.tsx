import { useState, useEffect, useCallback } from 'preact/hooks';
import { ArrowLeft, Users, Calendar, Layers, Shield, UserPlus, Trash2, Mail } from 'lucide-preact';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import Select from '@islands/shared/Select/Select';
import ConfirmDialog from '@islands/shared/ConfirmDialog/ConfirmDialog';
import InviteModal from '@islands/teams/InviteModal/InviteModal';
import type { Team, TeamMember, TeamInvitation } from '@lib/types';
import styles from './team-detail.module.css';

type TabId = 'members' | 'invitations' | 'settings';

const tabs: { id: TabId; label: string }[] = [
  { id: 'members', label: 'Members' },
  { id: 'invitations', label: 'Invitations' },
  { id: 'settings', label: 'Settings' },
];

const roleOptions = [
  { value: 'admin', label: 'Admin' },
  { value: 'member', label: 'Member' },
  { value: 'viewer', label: 'Viewer' },
];

function roleBadgeClass(role: string): string {
  switch (role) {
    case 'owner': return styles.badgeOwner;
    case 'admin': return styles.badgeAdmin;
    case 'member': return styles.badgeMember;
    case 'viewer': return styles.badgeViewer;
    default: return styles.badgeMember;
  }
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}

export default function TeamDetail() {
  const [team, setTeam] = useState<Team | null>(null);
  const [members, setMembers] = useState<TeamMember[]>([]);
  const [resourceCount, setResourceCount] = useState(0);
  const [invitations, setInvitations] = useState<TeamInvitation[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [activeTab, setActiveTab] = useState<TabId>('members');

  // Settings state
  const [editName, setEditName] = useState('');
  const [savingName, setSavingName] = useState(false);

  // Delete state
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [deleteInput, setDeleteInput] = useState('');
  const [deleting, setDeleting] = useState(false);

  // Invite modal
  const [showInvite, setShowInvite] = useState(false);

  // Remove member confirm
  const [removingMember, setRemovingMember] = useState<TeamMember | null>(null);
  const [removeLoading, setRemoveLoading] = useState(false);

  // Role change loading
  const [roleChangeLoading, setRoleChangeLoading] = useState<string | null>(null);

  const teamId = typeof window !== 'undefined'
    ? window.location.pathname.split('/teams/')[1]?.split('/')[0] || ''
    : '';

  const fetchTeam = useCallback(async () => {
    if (!teamId) return;
    try {
      const res = await api.getTeam(teamId);
      setTeam(res.data.team);
      setMembers(res.data.members);
      setResourceCount(res.data.resource_count);
      setEditName(res.data.team.name);
    } catch {
      setError('Failed to load team details.');
      addToast('error', 'Failed to load team details.');
    } finally {
      setLoading(false);
    }
  }, [teamId]);

  const fetchInvitations = useCallback(async () => {
    if (!teamId) return;
    try {
      const res = await api.listTeamInvitations(teamId);
      setInvitations(res.data);
    } catch {
      /* Invitations may not be available — fail silently */
    }
  }, [teamId]);

  useEffect(() => {
    fetchTeam();
    fetchInvitations();
  }, [fetchTeam, fetchInvitations]);

  async function handleSaveName() {
    if (!team || !editName.trim() || editName.trim() === team.name) return;
    setSavingName(true);
    try {
      await api.updateTeam(teamId, { name: editName.trim() });
      addToast('success', 'Team name updated.');
      await fetchTeam();
    } catch {
      addToast('error', 'Failed to update team name.');
    } finally {
      setSavingName(false);
    }
  }

  async function handleDelete() {
    if (!team || deleteInput !== team.name) return;
    setDeleting(true);
    try {
      await api.deleteTeam(teamId);
      addToast('success', `Team "${team.name}" deleted.`);
      window.location.href = '/teams';
    } catch {
      addToast('error', 'Failed to delete team.');
      setDeleting(false);
    }
  }

  async function handleRoleChange(member: TeamMember, newRole: string) {
    setRoleChangeLoading(member.user_id);
    try {
      await api.updateTeamMemberRole(teamId, member.user_id, newRole);
      addToast('success', `${member.email} is now ${newRole}.`);
      await fetchTeam();
    } catch {
      addToast('error', 'Failed to change role.');
    } finally {
      setRoleChangeLoading(null);
    }
  }

  async function handleRemoveMember() {
    if (!removingMember) return;
    setRemoveLoading(true);
    try {
      await api.removeTeamMember(teamId, removingMember.user_id);
      addToast('success', `${removingMember.email} removed from team.`);
      setRemovingMember(null);
      await fetchTeam();
    } catch {
      addToast('error', 'Failed to remove member.');
    } finally {
      setRemoveLoading(false);
    }
  }

  async function handleRevokeInvitation(invitation: TeamInvitation) {
    try {
      await api.declineInvitation(invitation.token);
      addToast('success', `Invitation to ${invitation.email} revoked.`);
      await fetchInvitations();
    } catch {
      addToast('error', 'Failed to revoke invitation.');
    }
  }

  function handleTabKeyDown(e: KeyboardEvent) {
    const tabIds = tabs.map(t => t.id);
    const currentIdx = tabIds.indexOf(activeTab);
    let nextIdx = -1;

    if (e.key === 'ArrowRight') {
      nextIdx = currentIdx < tabIds.length - 1 ? currentIdx + 1 : 0;
    } else if (e.key === 'ArrowLeft') {
      nextIdx = currentIdx > 0 ? currentIdx - 1 : tabIds.length - 1;
    } else if (e.key === 'Home') {
      nextIdx = 0;
    } else if (e.key === 'End') {
      nextIdx = tabIds.length - 1;
    }

    if (nextIdx >= 0) {
      e.preventDefault();
      setActiveTab(tabIds[nextIdx]);
      const tabEl = document.getElementById(`tab-${tabIds[nextIdx]}`);
      tabEl?.focus();
    }
  }

  // Determine if current user is owner (first owner found, simplified heuristic)
  const isOwner = team ? members.some(m => m.role === 'owner') : false;

  if (loading) {
    return (
      <div class={styles.container}>
        <p class={styles.loadingState} role="status" aria-live="polite">Loading team...</p>
      </div>
    );
  }

  if (error || !team) {
    return (
      <div class={styles.container}>
        <a href="/teams" class={styles.backLink}>
          <ArrowLeft size={16} aria-hidden="true" />
          Back to teams
        </a>
        <p class={styles.errorState} role="alert">{error || 'Team not found.'}</p>
      </div>
    );
  }

  return (
    <div class={styles.container}>
      <a href="/teams" class={styles.backLink}>
        <ArrowLeft size={16} aria-hidden="true" />
        Back to teams
      </a>

      <div class={styles.header}>
        <div class={styles.headerTop}>
          <h1 class={styles.teamName}>{team.name}</h1>
          <span class={`${styles.badge} ${styles.badgeOwner}`}>
            <Shield size={12} aria-hidden="true" />
            &nbsp;Owner
          </span>
        </div>
        <div class={styles.teamSlug}>{team.slug}</div>
        <div class={styles.headerMeta}>
          <span class={styles.headerMetaItem}>
            <Users size={14} aria-hidden="true" />
            {members.length} {members.length === 1 ? 'member' : 'members'}
          </span>
          <span class={styles.headerMetaItem}>
            <Layers size={14} aria-hidden="true" />
            {resourceCount} {resourceCount === 1 ? 'resource' : 'resources'}
          </span>
          <span class={styles.headerMetaItem}>
            <Calendar size={14} aria-hidden="true" />
            Created {formatDate(team.created_at)}
          </span>
        </div>
      </div>

      {/* a11y [WCAG 4.1.2]: tab pattern with role="tablist", role="tab", role="tabpanel" */}
      <div class={styles.tabBar} role="tablist" aria-label="Team sections" onKeyDown={handleTabKeyDown}>
        {tabs.map((tab) => (
          <button
            key={tab.id}
            id={`tab-${tab.id}`}
            role="tab"
            type="button"
            aria-selected={activeTab === tab.id}
            aria-controls={`panel-${tab.id}`}
            tabIndex={activeTab === tab.id ? 0 : -1}
            class={`${styles.tab} ${activeTab === tab.id ? styles.tabActive : ''}`}
            onClick={() => setActiveTab(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Members panel */}
      <div
        id="panel-members"
        role="tabpanel"
        aria-labelledby="tab-members"
        class={styles.panel}
        hidden={activeTab !== 'members'}
      >
        {members.length === 0 ? (
          <p class={styles.emptyText}>No members found.</p>
        ) : (
          <div class={styles.memberTable} role="table" aria-label="Team members">
            <div class={styles.memberHeader} role="row">
              <span class={styles.memberHeaderCell} role="columnheader">Email</span>
              <span class={styles.memberHeaderCell} role="columnheader">Role</span>
              <span class={styles.memberHeaderCell} role="columnheader">Joined</span>
              <span class={styles.memberHeaderCell} role="columnheader">Actions</span>
            </div>
            {members.map((member) => (
              <div key={member.id} class={styles.memberRow} role="row">
                <div class={styles.memberEmail} role="cell">{member.email}</div>
                <div class={styles.memberRole} role="cell">
                  <span class={`${styles.badge} ${roleBadgeClass(member.role)}`}>
                    {member.role}
                  </span>
                </div>
                <div class={styles.memberDate} role="cell">
                  {member.accepted_at ? formatDate(member.accepted_at) : formatDate(member.created_at)}
                </div>
                <div class={styles.memberActions} role="cell">
                  {member.role !== 'owner' && (
                    <>
                      <Select
                        options={roleOptions}
                        value={member.role}
                        onChange={(val) => handleRoleChange(member, val)}
                        size="sm"
                        id={`role-${member.user_id}`}
                        aria-label={`Change role for ${member.email}`}
                        disabled={roleChangeLoading === member.user_id}
                      />
                      <Button
                        variant="danger"
                        size="sm"
                        onClick={() => setRemovingMember(member)}
                        aria-label={`Remove ${member.email} from team`}
                      >
                        <Trash2 size={14} aria-hidden="true" />
                      </Button>
                    </>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Invitations panel */}
      <div
        id="panel-invitations"
        role="tabpanel"
        aria-labelledby="tab-invitations"
        class={styles.panel}
        hidden={activeTab !== 'invitations'}
      >
        <div class={styles.inviteHeader}>
          <span class={styles.inviteHeaderTitle}>Pending invitations</span>
          <Button size="sm" onClick={() => setShowInvite(true)}>
            <UserPlus size={14} aria-hidden="true" />
            Invite member
          </Button>
        </div>

        {invitations.length === 0 ? (
          <p class={styles.emptyText}>No pending invitations.</p>
        ) : (
          <div class={styles.invitationList}>
            {invitations.map((inv) => (
              <div key={inv.id} class={styles.invitationRow}>
                <div class={styles.invitationInfo}>
                  <span class={styles.invitationEmail}>
                    <Mail size={14} aria-hidden="true" />
                    &nbsp;{inv.email}
                  </span>
                  <span class={styles.invitationMeta}>
                    <span class={`${styles.badge} ${roleBadgeClass(inv.role)}`}>{inv.role}</span>
                    <span>Expires {formatDate(inv.expires_at)}</span>
                  </span>
                </div>
                <div class={styles.invitationActions}>
                  <Button
                    variant="danger"
                    size="sm"
                    onClick={() => handleRevokeInvitation(inv)}
                    aria-label={`Revoke invitation for ${inv.email}`}
                  >
                    Revoke
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Settings panel */}
      <div
        id="panel-settings"
        role="tabpanel"
        aria-labelledby="tab-settings"
        class={styles.panel}
        hidden={activeTab !== 'settings'}
      >
        <div class={styles.settingsSection}>
          <h2 class={styles.settingsSectionTitle}>General</h2>
          <div class={styles.settingsRow}>
            <Input
              label="Team name"
              name="team-name-edit"
              value={editName}
              onChange={setEditName}
            />
            <Button
              variant="primary"
              onClick={handleSaveName}
              loading={savingName}
              disabled={!editName.trim() || editName.trim() === team.name}
            >
              Save name
            </Button>
          </div>
        </div>

        {isOwner && (
          <div class={`${styles.settingsSection} ${styles.dangerZone}`}>
            <h2 class={styles.dangerTitle}>Danger zone</h2>
            <p class={styles.dangerDescription}>
              Deleting this team will remove all member associations and cannot be undone.
              Type the team name to confirm.
            </p>
            <div class={styles.deleteConfirmInput}>
              <Input
                label={`Type "${team.name}" to confirm`}
                name="delete-confirm"
                value={deleteInput}
                onChange={setDeleteInput}
                placeholder={team.name}
              />
            </div>
            <Button
              variant="danger"
              onClick={() => setShowDeleteConfirm(true)}
              disabled={deleteInput !== team.name}
            >
              <Trash2 size={14} aria-hidden="true" />
              Delete team
            </Button>
          </div>
        )}
      </div>

      {/* Confirm delete dialog */}
      <ConfirmDialog
        open={showDeleteConfirm}
        title={`Delete "${team.name}"?`}
        description="This action is permanent. All members will lose access and resources will be disassociated."
        confirmLabel="Delete team"
        variant="danger"
        loading={deleting}
        onConfirm={handleDelete}
        onCancel={() => { setShowDeleteConfirm(false); setDeleting(false); }}
      />

      {/* Confirm remove member dialog */}
      <ConfirmDialog
        open={!!removingMember}
        title="Remove team member?"
        description={`${removingMember?.email || 'This member'} will lose access to all team resources.`}
        confirmLabel="Remove member"
        variant="danger"
        loading={removeLoading}
        onConfirm={handleRemoveMember}
        onCancel={() => { setRemovingMember(null); setRemoveLoading(false); }}
      />

      {/* Invite modal */}
      <InviteModal
        teamId={teamId}
        open={showInvite}
        onClose={() => setShowInvite(false)}
        onInvited={fetchInvitations}
      />
    </div>
  );
}
