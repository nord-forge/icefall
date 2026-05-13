import { useState, useEffect, useRef, useCallback, useMemo } from 'preact/hooks';
import { Search, LayoutGrid, Database, Server, Users, Settings, Globe, Rocket, RotateCw, Square, FileText, Clock, Command, ArrowUp, ArrowDown, CornerDownLeft, Plus } from 'lucide-preact';
import { api } from '@lib/api';
import type { App } from '@lib/types';
import styles from './command-palette.module.css';

/* ------------------------------------------------------------------ */
/*  Types                                                              */
/* ------------------------------------------------------------------ */

type ResultGroup = 'recent' | 'apps' | 'databases' | 'pages' | 'actions';

type PaletteItem = {
  id: string;
  group: ResultGroup;
  label: string;
  meta?: string;
  href?: string;
  action?: () => void | Promise<void>;
  /** If true, show a confirmation before executing the action */
  dangerous?: boolean;
  icon: preact.ComponentType<{ size?: number; 'aria-hidden'?: string }>;
};

type RecentEntry = {
  id: string;
  label: string;
  href: string;
  group: ResultGroup;
};

/* ------------------------------------------------------------------ */
/*  Constants                                                          */
/* ------------------------------------------------------------------ */

const RECENT_KEY = 'icefall_recent';
const MAX_RECENT = 5;
const MAX_RESULTS = 10;

const ICON_MAP: Record<ResultGroup, preact.ComponentType<{ size?: number; 'aria-hidden'?: string }>> = {
  recent: Clock,
  apps: Rocket,
  databases: Database,
  pages: Globe,
  actions: Command,
};

const GROUP_LABELS: Record<ResultGroup, string> = {
  recent: 'Recent',
  apps: 'Apps',
  databases: 'Databases',
  pages: 'Pages',
  actions: 'Actions',
};

const STATIC_PAGES: PaletteItem[] = [
  { id: 'page-home', group: 'pages', label: 'Dashboard Home', href: '/', icon: LayoutGrid },
  { id: 'page-databases', group: 'pages', label: 'Databases', href: '/databases', icon: Database },
  { id: 'page-server', group: 'pages', label: 'Server', href: '/server', icon: Server },
  { id: 'page-users', group: 'pages', label: 'Users', href: '/users', icon: Users },
  { id: 'page-settings', group: 'pages', label: 'Settings', href: '/settings', icon: Settings },
  { id: 'page-domains', group: 'pages', label: 'Domains', href: '/domains', icon: Globe },
];

/* ------------------------------------------------------------------ */
/*  localStorage helpers                                               */
/* ------------------------------------------------------------------ */

function loadRecent(): RecentEntry[] {
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.slice(0, MAX_RECENT) : [];
  } catch {
    return [];
  }
}

function saveRecent(entry: RecentEntry) {
  const list = loadRecent().filter((r) => r.id !== entry.id);
  list.unshift(entry);
  localStorage.setItem(RECENT_KEY, JSON.stringify(list.slice(0, MAX_RECENT)));
}

/* ------------------------------------------------------------------ */
/*  Fuzzy match (case-insensitive substring)                           */
/* ------------------------------------------------------------------ */

function fuzzyMatch(query: string, text: string): boolean {
  return text.toLowerCase().includes(query.toLowerCase());
}

/* ------------------------------------------------------------------ */
/*  Component                                                          */
/* ------------------------------------------------------------------ */

