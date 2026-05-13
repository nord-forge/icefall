import { atom } from 'nanostores';
import type { Server } from '@lib/types';

export const $servers = atom<Server[]>([]);
export const $serversLoading = atom(true);
export const $serverCount = atom(0);
