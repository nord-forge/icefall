import { useState, useEffect } from 'preact/hooks';
import type { GitHubInstallation, GitHubRepo } from '@lib/types';
import { api } from '@lib/api';
import { addToast } from '@stores/toast';
import { GitBranch, Lock, Search } from 'lucide-preact';
import styles from './repo-browser.module.css';

type Props = {
  onSelect: (repo: string, branch: string) => void;
};

type SourceWithRepos = {
  source: GitHubInstallation;
  repos: GitHubRepo[];
  loading: boolean;
};

export default function RepoBrowser({ onSelect }: Props) {
  const [sources, setSources] = useState<SourceWithRepos[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState('');

  useEffect(() => {
    api.listGitSources()
      .then(({ data }) => {
        const withRepos = data.map((source) => ({
          source,
          repos: [] as GitHubRepo[],
          loading: true,
        }));
        setSources(withRepos);
        setLoading(false);

        data.forEach((source, index) => {
          api.listGitSourceRepos(source.id)
            .then(({ data: repos }) => {
              setSources((prev) =>
                prev.map((s, i) =>
                  i === index ? { ...s, repos, loading: false } : s
                )
              );
            })
            .catch(() => {
              setSources((prev) =>
                prev.map((s, i) =>
                  i === index ? { ...s, loading: false } : s
                )
              );
            });
        });
      })
      .catch(() => {
        addToast('error', 'Failed to load Git sources');
        setLoading(false);
      });
  }, []);

  const query = searchQuery.toLowerCase().trim();

  const filteredSources = sources.map((s) => ({
    ...s,
    repos: query
      ? s.repos.filter((r) => r.full_name.toLowerCase().includes(query))
      : s.repos,
  }));

  const totalRepos = filteredSources.reduce((sum, s) => sum + s.repos.length, 0);

  if (loading) {
    return (
      <div class={styles.container}>
        <p class={styles.loadingText}>Loading repositories...</p>
      </div>
    );
  }

  if (sources.length === 0) {
    return (
      <div class={styles.container}>
        <p class={styles.emptyText}>
          No Git sources connected. Add a GitHub integration in platform settings first.
        </p>
      </div>
    );
  }

  return (
    <div class={styles.container}>
      <div style={{ position: 'relative' }}>
        <Search
          size={14}
          aria-hidden="true"
          style={{
            position: 'absolute',
            left: 'var(--space-3)',
            top: '50%',
            transform: 'translateY(-50%)',
            color: 'var(--color-text-muted)',
          }}
        />
        <label htmlFor="repo-search" class="sr-only">Search repositories</label>
        <input
          id="repo-search"
          type="search"
          class={styles.searchInput}
          style={{ paddingLeft: 'var(--space-8)' }}
          value={searchQuery}
          onInput={(e) => setSearchQuery((e.target as HTMLInputElement).value)}
          placeholder="Search repositories..."
        />
      </div>

      {filteredSources.map((s) => {
        if (s.repos.length === 0 && !s.loading) return null;

        return (
          <div key={s.source.id} class={styles.sourceGroup}>
            <span class={styles.sourceLabel}>
              {s.source.account_name}
              {s.source.account_type === 'organization' ? ' (org)' : ''}
            </span>

            {s.loading ? (
              <p class={styles.loadingText}>Loading repos...</p>
            ) : (
              <div class={styles.repoList} role="listbox" aria-label={`Repositories from ${s.source.account_name}`}>
                {s.repos.map((repo) => (
                  <button
                    key={repo.id}
                    type="button"
                    class={styles.repoRow}
                    role="option"
                    aria-selected={false}
                    onClick={() => onSelect(repo.full_name, repo.default_branch)}
                  >
                    <GitBranch size={14} aria-hidden="true" />
                    <div class={styles.repoInfo}>
                      <span class={styles.repoName}>{repo.full_name}</span>
                      <span class={styles.repoBranch}>{repo.default_branch}</span>
                    </div>
                    {repo.private && (
                      <span class={styles.privateBadge}>
                        <Lock size={10} aria-hidden="true" style={{ marginRight: '2px' }} /> private
                      </span>
                    )}
                  </button>
                ))}
              </div>
            )}
          </div>
        );
      })}

      {!loading && totalRepos === 0 && query && (
        <p class={styles.emptyText}>
          No repositories match "{searchQuery}". Try a different search term.
        </p>
      )}

      {/* a11y [WCAG 4.1.3]: announce result count */}
      <div role="status" aria-live="polite" class="sr-only">
        {query ? `${totalRepos} ${totalRepos === 1 ? 'repository' : 'repositories'} found` : ''}
      </div>
    </div>
  );
}
