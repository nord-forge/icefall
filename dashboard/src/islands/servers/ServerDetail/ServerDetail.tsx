import { useEffect, useState, useMemo } from 'preact/hooks';
import { api } from '@lib/api';
import { createVisibleInterval } from '@lib/visibility';
import { createSSEClient } from '@lib/sse';
import type { App, Server, ServerResources, ServerMetricsSnapshot } from '@lib/types';
import { formatBytes, formatPercent, formatRelativeTime } from '@lib/format';
import StatusDot from '@islands/shared/StatusDot/StatusDot';
import ProgressBar from '@islands/shared/ProgressBar/ProgressBar';
import MetricsChart from '@islands/shared/MetricsChart/MetricsChart';
import Button from '@islands/shared/Button/Button';
import AppCard from '@islands/dashboard/AppCard/AppCard';
import ConfirmDialog from '@islands/shared/ConfirmDialog/ConfirmDialog';
import Skeleton from '@islands/shared/Skeleton/Skeleton';
import { Cpu, MemoryStick, HardDrive, Server as ServerIcon, Globe, Hash, Clock, Trash2, Unplug } from 'lucide-preact';
import { addToast } from '@stores/toast';
import Input from '@islands/shared/Input/Input';
import ForecastSection from './components/ForecastSection';
import InstancesSection from './components/InstancesSection';
import ServerCleanupCard from './components/ServerCleanupCard';
import styles from './server-detail.module.css';

function getServerIdFromUrl(): string {
  const segments = window.location.pathname.replace('/servers/', '').split('/').filter(Boolean);
  return segments[0] || '';
}

type Tab = 'overview' | 'apps' | 'metrics' | 'settings';

const TABS: { key: Tab; label: string }[] = [
  { key: 'overview', label: 'Overview' },
  { key: 'apps', label: 'Apps' },
  { key: 'metrics', label: 'Metrics' },
  { key: 'settings', label: 'Settings' },
];

