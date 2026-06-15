export type App = {
  id: string;
  name: string;
  git_repo: string | null;
  git_branch: string;
  framework: string | null;
  build_config: string | null;
  resource_limits: string | null;
  preview_enabled: boolean;
  preview_branch_pattern: string | null;
  webhook_secret: string | null;
  tags: string | null;
  volumes: string | null;
  image_ref: string | null;
  compose_content: string | null;
  project_id: string | null;
  server_id: string | null;
  deploy_mode: string;
  disable_build_cache: boolean;
  ghost_mode_enabled: boolean;
  ghost_mode_idle_minutes: number;
  ghost_mode_status: string;
  canary_enabled: boolean;
  canary_config: string | null;
  tunnel_enabled: boolean;
  require_deploy_approval: boolean;
  log_noise_patterns: string | null;
  log_highlight_patterns: string | null;
  project_environment_id: string | null;
  desired_instances: number;
  lb_policy: LbPolicy;
  lb_health_check_path: string;
  lb_sticky_sessions: boolean;
  created_at: string;
  updated_at: string;
}

export type LbPolicy = 'round_robin' | 'least_conn' | 'ip_hash' | 'random';

export type AppInstanceStatus =
  | 'deploying'
  | 'running'
  | 'unhealthy'
  | 'stopped'
  | 'failed';

export type AppInstance = {
  id: string;
  app_id: string;
  server_id: string;
  status: AppInstanceStatus;
  container_id: string | null;
  host_port: number | null;
  created_at: string;
  updated_at: string;
}

/** An app instance annotated with its app name (server-scoped listing). */
export type ServerAppInstance = AppInstance & {
  app_name: string;
}

export type DeployMode = 'auto' | 'native' | 'container';

export type Project = {
  id: string;
  name: string;
  description: string | null;
  color: string | null;
  app_count?: number;
  database_count?: number;
  created_at: string;
  updated_at: string;
}

export type Deploy = {
  id: string;
  app_id: string;
  environment_id: string;
  status: DeployStatus;
  git_sha: string | null;
  build_log: string | null;
  started_at: string | null;
  finished_at: string | null;
  image_ref: string | null;
  container_id: string | null;
  env_snapshot: string | null;
  no_cache: boolean;
  config_hash: string | null;
  scheduled_at: string | null;
  created_at: string;
}

export type DeployStatus =
  | 'scheduled'
  | 'pending'
  | 'building'
  | 'deploying'
  | 'running'
  | 'failed'
  | 'stopped'
  | 'cancelled'
  | 'missed';

export type EnvVar = {
  id: string;
  key: string;
  value: string;
  scope: 'shared' | 'production' | 'preview';
  created_at: string;
}

export type Domain = {
  id: string;
  app_id: string;
  domain: string;
  path: string | null;
  verified: boolean;
  ssl_status: string;
  created_at: string;
}

export type ServerStatus = {
  status: string;
  version: string;
  cpu_percent: number;
  memory_used_bytes: number;
  memory_total_bytes: number;
  disk_used_bytes: number;
  disk_total_bytes: number;
}

export type Server = {
  id: string;
  name: string;
  host: string;
  role: 'control-plane' | 'worker';
  status: 'online' | 'offline' | 'enrolling' | 'draining';
  agent_version: string | null;
  resources: string | null;
  labels: string | null;
  last_heartbeat_at: string | null;
  created_at: string;
  updated_at: string;
  app_count?: number;
  instance_count?: number;
  recommendation_score?: number;
  recommended?: boolean;
}

export type ServerResources = {
  cpu_percent: number;
  cpu_cores: number;
  ram_used_bytes: number;
  ram_total_bytes: number;
  disk_used_bytes: number;
  disk_total_bytes: number;
  load_average: number[];
}

export type BuildStep = {
  name: string;
  status: 'pending' | 'running' | 'done' | 'failed';
  started_at: string | null;
  finished_at: string | null;
  output: string[];
}

export type AppStatus = 'online' | 'building' | 'deploying' | 'failed' | 'stopped';

export type ServerMetricsSnapshot = {
  timestamp: string;
  cpu_percent: number;
  memory_used_bytes: number;
  memory_total_bytes: number;
  disk_used_bytes: number;
  disk_total_bytes: number;
}

export type User = {
  id: string;
  email: string;
  role: 'admin' | 'deployer' | 'viewer';
  totp_enabled: boolean;
  is_active: boolean;
  last_login_at: string | null;
  created_at: string;
}

export type RegistrationSettings = {
  allow_registration: boolean;
  allowed_domains: string | null;
  default_role: string;
}

export type ApiToken = {
  id: string;
  name: string;
  prefix: string;
  abilities?: string[];
  last_used_at: string | null;
  expires_at: string | null;
  created_at: string;
}

