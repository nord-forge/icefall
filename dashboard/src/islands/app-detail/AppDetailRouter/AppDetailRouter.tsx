import { useEffect, useState, useRef, useCallback } from 'preact/hooks';
import { api } from '@lib/api';
import type { App, DeployStatus } from '@lib/types';
import type { ComponentType } from 'preact';
import AppHeader from '@islands/app-detail/AppHeader/AppHeader';
import AppTabs from '@islands/app-detail/AppTabs/AppTabs';
import styles from './app-detail-router.module.css';

function parseRoute() {
  const path = window.location.pathname.replace('/apps/', '');
  const segments = path.split('/').filter(Boolean);
  if (segments.length === 0) return { appId: '', tab: 'overview', subId: '' };
  return { appId: segments[0], tab: segments[1] || 'overview', subId: segments[2] || '' };
}

const TAB_LOADERS: Record<string, () => Promise<{ default: ComponentType<any> }>> = {
  overview: () => import('@islands/app-detail/OverviewTab/OverviewTab'),
  deploys: () => import('@islands/app-detail/DeploysTab/DeploysTab'),
  instances: () => import('@islands/app-detail/InstancesTab/InstancesTab'),
  logs: () => import('@islands/logs/LogViewer/LogViewer'),
  env: () => import('@islands/env-vars/EnvVarEditor/EnvVarEditor'),
  databases: () => import('@islands/app-detail/DatabaseTab/DatabaseTab'),
  terminal: () => import('@islands/app-detail/TerminalTab/TerminalTab'),
  domains: () => import('@islands/app-detail/DomainsTab/DomainsTab'),
  tasks: () => import('@islands/app-detail/TasksTab/TasksTab'),
  proxy: () => import('@islands/app-detail/ProxyTab/ProxyTab'),
  history: () => import('@islands/app-detail/HistoryTab/HistoryTab'),
  settings: () => import('@islands/app-detail/SettingsTab/SettingsTab'),
};

const VALID_TABS = new Set(Object.keys(TAB_LOADERS));

export default function AppDetailRouter() {
  const [route, setRoute] = useState(parseRoute);
  const [app, setApp] = useState<App | null>(null);
  const [appStatus, setAppStatus] = useState<DeployStatus | 'online'>('stopped');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  const componentCache = useRef<Record<string, ComponentType<any>>>({});
  const [loadedTabs, setLoadedTabs] = useState<Record<string, ComponentType<any>>>({});
  const [DeployDetail, setDeployDetail] = useState<ComponentType<any> | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Tab fade transition state
  const [tabTransition, setTabTransition] = useState<'visible' | 'entering'>('visible');
  const prevTabRef = useRef(route.tab);

  // Scroll position preservation per tab
  const scrollPositions = useRef<Record<string, number>>({});

  const { appId, tab: activeTab, subId } = route;

  // Central navigation function — all internal links go through this
  const navigate = useCallback((path: string, replace = false) => {
    // Save scroll position before navigating
    const currentTab = parseRoute().tab;
    scrollPositions.current[currentTab] = window.scrollY;

    if (replace) {
      window.history.replaceState(null, '', path);
    } else {
      window.history.pushState(null, '', path);
    }
    setRoute(parseRoute());
  }, []);

  // Sync state with URL on browser back/forward
  useEffect(() => {
    function onPopState() {
      const currentTab = parseRoute().tab;
      scrollPositions.current[activeTab] = window.scrollY;
      setRoute(parseRoute());
    }
    window.addEventListener('popstate', onPopState);
    return () => window.removeEventListener('popstate', onPopState);
  }, [activeTab]);

  // Intercept <a> clicks inside this island that point to /apps/{appId}/...
  // This prevents full page reloads for internal navigation
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    function handleClick(e: MouseEvent) {
      if (e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;

      const link = (e.target as HTMLElement).closest('a');
      if (!link) return;

      const href = link.getAttribute('href');
      if (!href || !href.startsWith(`/apps/${appId}`)) return;

      // This is an internal app link — handle it as SPA navigation
      e.preventDefault();
      e.stopPropagation();
      navigate(href);
    }

    container.addEventListener('click', handleClick);
    return () => container.removeEventListener('click', handleClick);
  }, [appId, navigate]);

  // Load app data
  const refreshStatus = useCallback(() => {
    if (!appId) return;
    api.listDeploys(appId).then(({ data }) => {
      const latest = data[0];
      setAppStatus(latest?.status === 'running' ? 'running' : latest?.status || 'stopped');
    }).catch(() => {});
  }, [appId]);

  useEffect(() => {
    if (!appId) return;
    api.getApp(appId)
      .then(({ data }) => { setApp(data); setLoading(false); })
      .catch((err) => { setError(err.message); setLoading(false); });
    refreshStatus();
  }, [appId, refreshStatus]);

  // Lazy-load tab component with fade transition
  useEffect(() => {
    const tabChanged = prevTabRef.current !== activeTab;
    prevTabRef.current = activeTab;

    if (tabChanged) {
      // Start fade-out briefly, then swap content
      setTabTransition('entering');
      const timer = setTimeout(() => {
        setTabTransition('visible');
      }, 30);

      // Restore scroll position after a micro delay
      const scrollTimer = setTimeout(() => {
        const saved = scrollPositions.current[activeTab];
        if (saved != null) {
          window.scrollTo(0, saved);
        }
      }, 60);

      return () => {
        clearTimeout(timer);
        clearTimeout(scrollTimer);
      };
    }
  }, [activeTab]);

  useEffect(() => {
    if (componentCache.current[activeTab]) {
      setLoadedTabs(prev => ({ ...prev, [activeTab]: componentCache.current[activeTab] }));
      return;
    }

    const loader = TAB_LOADERS[activeTab];
    if (loader) {
      loader().then((m) => {
        componentCache.current[activeTab] = m.default;
        setLoadedTabs(prev => ({ ...prev, [activeTab]: m.default }));
      });
    }
  }, [activeTab]);

  // Load deploy detail component when needed
  useEffect(() => {
    if (activeTab === 'deploys' && subId && !DeployDetail) {
      import('@islands/deploy/DeployDetail/DeployDetail').then((m) => {
        setDeployDetail(() => m.default);
      });
    }
  }, [activeTab, subId, DeployDetail]);

  if (loading) {
    return <div class={styles.loading}>Loading...</div>;
  }

  if (error || !app) {
    return <div class={styles.error} role="alert">{error || 'App not found'}</div>;
  }

  // Deploy detail sub-view with slide-in animation
  if (activeTab === 'deploys' && subId && DeployDetail) {
    return (
      <div ref={containerRef} class={styles.deploySlide}>
        <DeployDetail appId={app.id} deployId={subId} appName={app.name} />
      </div>
    );
  }

  const TabComponent = loadedTabs[activeTab] || null;
  const tabProps = activeTab === 'overview' || activeTab === 'settings' ? { app } : { appId: app.id };

  const panelClass = `${styles.tabPanel} ${tabTransition === 'entering' ? styles.tabPanelEntering : styles.tabPanelVisible}`;

  return (
    <div ref={containerRef}>
      <AppHeader app={app} status={appStatus} onStatusChange={refreshStatus} />
      <AppTabs appId={app.id} activeTab={activeTab} onTabChange={(tab) => {
        navigate(`/apps/${app.id}/${tab}`);
      }} />
      <div class={styles.tabContent}>
        <div class={panelClass}>
          {TabComponent ? (
            <TabComponent key={activeTab} {...tabProps} />
          ) : (
            <div class={styles.tabLoading}>Loading...</div>
          )}
        </div>
      </div>
    </div>
  );
}
