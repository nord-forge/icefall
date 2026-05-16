import { useState, useEffect, useRef, useCallback } from 'preact/hooks';
import { Users, ChevronDown, Check, Plus } from 'lucide-preact';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import type { Team } from '@lib/types';
import styles from './team-switcher.module.css';

export default function TeamSwitcher() {
  const [teams, setTeams] = useState<Team[]>([]);
  const [loading, setLoading] = useState(true);
  const [open, setOpen] = useState(false);
  const [activeTeamId, setActiveTeamId] = useState<string | null>(null);
  const [focusIdx, setFocusIdx] = useState(-1);
  const [switching, setSwitching] = useState(false);
  const wrapperRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  useEffect(() => {
    async function load() {
      try {
        const res = await api.listTeams();
        setTeams(res.data);
        if (res.data.length > 0) {
          setActiveTeamId(res.data[0].id);
        }
      } catch {
        /* Fail silently — switcher is non-critical */
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    function handleOutside(e: MouseEvent) {
      if (!wrapperRef.current?.contains(e.target as Node)) {
        setOpen(false);
        setFocusIdx(-1);
      }
    }
    document.addEventListener('mousedown', handleOutside);
    return () => document.removeEventListener('mousedown', handleOutside);
  }, [open]);

  // Scroll focused option into view
  useEffect(() => {
    if (!open || focusIdx < 0) return;
    const items = listRef.current?.querySelectorAll<HTMLLIElement>('[role="option"]');
    items?.[focusIdx]?.scrollIntoView({ block: 'nearest' });
  }, [focusIdx, open]);

  const close = useCallback(() => {
    setOpen(false);
    setFocusIdx(-1);
    triggerRef.current?.focus();
  }, []);

  async function handleSelect(team: Team) {
    if (team.id === activeTeamId) {
      close();
      return;
    }
    setSwitching(true);
    try {
      await api.switchTeam(team.id);
      setActiveTeamId(team.id);
      close();
      window.location.reload();
    } catch {
      addToast('error', 'Failed to switch team.');
      setSwitching(false);
    }
  }

  function handleTriggerClick() {
    if (open) {
      close();
    } else {
      setOpen(true);
      const idx = teams.findIndex(t => t.id === activeTeamId);
      setFocusIdx(idx >= 0 ? idx : 0);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (!open) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        setOpen(true);
        const idx = teams.findIndex(t => t.id === activeTeamId);
        setFocusIdx(idx >= 0 ? idx : 0);
      }
      return;
    }

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setFocusIdx(i => (i < teams.length - 1 ? i + 1 : 0));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setFocusIdx(i => (i > 0 ? i - 1 : teams.length - 1));
        break;
      case 'Home':
        e.preventDefault();
        setFocusIdx(0);
        break;
      case 'End':
        e.preventDefault();
        setFocusIdx(teams.length - 1);
        break;
      case 'Enter':
      case ' ':
        e.preventDefault();
        if (focusIdx >= 0 && focusIdx < teams.length) {
          handleSelect(teams[focusIdx]);
        }
        break;
      case 'Escape':
        e.preventDefault();
        close();
        break;
      case 'Tab':
        close();
        break;
    }
  }

  if (loading || teams.length === 0) {
    return null;
  }

  const activeTeam = teams.find(t => t.id === activeTeamId);
  const listboxId = 'team-switcher-listbox';

  return (
    <div class={styles.wrapper} ref={wrapperRef} onKeyDown={handleKeyDown}>
      {/* a11y [WCAG 4.1.2]: listbox pattern with aria-expanded, activedescendant */}
      <button
        ref={triggerRef}
        type="button"
        class={styles.trigger}
        role="combobox"
        aria-expanded={open}
        aria-haspopup="listbox"
        aria-controls={listboxId}
        aria-activedescendant={open && focusIdx >= 0 ? `team-opt-${focusIdx}` : undefined}
        aria-label="Switch team"
        onClick={handleTriggerClick}
        disabled={switching}
      >
        <span class={styles.triggerContent}>
          <Users size={16} aria-hidden="true" class={styles.teamIcon} />
          <span class={styles.teamName}>{activeTeam?.name || 'Select team'}</span>
        </span>
        <ChevronDown
          size={14}
          aria-hidden="true"
          class={`${styles.chevron} ${open ? styles.chevronOpen : ''}`}
        />
      </button>

      {open && (
        <ul
          ref={listRef}
          id={listboxId}
          role="listbox"
          aria-label="Available teams"
          class={styles.dropdown}
          tabIndex={-1}
        >
          {teams.map((team, i) => (
            <li
              key={team.id}
              id={`team-opt-${i}`}
              role="option"
              aria-selected={team.id === activeTeamId}
              class={`${styles.option} ${i === focusIdx ? styles.optionActive : ''}`}
              onClick={() => handleSelect(team)}
              onMouseEnter={() => setFocusIdx(i)}
            >
              <span class={styles.optionName}>{team.name}</span>
              {team.id === activeTeamId && (
                <Check size={14} aria-hidden="true" class={styles.optionCheck} />
              )}
            </li>
          ))}
          <a href="/teams" class={styles.createLink} onClick={() => close()}>
            <Plus size={14} aria-hidden="true" />
            Create team
          </a>
        </ul>
      )}
    </div>
  );
}