export type HealthCheck = {
  id: string;
  app_id: string;
  check_type: string;
  config: string | null;
  interval_secs: number;
  failure_threshold: number;
  auto_restart: boolean;
  created_at: string;
}

export type HealthCheckEvent = {
  id: string;
  health_check_id: string;
  status: 'healthy' | 'unhealthy';
  checked_at: string;
}

export type HealthCheckResult = {
  check: HealthCheck;
  current_status: string;
  uptime_percent: number;
  recent_events: HealthCheckEvent[];
}

export type ProjectEnvironment = {
  id: string;
  project_id: string;
  name: string;
  color: string | null;
  created_at: string;
}

export type EnvironmentVariable = {
  id: string;
  key: string;
  value: string;
  is_secret: boolean;
}

export type LogDrain = {
  id: string;
  app_id: string | null;
  name: string;
  drain_type: 'loki' | 'axiom' | 'http';
  config: string;
  enabled: boolean;
  last_sent_at: string | null;
  created_at: string;
}

export type GitHubInstallation = {
  id: string;
  account_name: string;
  account_type: 'user' | 'organization';
  repo_count: number;
  status: 'active' | 'suspended';
  created_at: string;
}

export type GitHubRepo = {
  id: string;
  full_name: string;
  default_branch: string;
  private: boolean;
}

export type CleanupSchedule = {
  cron: string;
  disk_threshold_percent: number;
  dangling_images: boolean;
  unused_images: boolean;
  stopped_containers: boolean;
  stopped_container_age_hours: number;
  volumes: boolean;
  networks: boolean;
  enabled: boolean;
}

export type CleanupRun = {
  id: string;
  started_at: string;
  finished_at: string | null;
  status: 'running' | 'completed' | 'failed';
  freed_bytes: number;
  removed_items: number;
  error: string | null;
}

export type ServerForecast = {
  disk: {
    current_ratio: number;
    daily_growth: number;
    days_until_full: number | null;
  };
  memory: {
    current_ratio: number;
    daily_growth: number;
    days_until_full: number | null;
  };
  cpu: {
    current_percent: number;
    daily_trend: number;
  };
  data_points: number;
}

export type DeployApproval = {
  id: string;
  deploy_id: string;
  status: 'pending' | 'approved' | 'rejected';
  reviewer: string | null;
  comment: string | null;
  decided_at: string | null;
}

export type CanaryResult = {
  deploy_id: string;
  status: 'running' | 'passed' | 'failed';
  p50_ms: number;
  p95_ms: number;
  p99_ms: number;
  error_rate: number;
  request_count: number;
}

export type Team = {
  id: string;
  name: string;
  slug: string;
  owner_id: string;
  settings: string | null;
  created_at: string;
  updated_at: string;
}

export type TeamMember = {
  id: string;
  user_id: string;
  email: string;
  role: 'owner' | 'admin' | 'member' | 'viewer';
  accepted_at: string | null;
  created_at: string;
}

export type TeamInvitation = {
  id: string;
  team_id: string;
  email: string;
  role: string;
  token: string;
  invited_by: string;
  expires_at: string;
  created_at: string;
}

// Reverse proxy management (IF-149)

export type RateLimitPreset = {
  enabled: boolean;
  requests: number;
  window: 'second' | 'minute';
  burst: number;
  per_ip: boolean;
}

export type BasicAuthPreset = {
  enabled: boolean;
  username: string;
  /** Read-only: bcrypt hash returned by the server. Never set this from the client. */
  password_hash?: string;
  /** Plaintext password to set; hashed server-side. Empty = keep existing. */
  password?: string;
  path?: string | null;
}

export type RedirectRule = {
  from: string;
  to: string;
  status: number;
}

export type HeaderRule = {
  name: string;
  value: string;
}

export type ProxyPresets = {
  force_https?: boolean | null;
  rate_limit?: RateLimitPreset | null;
  basic_auth?: BasicAuthPreset | null;
  redirects: RedirectRule[];
  headers: HeaderRule[];
}

export type ProxyConfig = {
  has_custom_proxy_config: boolean;
  custom_proxy_config: string | null;
  presets: ProxyPresets;
  routes: unknown;
}

export type ProxyConfigHistoryEntry = {
  id: string;
  app_id: string;
  config: string;
  created_at: string;
}

export type GlobalProxySettings = {
  default_headers: string | null;
  default_rate_limit: string | null;
  force_https: boolean;
  public_port_range_start: number;
  public_port_range_end: number;
  updated_at: string;
  caddy_running: boolean;
  caddy_version: string;
}

// IF-172: public TCP access status for a managed database.
export type PublicAccessConnection = {
  host: string;
  port: number;
  user: string;
  password: string;
  url: string;
}

export type PublicAccess = {
  enabled: boolean;
  port?: number;
  host?: string;
  ip_whitelist?: string | null;
  connection?: PublicAccessConnection | null;
}