function parseResources(raw: string | null): ServerResources | null {
  if (!raw) return null;
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

export default function ServerDetail() {
  const serverId = getServerIdFromUrl();
  const [server, setServer] = useState<Server | null>(null);
  const [apps, setApps] = useState<App[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<Tab>('overview');
  const [metricsHistory, setMetricsHistory] = useState<ServerMetricsSnapshot[]>([]);
  const [metricsRange, setMetricsRange] = useState<'1h' | '6h' | '24h' | '7d'>('1h');
  const [localStatus, setLocalStatus] = useState<ServerResources | null>(null);

  // Settings state
  const [editName, setEditName] = useState('');
  const [editLabels, setEditLabels] = useState('');
  const [saving, setSaving] = useState(false);

  // Danger zone
  const [showDisconnect, setShowDisconnect] = useState(false);
  const [showForceRemove, setShowForceRemove] = useState(false);
  const [confirmName, setConfirmName] = useState('');
  const [deleting, setDeleting] = useState(false);

  const isControlPlane = server?.role === 'control-plane';

  useEffect(() => {
    let active = true;

    async function load() {
      try {
        const [serverRes, appsRes] = await Promise.all([
          api.getServer(serverId),
          api.listApps(),
        ]);
        if (!active) return;
        setServer(serverRes.data);
        setEditName(serverRes.data.name);
        setEditLabels(serverRes.data.labels || '');
        setApps(appsRes.data.filter((a) => a.server_id === serverId));

        if (serverRes.data.role === 'control-plane') {
          try {
            const status = await api.getServerStatus();
            if (active) {
              setLocalStatus({
                cpu_percent: status.cpu_percent,
                cpu_cores: 1,
                ram_used_bytes: status.memory_used_bytes,
                ram_total_bytes: status.memory_total_bytes,
                disk_used_bytes: status.disk_used_bytes,
                disk_total_bytes: status.disk_total_bytes,
                load_average: [],
              });
            }
          } catch {}
        }
      } catch {}
      if (active) setLoading(false);
    }

    load();

    const sse = createSSEClient('/api/v1/events', {
      'server.connected': (data: any) => {
        if (data.server_id === serverId) {
          setServer((s) => s ? { ...s, status: 'online' } : s);
        }
      },
      'server.disconnected': (data: any) => {
        if (data.server_id === serverId) {
          setServer((s) => s ? { ...s, status: 'offline' } : s);
        }
      },
    });

    const stopMetricsPolling = createVisibleInterval(async () => {
      try {
        const res = await api.getServer(serverId);
        if (active) setServer(res.data);
        if (res.data.role === 'control-plane') {
          const status = await api.getServerStatus();
          if (active) {
            setLocalStatus({
              cpu_percent: status.cpu_percent,
              cpu_cores: 1,
              ram_used_bytes: status.memory_used_bytes,
              ram_total_bytes: status.memory_total_bytes,
              disk_used_bytes: status.disk_used_bytes,
              disk_total_bytes: status.disk_total_bytes,
              load_average: [],
            });
          }
        }
      } catch {}
    }, 10_000);

    return () => {
      active = false;
      sse.close();
      stopMetricsPolling();
    };
  }, [serverId]);

  useEffect(() => {
    if (activeTab !== 'metrics') return;
    let active = true;

    async function loadMetrics() {
      try {
        if (isControlPlane) {
          const rangeMs: Record<string, number> = {
            '1h': 60 * 60 * 1000,
            '6h': 6 * 60 * 60 * 1000,
            '24h': 24 * 60 * 60 * 1000,
            '7d': 7 * 24 * 60 * 60 * 1000,
          };
          const now = new Date();
          const from = new Date(now.getTime() - (rangeMs[metricsRange] || rangeMs['1h']));
          const { data } = await api.getServerMetricsRange(from.toISOString(), now.toISOString(), 500);
          if (active) setMetricsHistory(data);
        } else {
          const { data } = await api.getServerMetrics(serverId, metricsRange);
          if (active) setMetricsHistory(data);
        }
      } catch {}
    }

    loadMetrics();
    return () => { active = false; };
  }, [serverId, activeTab, metricsRange, isControlPlane]);

  const agentResources = server ? parseResources(server.resources) : null;
  const resources = agentResources || localStatus;

  const cpuHistory = useMemo(() =>
    metricsHistory.map((s) => ({ timestamp: s.timestamp, value: s.cpu_percent })),
    [metricsHistory]
  );
  const memHistory = useMemo(() =>
    metricsHistory.map((s) => ({
      timestamp: s.timestamp,
      value: s.memory_total_bytes > 0 ? (s.memory_used_bytes / s.memory_total_bytes) * 100 : 0,
    })),
    [metricsHistory]
  );
  const diskHistory = useMemo(() =>
    metricsHistory.map((s) => ({
      timestamp: s.timestamp,
      value: s.disk_total_bytes > 0 ? (s.disk_used_bytes / s.disk_total_bytes) * 100 : 0,
    })),
    [metricsHistory]
  );

  async function handleSaveSettings() {
    if (!server) return;
    setSaving(true);
    try {
      const { data } = await api.updateServer(server.id, {
        name: editName,
        labels: editLabels || undefined,
      });
      setServer(data);
    } catch {}
    setSaving(false);
  }

  async function handleDisconnect() {
    if (!server) return;
    setSaving(true);
    try {
      await api.updateServer(server.id, { name: server.name });
      setServer((s) => s ? { ...s, status: 'draining' } : s);
      setShowDisconnect(false);
    } catch {}
    setSaving(false);
  }

  async function handleForceRemove() {
    if (!server) return;
    setDeleting(true);
    try {
      await api.deleteServer(server.id, true);
      window.location.href = '/servers';
    } catch {}
    setDeleting(false);
  }

  if (loading) {
    return (
      <div>
        <Skeleton width="200px" height="2rem" />
        <Skeleton width="100%" height="300px" variant="rect" />
      </div>
    );
  }

  if (!server) {
    return (
      <div class={styles.empty}>
        <p>Server not found.</p>
        <a href="/servers"><Button variant="secondary">Back to servers</Button></a>
      </div>
    );
  }

  return (
    <div>
      <div class={styles.pageHeader}>
        <div>
          <nav aria-label="Breadcrumb" class={styles.breadcrumb}>
            <a href="/servers">Servers</a> / <span aria-current="page">{server.name}</span>
          </nav>
          <div class={styles.titleRow}>
            <h1 class={styles.pageTitle}>{server.name}</h1>
            <StatusDot status={server.status} />
            <span class={isControlPlane ? styles.roleBadgeCp : styles.roleBadgeWorker}>
              {isControlPlane ? 'Control plane' : 'Worker'}
            </span>
          </div>
          <span class={styles.hostLabel}>{server.host}</span>
        </div>
      </div>

      {/* a11y [WCAG 4.1.2]: tablist with roving tabindex */}
      <div class={styles.tabBar} role="tablist" aria-label="Server sections">
        {TABS.map((tab) => (
          <button
            key={tab.key}
            type="button"
            role="tab"
            aria-selected={activeTab === tab.key}
            aria-controls={`panel-${tab.key}`}
            tabIndex={activeTab === tab.key ? 0 : -1}
            class={`${styles.tab} ${activeTab === tab.key ? styles.tabActive : ''}`}
            onClick={() => setActiveTab(tab.key)}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Overview tab */}
      {activeTab === 'overview' && (
        <div id="panel-overview" role="tabpanel" aria-labelledby="tab-overview" class={styles.tabPanel}>
          {resources ? (
            <div class={styles.metricsGrid}>
              <div class={styles.metricCard}>
                <div class={styles.metricHeader}>
                  <Cpu size={18} aria-hidden="true" />
                  <ProgressBar label="CPU" value={resources.cpu_percent} max={100} formatValue={(v) => formatPercent(v)} />
                </div>
              </div>
              <div class={styles.metricCard}>
                <div class={styles.metricHeader}>
                  <MemoryStick size={18} aria-hidden="true" />
                  <ProgressBar label="RAM" value={resources.ram_used_bytes} max={resources.ram_total_bytes} formatValue={(v, m) => `${formatBytes(v)} / ${formatBytes(m)}`} />
                </div>
              </div>
              <div class={styles.metricCard}>
                <div class={styles.metricHeader}>
                  <HardDrive size={18} aria-hidden="true" />
                  <ProgressBar label="Disk" value={resources.disk_used_bytes} max={resources.disk_total_bytes} formatValue={(v, m) => `${formatBytes(v)} / ${formatBytes(m)}`} />
                </div>
              </div>
              <div class={styles.metricCard}>
                <div class={styles.metricHeader}>
                  <ServerIcon size={18} aria-hidden="true" />
                  <div class={styles.metricStat}>
                    <span class={styles.metricStatLabel}>Apps</span>
                    <span class={styles.metricStatValue}>{apps.length}</span>
                  </div>
                </div>
              </div>
            </div>
          ) : (
            <p class={styles.noMetrics}>No metrics available yet. Connect the agent to start collecting data.</p>
          )}

          <div class={styles.infoSection}>
            <h2 class={styles.sectionTitle}>Connection</h2>
            <dl class={styles.infoGrid}>
              {server.agent_version && (
                <>
                  <dt class={styles.infoLabel}><Hash size={14} aria-hidden="true" /> Agent version</dt>
                  <dd class={styles.infoValueMono}>
                    {server.agent_version}
                    {server.status === 'online' && (
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={async () => {
                          try {
                            await api.updateAgent(server.id);
                            addToast('success', `Update sent to ${server.name}`);
                          } catch (e: any) {
                            addToast('error', e.message || 'Update failed');
                          }
                        }}
                      >
                        Update
                      </Button>
                    )}
                  </dd>
                </>
              )}
              {server.last_heartbeat_at && (
                <>
                  <dt class={styles.infoLabel}><Clock size={14} aria-hidden="true" /> Last heartbeat</dt>
                  <dd class={styles.infoValue}>{formatRelativeTime(server.last_heartbeat_at)}</dd>
                </>
              )}
              <dt class={styles.infoLabel}><Globe size={14} aria-hidden="true" /> Host</dt>
              <dd class={styles.infoValueMono}>{server.host}</dd>
              <dt class={styles.infoLabel}><Clock size={14} aria-hidden="true" /> Registered</dt>
              <dd class={styles.infoValue}>{new Date(server.created_at).toLocaleDateString()}</dd>
            </dl>
          </div>

          <ForecastSection serverId={serverId} />

          <InstancesSection serverId={serverId} />
        </div>
      )}

      {/* Apps tab */}
      {activeTab === 'apps' && (
        <div id="panel-apps" role="tabpanel" aria-labelledby="tab-apps" class={styles.tabPanel}>
          {apps.length === 0 ? (
            <div class={styles.emptyTab}>
              <p>No apps deployed to this server.</p>
              <a href="/apps/new">
                <Button variant="primary">Create an app</Button>
              </a>
            </div>
          ) : (
            <div class={styles.appGrid}>
              {apps.map((app) => (
                <AppCard key={app.id} app={app} />
              ))}
            </div>
          )}
        </div>
      )}

      {/* Metrics tab */}
      {activeTab === 'metrics' && (
        <div id="panel-metrics" role="tabpanel" aria-labelledby="tab-metrics" class={styles.tabPanel}>
          <div class={styles.rangeBar}>
            {(['1h', '6h', '24h', '7d'] as const).map((r) => (
              <button
                key={r}
                type="button"
                class={`${styles.rangeButton} ${metricsRange === r ? styles.rangeActive : ''}`}
                onClick={() => setMetricsRange(r)}
                aria-pressed={metricsRange === r}
              >
                {r}
              </button>
            ))}
          </div>

          <div class={styles.chartsGrid}>
            <div class={styles.chartCard}>
              <h3 class={styles.chartTitle}>CPU</h3>
              <MetricsChart data={cpuHistory} label="CPU" formatValue={(v) => formatPercent(v)} min={0} max={100} color="var(--color-primary)" />
            </div>
            <div class={styles.chartCard}>
              <h3 class={styles.chartTitle}>Memory</h3>
              <MetricsChart data={memHistory} label="Memory" formatValue={(v) => `${Math.round(v)}%`} min={0} max={100} color="var(--color-info)" />
            </div>
            <div class={styles.chartCard}>
              <h3 class={styles.chartTitle}>Disk</h3>
              <MetricsChart data={diskHistory} label="Disk" formatValue={(v) => `${Math.round(v)}%`} min={0} max={100} color="var(--color-warning)" />
            </div>
          </div>
        </div>
      )}

      {/* Settings tab */}
      {activeTab === 'settings' && (
        <div id="panel-settings" role="tabpanel" aria-labelledby="tab-settings" class={styles.tabPanel}>
          <div class={styles.settingsSection}>
            <h2 class={styles.sectionTitle}>General</h2>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}>
              <Input
                label="Server name"
                name="edit-server-name"
                id="edit-server-name"
                value={editName}
                onChange={setEditName}
              />
              <Input
                label="Labels"
                name="edit-server-labels"
                id="edit-server-labels"
                mono
                value={editLabels}
                onChange={setEditLabels}
                placeholder="env=production, region=eu-west"
                helpText="Comma-separated key=value pairs"
              />
            </div>
            <div class={styles.settingsActions}>
              <Button variant="primary" onClick={handleSaveSettings} loading={saving}>Save changes</Button>
            </div>
          </div>

          <ServerCleanupCard serverId={serverId} />

          {!isControlPlane && (
            <div class={styles.dangerZone}>
              <h2 class={styles.dangerTitle}>Danger zone</h2>
              <div class={styles.dangerActions}>
                <div class={styles.dangerRow}>
                  <div>
                    <p class={styles.dangerLabel}>Disconnect server</p>
                    <p class={styles.dangerHint}>Sets status to draining. Migrate apps before removing.</p>
                  </div>
                  <Button variant="danger" onClick={() => setShowDisconnect(true)}>
                    <Unplug size={14} /> Disconnect
                  </Button>
                </div>
                <div class={styles.dangerRow}>
                  <div>
                    <p class={styles.dangerLabel}>Force remove</p>
                    <p class={styles.dangerHint}>Permanently removes this server and orphans any remaining apps.</p>
                  </div>
                  <Button variant="danger" onClick={() => setShowForceRemove(true)}>
                    <Trash2 size={14} /> Remove
                  </Button>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      <ConfirmDialog
        open={showDisconnect}
        title="Disconnect server?"
        description={`This will set "${server.name}" to draining. Migrate all apps to another server before removing it.`}
        confirmLabel="Disconnect"
        variant="danger"
        loading={saving}
        onConfirm={handleDisconnect}
        onCancel={() => setShowDisconnect(false)}
      />

      <ConfirmDialog
        open={showForceRemove}
        title="Force remove server?"
        description={`This will permanently remove "${server.name}". Any apps still deployed on it will become orphaned. Type the server name to confirm.`}
        confirmLabel="Remove server"
        variant="danger"
        loading={deleting}
        onConfirm={handleForceRemove}
        onCancel={() => { setShowForceRemove(false); setConfirmName(''); }}
      />
    </div>
  );
}
