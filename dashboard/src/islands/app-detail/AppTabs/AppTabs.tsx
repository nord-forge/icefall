import { useCallback } from 'preact/hooks';
import styles from './app-tabs.module.css';

type Props = {
  appId: string;
  activeTab: string;
  onTabChange: (tab: string) => void;
}

const TABS = [
  { key: 'overview', label: 'Overview' },
  { key: 'deploys', label: 'Deploys' },
  { key: 'instances', label: 'Instances' },
  { key: 'logs', label: 'Logs' },
  { key: 'env', label: 'Env Vars' },
  { key: 'databases', label: 'Databases' },
  { key: 'terminal', label: 'Terminal' },
  { key: 'domains', label: 'Domains' },
  { key: 'tasks', label: 'Tasks' },
  { key: 'proxy', label: 'Proxy' },
  { key: 'history', label: 'History' },
  { key: 'settings', label: 'Settings' },
];

const TAB_PRELOADERS: Record<string, () => void> = {
  overview: () => { import('@islands/app-detail/OverviewTab/OverviewTab'); },
  deploys: () => { import('@islands/app-detail/DeploysTab/DeploysTab'); },
  instances: () => { import('@islands/app-detail/InstancesTab/InstancesTab'); },
  logs: () => { import('@islands/logs/LogViewer/LogViewer'); },
  env: () => { import('@islands/env-vars/EnvVarEditor/EnvVarEditor'); },
  databases: () => { import('@islands/app-detail/DatabaseTab/DatabaseTab'); },
  terminal: () => { import('@islands/app-detail/TerminalTab/TerminalTab'); },
  domains: () => { import('@islands/app-detail/DomainsTab/DomainsTab'); },
  tasks: () => { import('@islands/app-detail/TasksTab/TasksTab'); },
  proxy: () => { import('@islands/app-detail/ProxyTab/ProxyTab'); },
  history: () => { import('@islands/app-detail/HistoryTab/HistoryTab'); },
  settings: () => { import('@islands/app-detail/SettingsTab/SettingsTab'); },
};

const preloaded = new Set<string>();

function preloadTab(key: string) {
  if (preloaded.has(key)) return;
  preloaded.add(key);
  TAB_PRELOADERS[key]?.();
}

export default function AppTabs({ appId, activeTab, onTabChange }: Props) {
  const handlePreload = useCallback((key: string) => () => preloadTab(key), []);

  return (
    <div role="tablist" aria-label="App sections" class={styles.nav}>
      {TABS.map((tab) => {
        const isActive = tab.key === activeTab;
        return (
          <button
            key={tab.key}
            type="button"
            role="tab"
            id={`tab-${tab.key}`}
            aria-selected={isActive}
            aria-controls={`tabpanel-${tab.key}`}
            tabIndex={isActive ? 0 : -1}
            onClick={() => onTabChange(tab.key)}
            onMouseEnter={handlePreload(tab.key)}
            onFocus={handlePreload(tab.key)}
            class={isActive ? styles.tabActive : styles.tab}
          >
            {tab.label}
          </button>
        );
      })}
    </div>
  );
}
