import { atom } from 'nanostores';

export type UpdateInfo = {
  available: boolean;
  current_version: string;
  latest_version: string | null;
  changelog_highlights: string[];
  changelog_url: string | null;
  breaking: boolean;
  breaking_changes: string | null;
  published_at: string | null;
  checked_at: string;
};

export type UpdateStepName =
  | 'checking_compatibility'
  | 'creating_backup'
  | 'downloading'
  | 'verifying_integrity'
  | 'applying_migrations'
  | 'restarting'
  | 'verifying_health';

export type UpdateStep = {
  name: UpdateStepName;
  label: string;
  status: 'pending' | 'running' | 'done' | 'failed';
  progress: number | null;
  duration_secs: number | null;
  error: string | null;
};

export type UpdateStatus = {
  state: 'idle' | 'downloading' | 'applying' | 'completed' | 'failed';
  target_version: string | null;
  steps: UpdateStep[];
  error: string | null;
};

// Raw payload shape returned by GET /system/update/status. The backend tracks a
// flat download_state machine; the dialog needs the richer UpdateStatus above,
// so mapUpdateStatus() bridges the two.
export type RawUpdateStatus = {
  current_version: string;
  available_version: string | null;
  download_state: 'none' | 'downloading' | 'ready' | 'error';
  download_progress: number | null;
  error_message: string | null;
};

const STATE_BY_DOWNLOAD: Record<RawUpdateStatus['download_state'], UpdateStatus['state']> = {
  none: 'idle',
  downloading: 'downloading',
  ready: 'downloading', // downloaded, apply about to start — still part of the "updating" flow
  error: 'failed',
};

export function mapUpdateStatus(raw: RawUpdateStatus): UpdateStatus {
  const downloadStatus: UpdateStep['status'] =
    raw.download_state === 'ready'
      ? 'done'
      : raw.download_state === 'error'
        ? 'failed'
        : raw.download_state === 'downloading'
          ? 'running'
          : 'pending';

  const steps: UpdateStep[] = [
    {
      name: 'downloading',
      label: 'Downloading update',
      status: downloadStatus,
      progress: raw.download_state === 'downloading' ? (raw.download_progress ?? 0) : null,
      duration_secs: null,
      error: raw.download_state === 'error' ? raw.error_message : null,
    },
  ];

  return {
    state: STATE_BY_DOWNLOAD[raw.download_state],
    target_version: raw.available_version,
    steps,
    error: raw.error_message,
  };
}

export const $updateInfo = atom<UpdateInfo | null>(null);
export const $updateStatus = atom<UpdateStatus | null>(null);
export const $updateDialogOpen = atom(false);
