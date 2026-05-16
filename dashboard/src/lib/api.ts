import type { App, AppInstance, Deploy, Domain, EnvVar, LbPolicy, Project, Server, ServerAppInstance, ServerStatus, ServerMetricsSnapshot, User, ApiToken, HealthCheckResult, ProjectEnvironment, EnvironmentVariable, LogDrain, GitHubInstallation, GitHubRepo, CleanupSchedule, CleanupRun, ServerForecast, DeployApproval, CanaryResult, Team, TeamMember, TeamInvitation } from './types';
import type { UpdateInfo, UpdateStatus } from '@stores/update';
import { getCached, setCache, invalidatePrefix } from './cache';

const API_BASE = '/api/v1';

class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

export async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const method = (options?.method ?? 'GET').toUpperCase();

  // Serve GET requests from cache when a fresh entry exists
  if (method === 'GET') {
    const cached = getCached<T>(path);
    if (cached !== null) return cached;
  }

  const headers: Record<string, string> = {};
  if (options?.body) {
    headers['Content-Type'] = 'application/json';
  }

  const res = await fetch(`${API_BASE}${path}`, {
    credentials: 'same-origin',
    headers,
    ...options,
  });
  if (!res.ok) {
    if ((res.status === 401 || res.status === 400) && !path.startsWith('/auth/')) {
      const body = await res.json().catch(() => ({ error: '' }));
      if (body.error === 'Not authenticated' || res.status === 401) {
        window.location.href = '/login';
        throw new ApiError(res.status, 'Session expired');
      }
    }
    const body = await res.json().catch(() => ({ error: res.statusText }));
    throw new ApiError(res.status, body.error || 'Unknown error');
  }

  const data: T = await res.json();

  // Cache GET responses; invalidate related caches on mutations
  if (method === 'GET') {
    setCache(path, data);
  } else {
    // Invalidate the resource path (e.g. /apps/123 invalidates /apps*)
    const basePath = '/' + path.split('/').filter(Boolean).slice(0, 2).join('/');
    invalidatePrefix(basePath);
  }

  return data;
}

