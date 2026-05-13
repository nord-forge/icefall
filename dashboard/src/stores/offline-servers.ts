import { atom } from 'nanostores';

export type OfflineServer = {
  id: string;
  name: string;
};

export const $offlineServers = atom<OfflineServer[]>([]);

export function addOfflineServer(id: string, name: string) {
  const current = $offlineServers.get();
  if (!current.some((s) => s.id === id)) {
    $offlineServers.set([...current, { id, name }]);
  }
}

export function removeOfflineServer(id: string) {
  $offlineServers.set($offlineServers.get().filter((s) => s.id !== id));
}
