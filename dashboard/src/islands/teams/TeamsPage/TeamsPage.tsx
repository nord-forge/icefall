import { useState, useEffect, useCallback } from 'preact/hooks';
import { Users, Plus, Calendar } from 'lucide-preact';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import Button from '@islands/shared/Button/Button';
import Input from '@islands/shared/Input/Input';
import type { Team } from '@lib/types';
import styles from './teams-page.module.css';

export default function TeamsPage() {
  const [teams, setTeams] = useState<Team[]>([]);
  const [loading, setLoading] = useState(true);
  const [showCreate, setShowCreate] = useState(false);
  const [newName, setNewName] = useState('');
  const [creating, setCreating] = useState(false);

  const fetchTeams = useCallback(async () => {
    try {
      const res = await api.listTeams();
      setTeams(res.data);
    } catch {
      addToast('error', 'Failed to load teams. Please try again.');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchTeams();
  }, [fetchTeams]);

  async function handleCreate(e: Event) {
    e.preventDefault();
    const trimmed = newName.trim();
    if (!trimmed) return;

    setCreating(true);
    try {
      await api.createTeam(trimmed);
      addToast('success', `Team "${trimmed}" created.`);
      setNewName('');
      setShowCreate(false);
      await fetchTeams();
    } catch {
      addToast('error', 'Failed to create team. Please try again.');
    } finally {
      setCreating(false);
    }
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    });
  }

  if (loading) {
    return (
      <div class={styles.container}>
        <p class={styles.loadingState} role="status" aria-live="polite">Loading teams...</p>
      </div>
    );
  }

  return (
    <div class={styles.container}>
      <div class={styles.header}>
        <h1 class={styles.title}>Teams</h1>
        {!showCreate && (
          <Button onClick={() => setShowCreate(true)}>
            <Plus size={16} aria-hidden="true" />
            Create team
          </Button>
        )}
      </div>

      {showCreate && (
        <form onSubmit={handleCreate} class={styles.createForm}>
          <Input
            label="Team name"
            name="team-name"
            value={newName}
            placeholder="e.g. Engineering"
            required
            onChange={setNewName}
          />
          <div style={{ display: 'flex', gap: 'var(--space-2)', alignSelf: 'flex-end' }}>
            <Button variant="secondary" onClick={() => { setShowCreate(false); setNewName(''); }}>
              Cancel
            </Button>
            <Button variant="primary" type="submit" loading={creating} disabled={!newName.trim()}>
              Create team
            </Button>
          </div>
        </form>
      )}

      {teams.length === 0 ? (
        <div class={styles.emptyState}>
          <Users size={40} aria-hidden="true" class={styles.emptyIcon} />
          <p class={styles.emptyTitle}>No teams yet</p>
          <p class={styles.emptyDescription}>
            Create a team to collaborate on projects and manage shared resources.
          </p>
          {!showCreate && (
            <Button onClick={() => setShowCreate(true)}>
              <Plus size={16} aria-hidden="true" />
              Create your first team
            </Button>
          )}
        </div>
      ) : (
        <div class={styles.grid} role="list">
          {teams.map((team) => (
            <a
              key={team.id}
              href={`/teams/${team.id}`}
              class={styles.card}
              role="listitem"
            >
              <div class={styles.cardName}>{team.name}</div>
              <div class={styles.cardSlug}>{team.slug}</div>
              <div class={styles.cardMeta}>
                <span class={styles.cardMetaItem}>
                  <Calendar size={14} aria-hidden="true" />
                  {formatDate(team.created_at)}
                </span>
              </div>
            </a>
          ))}
        </div>
      )}
    </div>
  );
}