export const api = {
  logout: async () => {
    await fetch(`${API_BASE}/auth/logout`, { method: 'POST', credentials: 'same-origin' });
    window.location.href = '/login';
  },


  listApps: (params?: { tag?: string; project_id?: string }) => {
    const search = new URLSearchParams();
    if (params?.tag) search.set('tag', params.tag);
    if (params?.project_id) search.set('project_id', params.project_id);
    const qs = search.toString();
    return request<{ data: App[] }>(`/apps${qs ? `?${qs}` : ''}`);
  },

  getApp: (id: string) => request<{ data: App }>(`/apps/${id}`),

  createApp: (body: {
    name: string;
    git_repo?: string;
    git_branch?: string;
    framework?: string;
    image_ref?: string;
    compose_content?: string;
    port?: number;
    deploy_mode?: string;
    server_id?: string;
  }) => request<{ data: App }>('/apps', { method: 'POST', body: JSON.stringify(body) }),

  updateApp: (id: string, body: Partial<App>) =>
    request<{ data: App }>(`/apps/${id}`, { method: 'PUT', body: JSON.stringify(body) }),

  deleteApp: (id: string) =>
    request<{ message: string }>(`/apps/${id}`, { method: 'DELETE' }),

  startApp: (id: string) =>
    request<{ message: string; containers: number }>(`/apps/${id}/start`, { method: 'POST' }),

  stopApp: (id: string) =>
    request<{ message: string; containers: number }>(`/apps/${id}/stop`, { method: 'POST' }),

  restartApp: (id: string) =>
    request<{ message: string; containers: number }>(`/apps/${id}/restart`, { method: 'POST' }),

  listDeploys: (appId: string) =>
    request<{ data: Deploy[] }>(`/apps/${appId}/deploys`),

  getLatestDeploys: (appIds: string[]) =>
    request<{ data: Deploy[] }>(`/deploys/latest?app_ids=${appIds.join(',')}`),

  triggerDeploy: (appId: string, options?: { no_cache?: boolean }) =>
    request<{ data: Deploy }>(`/apps/${appId}/deploys`, {
      method: 'POST',
      ...(options ? { body: JSON.stringify(options) } : {}),
    }),

  rollbackDeploy: (appId: string, deployId: string) =>
    request<{ data: Deploy }>(`/apps/${appId}/deploys/${deployId}/rollback`, { method: 'POST' }),

  cancelDeploy: (deployId: string) =>
    request<{ data: Deploy }>(`/deploys/${deployId}/cancel`, { method: 'POST' }),

  checkDrift: (appId: string) =>
    request<{ data: { drifted: boolean; current_hash: string; deployed_hash: string | null; fields: string[] } }>(
      `/apps/${appId}/drift`
    ),

  // --- Load balancing / scaling ---

  listInstances: (appId: string) =>
    request<{ data: AppInstance[] }>(`/apps/${appId}/instances`),

  scaleApp: (appId: string, desiredInstances: number) =>
    request<{ message: string; deploy_id?: string; desired_instances: number }>(
      `/apps/${appId}/scale`,
      { method: 'PUT', body: JSON.stringify({ desired_instances: desiredInstances }) }
    ),

  updateLbConfig: (
    appId: string,
    body: { policy?: LbPolicy; health_check_path?: string; sticky_sessions?: boolean }
  ) =>
    request<{ message: string }>(`/apps/${appId}/lb-config`, {
      method: 'PUT',
      body: JSON.stringify(body),
    }),

  deleteInstance: (appId: string, instanceId: string) =>
    request<{ message: string }>(`/apps/${appId}/instances/${instanceId}`, {
      method: 'DELETE',
    }),

  listEnvVars: (appId: string, reveal = false) =>
    request<{ data: EnvVar[] }>(`/apps/${appId}/env${reveal ? '?reveal=true' : ''}`),

  setEnvVar: (appId: string, body: { key: string; value: string; scope?: string }) =>
    request<{ data: { id: string; key: string; scope: string } }>(
      `/apps/${appId}/env`,
      { method: 'POST', body: JSON.stringify(body) },
    ),

  deleteEnvVar: (appId: string, varId: string) =>
    request<{ message: string }>(`/apps/${appId}/env/${varId}`, { method: 'DELETE' }),

  importEnv: (appId: string, content: string, scope = 'shared') =>
    request<{ imported: number; skipped: string[] }>(
      `/apps/${appId}/env/import`,
      { method: 'POST', body: JSON.stringify({ content, scope }) },
    ),

  listDomains: (appId: string) =>
    request<{ data: Domain[] }>(`/apps/${appId}/domains`),

  addDomain: (appId: string, domain: string, path?: string) =>
    request<{ data: Domain }>(
      `/apps/${appId}/domains`,
      { method: 'POST', body: JSON.stringify({ domain, path: path || undefined }) },
    ),

  listDatabases: () => request<{ data: any[] }>('/databases'),

  createDatabase: (body: { name: string; db_type: string; app_id?: string; memory_mb?: number; expose_port?: boolean }) =>
    request<{ data: any }>('/databases', { method: 'POST', body: JSON.stringify(body) }),

  linkDatabase: (dbId: string, appId: string) =>
    request<{ message: string }>(`/databases/${dbId}/link/${appId}`, { method: 'POST' }),

  unlinkDatabase: (dbId: string, appId: string) =>
    request<{ message: string }>(`/databases/${dbId}/link/${appId}`, { method: 'DELETE' }),

  startDatabase: (dbId: string) =>
    request<{ message: string }>(`/databases/${dbId}/start`, { method: 'POST' }),

  stopDatabase: (dbId: string) =>
    request<{ message: string }>(`/databases/${dbId}/stop`, { method: 'POST' }),

  restartDatabase: (dbId: string) =>
    request<{ message: string }>(`/databases/${dbId}/restart`, { method: 'POST' }),

  getHealth: (appId: string) =>
    request<{ data: HealthCheckResult[] }>(`/apps/${appId}/health`),

  updateHealth: (appId: string, body: {
    check_type?: string;
    interval_secs?: number;
    failure_threshold?: number;
    auto_restart?: boolean;
    config?: string;
  }) =>
    request<{ data: any }>(`/apps/${appId}/health`, { method: 'PUT', body: JSON.stringify(body) }),

  createHealthCheck: (appId: string, body: {
    check_type: string;
    interval_secs: number;
    failure_threshold: number;
    auto_restart: boolean;
    config?: string;
  }) =>
    request<{ data: any }>(`/apps/${appId}/health`, { method: 'POST', body: JSON.stringify(body) }),

  deleteHealthCheck: (appId: string, checkId: string) =>
    request<{ message: string }>(`/health-checks/${checkId}`, { method: 'DELETE' }),

  getServerStatus: () => request<ServerStatus>('/server/status'),

  deleteDomain: (appId: string, domainId: string) =>
    request<{ message: string }>(`/apps/${appId}/domains/${domainId}`, { method: 'DELETE' }),

  listUsers: () => request<{ data: User[] }>('/users'),

  getMe: () => request<{ data: User }>('/users/me'),

  inviteUser: (email: string, role: string) =>
    request<{ data: { invite_token: string } }>(
      '/users/invite',
      { method: 'POST', body: JSON.stringify({ email, role }) },
    ),

  changeRole: (userId: string, role: string) =>
    request<{ data: User }>(`/users/${userId}/role`, { method: 'PUT', body: JSON.stringify({ role }) }),

  deactivateUser: (userId: string) =>
    request<{ message: string }>(`/users/${userId}`, { method: 'DELETE' }),

  resetUserPassword: (userId: string) =>
    request<{
      data: { user_id: string; email: string; temporary_password: string };
      warning: string;
    }>(`/users/${userId}/reset-password`, { method: 'POST' }),

  resetUser2fa: (userId: string) =>
    request<{ message: string; user_id: string; email: string }>(
      `/users/${userId}/2fa`,
      { method: 'DELETE' },
    ),

  getRegistrationSettings: () =>
    request<{
      data: {
        allow_registration: boolean;
        allowed_domains: string | null;
        default_role: string;
      };
    }>('/settings/registration'),

  updateRegistrationSettings: (body: {
    allow_registration?: boolean;
    allowed_domains?: string;
    default_role?: string;
  }) =>
    request<{
      data: {
        allow_registration: boolean;
        allowed_domains: string | null;
        default_role: string;
      };
      message: string;
    }>('/settings/registration', { method: 'PUT', body: JSON.stringify(body) }),

  listTokens: () => request<{ data: ApiToken[] }>('/tokens'),

  createToken: (name: string, expiresAt?: string) =>
    request<{ data: ApiToken & { token: string } }>(
      '/tokens',
      { method: 'POST', body: JSON.stringify({ name, expires_at: expiresAt }) },
    ),

  revokeToken: (tokenId: string) =>
    request<{ message: string }>(`/tokens/${tokenId}`, { method: 'DELETE' }),

  getInstanceBackupConfig: () =>
    request<{ data: { enabled: boolean; cron_schedule: string; retention_count: number } }>(
      '/settings/instance-backup'
    ),

  updateInstanceBackupConfig: (body: {
    enabled?: boolean;
    cron_schedule?: string;
    retention_count?: number;
  }) =>
    request<{ data: { enabled: boolean; cron_schedule: string; retention_count: number } }>(
      '/settings/instance-backup',
      { method: 'PUT', body: JSON.stringify(body) },
    ),

  triggerInstanceBackup: () =>
    request<{ message: string; status: string }>(
      '/settings/instance-backup/trigger',
      { method: 'POST' },
    ),

  listInstanceBackupHistory: () =>
    request<{
      data: Array<{
        id: string;
        filename: string;
        size_bytes: number;
        status: string;
        error_message: string | null;
        s3_key: string | null;
        started_at: string;
        finished_at: string | null;
      }>;
    }>('/settings/instance-backup/history'),

  getServerIp: () => request<{ ip: string }>('/server/ip'),

  getServerMetricsHistory: (limit?: number) =>
    request<{ data: ServerMetricsSnapshot[] }>(
      `/server/metrics/history${limit ? `?limit=${limit}` : ''}`
    ),

  getServerMetricsRange: (from: string, to: string, limit?: number) =>
    request<{ data: ServerMetricsSnapshot[]; total: number }>(
      `/server/metrics/range?from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}${limit ? `&limit=${limit}` : ''}`
    ),

  listDbTables: (dbId: string) =>
    request<{ data: string[]; types?: Record<string, string> }>(`/databases/${dbId}/tables`),

  queryDb: (dbId: string, query: string) =>
    request<{ columns?: string[]; rows?: string[][]; documents?: any[]; row_count: number }>(
      `/databases/${dbId}/query`,
      { method: 'POST', body: JSON.stringify({ query }) },
    ),

  // Projects
  listProjects: () =>
    request<{ data: Project[] }>('/projects'),

  getProject: (id: string) =>
    request<{ data: Project & { apps: App[]; databases: any[] } }>(`/projects/${id}`),

  createProject: (body: { name: string; description?: string; color?: string }) =>
    request<{ data: Project }>('/projects', { method: 'POST', body: JSON.stringify(body) }),

  updateProject: (id: string, body: { name?: string; description?: string | null; color?: string | null }) =>
    request<{ data: Project }>(`/projects/${id}`, { method: 'PUT', body: JSON.stringify(body) }),

  deleteProject: (id: string) =>
    request<{ message: string }>(`/projects/${id}`, { method: 'DELETE' }),

  // 2FA
  setup2fa: () =>
    request<{ data: { secret: string; qr_svg: string; otpauth_url: string } }>(
      '/auth/2fa/setup',
      { method: 'POST' },
    ),

  verify2fa: (code: string) =>
    request<{ data: { totp_enabled: boolean; backup_codes: string[] }; warning: string }>(
      '/auth/2fa/verify',
      { method: 'POST', body: JSON.stringify({ code }) },
    ),

  validate2fa: (userId: string, code: string) =>
    request<{ data: { user: User; session_id: string } }>(
      '/auth/2fa/validate',
      { method: 'POST', body: JSON.stringify({ user_id: userId, code }) },
    ),

  regenerateBackupCodes: (code: string) =>
    request<{ data: { backup_codes: string[] }; warning: string }>(
      '/auth/2fa/backup-codes',
      { method: 'POST', body: JSON.stringify({ code }) },
    ),

  disable2fa: (code: string) =>
    request<{ message: string }>(
      '/auth/2fa',
      { method: 'DELETE', body: JSON.stringify({ code }) },
    ),

  // OAuth
  getEnabledOAuthProviders: () =>
    request<{ data: { github: boolean; google: boolean } }>(
      '/settings/oauth/providers',
    ),

  listOAuthIdentities: () =>
    request<{ data: Array<{ id: string; provider: string; provider_email: string | null; created_at: string }> }>(
      '/auth/oauth/identities',
    ),

  unlinkOAuthProvider: (provider: string) =>
    request<{ message: string }>(
      `/auth/oauth/${provider}/unlink`,
      { method: 'DELETE' },
    ),

  getOAuthSettings: () =>
    request<{ data: {
      github_client_id: string | null;
      github_has_secret: boolean;
      github_enabled: boolean;
      github_callback_url: string;
      google_client_id: string | null;
      google_has_secret: boolean;
      google_enabled: boolean;
      google_callback_url: string;
    } }>('/settings/oauth'),

  updateOAuthSettings: (body: {
    github_client_id?: string;
    github_client_secret?: string;
    github_enabled?: boolean;
    google_client_id?: string;
    google_client_secret?: string;
    google_enabled?: boolean;
  }) =>
    request<{ data: any; message: string }>(
      '/settings/oauth',
      { method: 'PUT', body: JSON.stringify(body) },
    ),

  // Volumes
  listVolumes: (appId: string) =>
    request<{ data: Array<{ source: string; target: string; read_only: boolean }> }>(
      `/apps/${appId}/volumes`,
    ),

  browseVolume: (appId: string, mountIndex: number, path = '/') =>
    request<{
      data: Array<{ name: string; size: number; modified: string; is_dir: boolean; permissions: string }>;
      path: string;
      mount_target: string;
    }>(`/apps/${appId}/volumes/${mountIndex}/browse?path=${encodeURIComponent(path)}`),

  volumeSize: (appId: string, mountIndex: number) =>
    request<{ data: { bytes_used: number; mount_target: string } }>(
      `/apps/${appId}/volumes/${mountIndex}/size`,
    ),

  deleteVolumeFile: (appId: string, mountIndex: number, path: string) =>
    request<{ message: string; path: string }>(
      `/apps/${appId}/volumes/${mountIndex}/delete`,
      { method: 'POST', body: JSON.stringify({ path }) },
    ),

  // Profile
  changePassword: (currentPassword: string, newPassword: string) =>
    request<{ message: string }>(
      '/users/me/password',
      { method: 'PUT', body: JSON.stringify({ current_password: currentPassword, new_password: newPassword }) },
    ),

  changeEmail: (email: string, password: string) =>
    request<{ message: string; data: { email: string } }>(
      '/users/me/email',
      { method: 'PUT', body: JSON.stringify({ email, password }) },
    ),

  listSessions: () =>
    request<{ data: Array<{ id: string; created_at: string; expires_at: string; is_current: boolean }> }>(
      '/users/me/sessions',
    ),

  revokeAllSessions: () =>
    request<{ message: string }>(
      '/users/me/sessions',
      { method: 'DELETE' },
    ),

  deleteAccount: (password: string) =>
    request<{ message: string }>(
      '/users/me',
      { method: 'DELETE', body: JSON.stringify({ password }) },
    ),

  getPreferences: () =>
    request<{ data: Record<string, unknown> }>('/users/me/preferences'),

  updatePreferences: (preferences: Record<string, unknown>) =>
    request<{ data: Record<string, unknown> }>(
      '/users/me/preferences',
      { method: 'PUT', body: JSON.stringify(preferences) },
    ),

  deleteUser: (userId: string) =>
    request<{ message: string }>(
      `/users/${userId}`,
      { method: 'DELETE' },
    ),

  // Servers
  listServers: () =>
    request<{ data: Server[] }>('/servers'),

  getServer: (id: string) =>
    request<{ data: Server }>(`/servers/${id}`),

  createServer: (body: { name: string; host: string }) =>
    request<{ data: Server & { enrollment_token: string } }>('/servers', {
      method: 'POST',
      body: JSON.stringify(body),
    }),

  updateServer: (id: string, body: { name?: string; labels?: string }) =>
    request<{ data: Server }>(`/servers/${id}`, {
      method: 'PUT',
      body: JSON.stringify(body),
    }),

  deleteServer: (id: string, force = false) =>
    request<{ message: string }>(`/servers/${id}${force ? '?force=true' : ''}`, {
      method: 'DELETE',
    }),

  regenerateServerToken: (id: string) =>
    request<{ data: { enrollment_token: string } }>(`/servers/${id}/token`, {
      method: 'POST',
    }),

  listServerInstances: (serverId: string) =>
    request<{ data: ServerAppInstance[] }>(`/servers/${serverId}/instances`),

  updateAgent: (id: string) =>
    request<{ data: { status: string; target_version?: string } }>(`/servers/${id}/update`, {
      method: 'POST',
    }),

  updateAllAgents: () =>
    request<{ data: { updated: number; skipped: number } }>('/servers/update-all', {
      method: 'POST',
    }),

  getServerMetrics: (id: string, range?: string) =>
    request<{ data: ServerMetricsSnapshot[] }>(
      `/servers/${id}/metrics${range ? `?range=${range}` : ''}`
    ),

  migrateApp: (appId: string, targetServerId: string, acknowledgeVolumeLoss: boolean) =>
    request<{ data: Deploy }>(`/apps/${appId}/migrate`, {
      method: 'PUT',
      body: JSON.stringify({
        target_server_id: targetServerId,
        acknowledge_volume_loss: acknowledgeVolumeLoss,
      }),
    }),

  // Self-update
  checkForUpdate: () =>
    request<{ data: UpdateInfo }>('/system/update/check'),

  getUpdateStatus: () =>
    request<{ data: UpdateStatus }>('/system/update/status'),

  applyUpdate: () =>
    request<{ data: UpdateStatus }>('/system/update/apply', { method: 'POST' }),

  downloadUpdate: () =>
    request<{ data: any }>('/system/update/download', { method: 'POST' }),

  skipUpdateVersion: (version: string) =>
    request<{ message: string }>('/system/update/skip', {
      method: 'POST',
      body: JSON.stringify({ version }),
    }),

  getUpdatePreferences: () =>
    request<{ data: any }>('/system/update/preferences'),

  updateUpdatePreferences: (body: any) =>
    request<{ data: any }>('/system/update/preferences', {
      method: 'PATCH',
      body: JSON.stringify(body),
    }),

  getUpdateHistory: () =>
    request<{ data: any[] }>('/system/update/history'),

  rollbackUpdate: () =>
    request<{ message: string }>('/system/update/rollback', { method: 'POST' }),

  // Wake (ghost mode)
  wakeApp: (id: string) =>
    request<{ message: string; containers: number }>(`/apps/${id}/wake`, { method: 'POST' }),

  // Deploy approval
  approveDeploy: (deployId: string, comment?: string) =>
    request<{ data: DeployApproval }>(`/deploys/${deployId}/approve`, {
      method: 'POST',
      body: JSON.stringify({ action: 'approve', comment }),
    }),

  rejectDeploy: (deployId: string, comment?: string) =>
    request<{ data: DeployApproval }>(`/deploys/${deployId}/approve`, {
      method: 'POST',
      body: JSON.stringify({ action: 'reject', comment }),
    }),

  // Project environments
  listProjectEnvironments: (projectId: string) =>
    request<{ data: ProjectEnvironment[] }>(`/projects/${projectId}/environments`),

  createProjectEnvironment: (projectId: string, body: { name: string; color?: string }) =>
    request<{ data: ProjectEnvironment }>(`/projects/${projectId}/environments`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),

  updateProjectEnvironment: (projectId: string, envId: string, body: { name: string; color?: string }) =>
    request<{ data: ProjectEnvironment }>(`/projects/${projectId}/environments/${envId}`, {
      method: 'PUT',
      body: JSON.stringify(body),
    }),

  deleteProjectEnvironment: (projectId: string, envId: string) =>
    request<{ message: string }>(`/projects/${projectId}/environments/${envId}`, {
      method: 'DELETE',
    }),

  // Environment variables
  listEnvironmentVariables: (envId: string) =>
    request<{ data: EnvironmentVariable[] }>(`/environments/${envId}/variables`),

  setEnvironmentVariable: (envId: string, body: { key: string; value: string; is_secret?: boolean }) =>
    request<{ data: EnvironmentVariable }>(`/environments/${envId}/variables`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),

  deleteEnvironmentVariable: (envId: string, varId: string) =>
    request<{ message: string }>(`/environments/${envId}/variables/${varId}`, {
      method: 'DELETE',
    }),

  // Log drains
  listLogDrains: (appId: string) =>
    request<{ data: LogDrain[] }>(`/apps/${appId}/log-drains`),

  listGlobalLogDrains: () =>
    request<{ data: LogDrain[] }>('/log-drains'),

  createLogDrain: (appId: string, body: { name: string; drain_type: string; config: Record<string, unknown>; enabled?: boolean }) =>
    request<{ data: LogDrain }>(`/apps/${appId}/log-drains`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),

  updateLogDrain: (drainId: string, body: { name: string; drain_type: string; config: Record<string, unknown>; enabled?: boolean }) =>
    request<{ data: LogDrain }>(`/log-drains/${drainId}`, {
      method: 'PUT',
      body: JSON.stringify(body),
    }),

  deleteLogDrain: (drainId: string) =>
    request<{ message: string }>(`/log-drains/${drainId}`, { method: 'DELETE' }),

  testLogDrain: (drainId: string) =>
    request<{ data: { success: boolean; message: string } }>(`/log-drains/${drainId}/test`, {
      method: 'POST',
    }),

  // GitHub Apps
  getGitHubSetup: () =>
    request<{ manifest: Record<string, unknown>; form_action: string }>('/github/setup'),

  listGitHubApps: () =>
    request<{ data: Array<{ id: string; name: string; app_id: number; html_url: string; created_at: string }> }>('/github/apps'),

  // Git sources
  listGitSources: () =>
    request<{ data: GitHubInstallation[] }>('/git-sources'),

  deleteGitSource: (id: string) =>
    request<{ message: string }>(`/git-sources/${id}`, { method: 'DELETE' }),

  listGitSourceRepos: (id: string) =>
    request<{ data: GitHubRepo[] }>(`/git-sources/${id}/repos`),

  // Cleanup
  getCleanupSchedule: () =>
    request<{ data: CleanupSchedule }>('/cleanup-schedule'),

  updateCleanupSchedule: (body: Partial<CleanupSchedule>) =>
    request<{ data: CleanupSchedule; message: string }>('/cleanup-schedule', {
      method: 'PUT',
      body: JSON.stringify(body),
    }),

  runCleanup: () =>
    request<{ data: { status: string; message: string } }>('/cleanup-schedule/run', {
      method: 'POST',
    }),

  listCleanupHistory: () =>
    request<{ data: CleanupRun[] }>('/cleanup-schedule/history'),

  // Server forecast
  getServerForecast: (serverId: string) =>
    request<{ data: ServerForecast }>(`/servers/${serverId}/forecast`),

  // Bundles
  exportBundle: (appId: string) =>
    request<{ data: Record<string, unknown> }>(`/apps/${appId}/export`),

  importBundle: (bundle: Record<string, unknown>) =>
    request<{ data: App }>('/bundles/import', {
      method: 'POST',
      body: JSON.stringify(bundle),
    }),

  // Teams
  listTeams: () =>
    request<{ data: Team[] }>('/teams'),

  createTeam: (name: string) =>
    request<{ data: Team }>('/teams', {
      method: 'POST',
      body: JSON.stringify({ name }),
    }),

  getTeam: (id: string) =>
    request<{ data: { team: Team; members: TeamMember[]; resource_count: number } }>(`/teams/${id}`),

  updateTeam: (id: string, body: { name?: string; settings?: Record<string, unknown> }) =>
    request<{ data: Team }>(`/teams/${id}`, {
      method: 'PUT',
      body: JSON.stringify(body),
    }),

  deleteTeam: (id: string) =>
    request<{ message: string }>(`/teams/${id}`, { method: 'DELETE' }),

  switchTeam: (id: string) =>
    request<{ data: { team_id: string; role: string }; message: string }>(`/teams/${id}/switch`, {
      method: 'POST',
    }),

  listTeamMembers: (teamId: string) =>
    request<{ data: TeamMember[] }>(`/teams/${teamId}/members`),

  updateTeamMemberRole: (teamId: string, userId: string, role: string) =>
    request<{ message: string }>(`/teams/${teamId}/members/${userId}`, {
      method: 'PUT',
      body: JSON.stringify({ role }),
    }),

  removeTeamMember: (teamId: string, userId: string) =>
    request<{ message: string }>(`/teams/${teamId}/members/${userId}`, {
      method: 'DELETE',
    }),

  inviteTeamMember: (teamId: string, email: string, role: string) =>
    request<{ data: TeamInvitation }>(`/teams/${teamId}/invite`, {
      method: 'POST',
      body: JSON.stringify({ email, role }),
    }),

  listTeamInvitations: (teamId: string) =>
    request<{ data: TeamInvitation[] }>(`/teams/${teamId}/invitations`),

  acceptInvitation: (token: string) =>
    request<{ data: { team: Team; role: string }; message: string }>(`/invitations/${token}/accept`, {
      method: 'POST',
    }),

  declineInvitation: (token: string) =>
    request<{ message: string }>(`/invitations/${token}`, {
      method: 'DELETE',
    }),
};

export { ApiError };
