export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const value = bytes / Math.pow(1024, i);
  return `${value.toFixed(value < 10 ? 1 : 0)} ${units[i]}`;
}

export function formatDuration(seconds: number): string {
  if (seconds < 60) return `${Math.round(seconds)}s`;
  const minutes = Math.floor(seconds / 60);
  const secs = Math.round(seconds % 60);
  if (minutes < 60) return `${minutes}m ${secs}s`;
  const hours = Math.floor(minutes / 60);
  const mins = minutes % 60;
  return `${hours}h ${mins}m`;
}

export function formatRelativeTime(isoDate: string): string {
  const date = new Date(isoDate);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSecs = Math.floor(diffMs / 1000);

  if (diffSecs < 60) return 'just now';
  if (diffSecs < 3600) return `${Math.floor(diffSecs / 60)} min ago`;
  if (diffSecs < 86400) return `${Math.floor(diffSecs / 3600)} hours ago`;
  if (diffSecs < 604800) return `${Math.floor(diffSecs / 86400)} days ago`;
  return date.toLocaleDateString();
}

/** Time remaining until a future ISO timestamp, e.g. "in 2h 5m" or "due now". */
export function formatCountdown(isoDate: string): string {
  const diffSecs = Math.floor((new Date(isoDate).getTime() - Date.now()) / 1000);
  if (diffSecs <= 0) return 'due now';
  if (diffSecs < 60) return `in ${diffSecs}s`;
  if (diffSecs < 3600) return `in ${Math.floor(diffSecs / 60)}m`;
  if (diffSecs < 86400) {
    const h = Math.floor(diffSecs / 3600);
    const m = Math.floor((diffSecs % 3600) / 60);
    return m ? `in ${h}h ${m}m` : `in ${h}h`;
  }
  return `in ${Math.floor(diffSecs / 86400)}d`;
}

export function formatPercent(value: number): string {
  return `${Math.round(value)}%`;
}

export function shortSha(sha: string): string {
  return sha.slice(0, 7);
}