export default function CommandPalette() {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [confirm, setConfirm] = useState<PaletteItem | null>(null);

  /* Cached data */
  const [apps, setApps] = useState<App[]>([]);
  const [databases, setDatabases] = useState<{ id: string; name: string; db_type: string }[]>([]);
  const dataFetched = useRef(false);

  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  /* ---------------------------------------------------------------- */
  /*  Fetch apps + databases once on first open                        */
  /* ---------------------------------------------------------------- */

  const fetchData = useCallback(async () => {
    if (dataFetched.current) return;
    dataFetched.current = true;
    try {
      const [appsRes, dbsRes] = await Promise.allSettled([
        api.listApps(),
        api.listDatabases(),
      ]);
      if (appsRes.status === 'fulfilled') setApps(appsRes.value.data);
      if (dbsRes.status === 'fulfilled') setDatabases(dbsRes.value.data);
    } catch {
      /* silently degrade — palette still works for pages + actions */
    }
  }, []);

  /* ---------------------------------------------------------------- */
  /*  Open / close                                                     */
  /* ---------------------------------------------------------------- */

  const openPalette = useCallback(() => {
    setOpen(true);
    setQuery('');
    setSelectedIndex(0);
    setConfirm(null);
    fetchData();
  }, [fetchData]);

  const closePalette = useCallback(() => {
    setOpen(false);
    setQuery('');
    setConfirm(null);
  }, []);

  /* Global keyboard shortcut: Cmd+K / Ctrl+K */
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        if (open) {
          closePalette();
        } else {
          openPalette();
        }
      }
      if (e.key === 'Escape' && open) {
        e.preventDefault();
        if (confirm) {
          setConfirm(null);
        } else {
          closePalette();
        }
      }
    }
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, confirm, openPalette, closePalette]);

  /* Autofocus input when palette opens */
  useEffect(() => {
    if (open) {
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open]);

  // a11y [WCAG 2.4.3]: focus trap — keep Tab cycling within the palette
  const paletteRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const dialog = paletteRef.current;
    if (!dialog) return;
    const focusable = dialog.querySelectorAll<HTMLElement>(
      'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])'
    );
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    function handleTab(e: KeyboardEvent) {
      if (e.key !== 'Tab') return;
      if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last?.focus(); }
      else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first?.focus(); }
    }
    dialog.addEventListener('keydown', handleTab);
    return () => dialog.removeEventListener('keydown', handleTab);
  }, [open, query, confirm]);

  /* ---------------------------------------------------------------- */
  /*  Determine mode (search vs action)                                */
  /* ---------------------------------------------------------------- */

  const isActionMode = query.startsWith('>');
  const searchTerm = isActionMode ? query.slice(1).trim() : query.trim();

  /* ---------------------------------------------------------------- */
  /*  Build action items from app data                                 */
  /* ---------------------------------------------------------------- */

  const actionItems = useMemo<PaletteItem[]>(() => {
    const items: PaletteItem[] = [];

    for (const app of apps) {
      items.push({
        id: `action-deploy-${app.id}`,
        group: 'actions',
        label: `Deploy ${app.name}`,
        icon: Rocket,
        action: () => { api.triggerDeploy(app.id); },
      });
      items.push({
        id: `action-restart-${app.id}`,
        group: 'actions',
        label: `Restart ${app.name}`,
        icon: RotateCw,
        action: () => { api.restartApp(app.id); },
      });
      items.push({
        id: `action-stop-${app.id}`,
        group: 'actions',
        label: `Stop ${app.name}`,
        icon: Square,
        dangerous: true,
        action: () => { api.stopApp(app.id); },
      });
      items.push({
        id: `action-logs-${app.id}`,
        group: 'actions',
        label: `View logs ${app.name}`,
        icon: FileText,
        href: `/apps/${app.id}/logs`,
      });
    }

    items.push({
      id: 'action-new-app',
      group: 'actions',
      label: 'New app',
      icon: Plus,
      href: '/apps/new',
    });
    items.push({
      id: 'action-new-database',
      group: 'actions',
      label: 'New database',
      icon: Plus,
      href: '/databases?create=1',
    });
    items.push({
      id: 'action-settings',
      group: 'actions',
      label: 'Settings',
      icon: Settings,
      href: '/settings',
    });

    return items;
  }, [apps]);

  /* ---------------------------------------------------------------- */
  /*  Build filtered results                                           */
  /* ---------------------------------------------------------------- */

  const results = useMemo<PaletteItem[]>(() => {
    /* Action mode: only show action items */
    if (isActionMode) {
      if (!searchTerm) return actionItems.slice(0, MAX_RESULTS);
      return actionItems.filter((a) => fuzzyMatch(searchTerm, a.label)).slice(0, MAX_RESULTS);
    }

    /* Empty query: show recent items */
    if (!searchTerm) {
      const recent = loadRecent();
      return recent.map<PaletteItem>((r) => ({
        id: `recent-${r.id}`,
        group: 'recent',
        label: r.label,
        href: r.href,
        icon: ICON_MAP[r.group] || Clock,
      }));
    }

    /* Search across apps, databases, and pages */
    const matched: PaletteItem[] = [];

    for (const app of apps) {
      if (fuzzyMatch(searchTerm, app.name)) {
        matched.push({
          id: `app-${app.id}`,
          group: 'apps',
          label: app.name,
          meta: app.framework || undefined,
          href: `/apps/${app.id}`,
          icon: Rocket,
        });
      }
    }

    for (const db of databases) {
      if (fuzzyMatch(searchTerm, db.name)) {
        matched.push({
          id: `db-${db.id}`,
          group: 'databases',
          label: db.name,
          meta: db.db_type,
          href: `/databases?selected=${db.id}`,
          icon: Database,
        });
      }
    }

    for (const page of STATIC_PAGES) {
      if (fuzzyMatch(searchTerm, page.label)) {
        matched.push(page);
      }
    }

    return matched.slice(0, MAX_RESULTS);
  }, [searchTerm, isActionMode, apps, databases, actionItems]);

  /* ---------------------------------------------------------------- */
  /*  Grouped results for rendering                                    */
  /* ---------------------------------------------------------------- */

  const grouped = useMemo(() => {
    const groups: { group: ResultGroup; label: string; items: PaletteItem[] }[] = [];
    const order: ResultGroup[] = ['recent', 'apps', 'databases', 'pages', 'actions'];
    for (const g of order) {
      const items = results.filter((r) => r.group === g);
      if (items.length > 0) {
        groups.push({ group: g, label: GROUP_LABELS[g], items });
      }
    }
    return groups;
  }, [results]);

  /* ---------------------------------------------------------------- */
  /*  Selection                                                        */
  /* ---------------------------------------------------------------- */

  /* Reset selection when results change */
  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  /* Scroll selected item into view */
  useEffect(() => {
    if (!listRef.current) return;
    const selected = listRef.current.querySelector(`[data-index="${selectedIndex}"]`);
    if (selected) {
      (selected as HTMLElement).scrollIntoView({ block: 'nearest' });
    }
  }, [selectedIndex]);

  /* Execute selected item */
  const executeItem = useCallback((item: PaletteItem) => {
    if (item.dangerous) {
      setConfirm(item);
      return;
    }

    /* Save to recent */
    if (item.href) {
      saveRecent({
        id: item.id.replace(/^recent-/, ''),
        label: item.label,
        href: item.href,
        group: item.group === 'recent' ? 'pages' : item.group,
      });
    }

    if (item.action) {
      item.action();
      closePalette();
    } else if (item.href) {
      closePalette();
      window.location.href = item.href;
    }
  }, [closePalette]);

  const confirmAction = useCallback(() => {
    if (!confirm) return;
    if (confirm.action) {
      confirm.action();
    }
    closePalette();
  }, [confirm, closePalette]);

  /* ---------------------------------------------------------------- */
  /*  Keyboard navigation inside the palette                           */
  /* ---------------------------------------------------------------- */

  function handleInputKeyDown(e: KeyboardEvent) {
    if (confirm) {
      /* In confirm mode, Enter = confirm, Escape handled globally */
      if (e.key === 'Enter') {
        e.preventDefault();
        confirmAction();
      }
      return;
    }

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, results.length - 1));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
        break;
      case 'Enter':
        e.preventDefault();
        if (results[selectedIndex]) {
          executeItem(results[selectedIndex]);
        }
        break;
    }
  }

  /* ---------------------------------------------------------------- */
  /*  Overlay click = close                                            */
  /* ---------------------------------------------------------------- */

  function handleOverlayClick(e: MouseEvent) {
    if ((e.target as HTMLElement).dataset.overlay === 'true') {
      closePalette();
    }
  }

  /* ---------------------------------------------------------------- */
  /*  Render                                                           */
  /* ---------------------------------------------------------------- */

  if (!open) return null;

  /* Flat index counter for keyboard selection mapping */
  let flatIndex = 0;

  return (
    <div
      ref={paletteRef}
      class={styles.overlay}
      data-overlay="true"
      onClick={handleOverlayClick}
      /* a11y [WCAG 2.4.3]: trap intent inside the modal */
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
    >
      <div class={styles.palette} role="combobox" aria-expanded="true" aria-haspopup="listbox">
        {/* Search row */}
        <div class={styles.searchRow}>
          <Search size={18} aria-hidden="true" class={styles.searchIcon} />
          <input
            ref={inputRef}
            type="text"
            class={styles.searchInput}
            placeholder={isActionMode ? 'Type an action...' : 'Search apps, databases, pages...'}
            value={query}
            onInput={(e) => setQuery((e.target as HTMLInputElement).value)}
            onKeyDown={handleInputKeyDown}
            aria-label="Search command palette"
            aria-autocomplete="list"
            aria-controls="command-palette-list"
            aria-activedescendant={results[selectedIndex] ? `cp-item-${results[selectedIndex].id}` : undefined}
          />
          {isActionMode && <span class={styles.actionBadge}>Actions</span>}
        </div>

        {/* Confirm overlay */}
        {confirm ? (
          <div class={styles.confirmOverlay} role="alertdialog" aria-label="Confirm action">
            <p class={styles.confirmText}>
              Are you sure you want to <strong>{confirm.label.toLowerCase()}</strong>?
            </p>
            <div class={styles.confirmActions}>
              <button
                type="button"
                onClick={() => setConfirm(null)}
                class={styles.resultItem}
                style={{ width: 'auto', justifyContent: 'center' }}
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={confirmAction}
                class={styles.resultItem}
                style={{ width: 'auto', justifyContent: 'center', color: 'var(--color-error)', fontWeight: 'var(--weight-semibold)' }}
              >
                Confirm
              </button>
            </div>
          </div>
        ) : (
          /* Results list */
          <div class={styles.results} ref={listRef} id="command-palette-list" role="listbox">
            {results.length === 0 && searchTerm && (
              <div class={styles.empty}>No results for "{searchTerm}"</div>
            )}
            {results.length === 0 && !searchTerm && !isActionMode && (
              <div class={styles.empty}>No recent items. Start typing to search.</div>
            )}
            {grouped.map((group) => (
              <div key={group.group}>
                <div class={styles.groupHeader} role="presentation">{group.label}</div>
                {group.items.map((item) => {
                  const idx = flatIndex++;
                  const Icon = item.icon;
                  const isSelected = idx === selectedIndex;

                  return (
                    <button
                      key={item.id}
                      id={`cp-item-${item.id}`}
                      type="button"
                      role="option"
                      aria-selected={isSelected}
                      data-index={idx}
                      class={`${styles.resultItem} ${isSelected ? styles.resultItemSelected : ''}`}
                      onClick={() => executeItem(item)}
                      onMouseEnter={() => setSelectedIndex(idx)}
                    >
                      <span class={styles.resultIcon} aria-hidden="true">
                        <Icon size={16} aria-hidden="true" />
                      </span>
                      <span class={styles.resultBody}>
                        <span class={styles.resultName}>{item.label}</span>
                        {item.meta && <span class={styles.resultMeta}>{item.meta}</span>}
                      </span>
                      {item.dangerous && <span class={styles.actionBadge} style={{ background: 'oklch(0.94 0.04 25)', color: 'var(--color-error)' }}>Destructive</span>}
                    </button>
                  );
                })}
              </div>
            ))}
          </div>
        )}

        {/* Footer hints */}
        <div class={styles.footer}>
          <span class={styles.footerHint}>
            <ArrowUp size={12} aria-hidden="true" />
            <ArrowDown size={12} aria-hidden="true" />
            <span>navigate</span>
          </span>
          <span class={styles.footerHint}>
            <CornerDownLeft size={12} aria-hidden="true" />
            <span>select</span>
          </span>
          <span class={styles.footerHint}>
            <kbd class={styles.kbd}>esc</kbd>
            <span>close</span>
          </span>
          <span class={styles.footerHint}>
            <kbd class={styles.kbd}>&gt;</kbd>
            <span>actions</span>
          </span>
        </div>
      </div>
    </div>
  );
}
