import { onVisibilityChange, isTabVisible } from './visibility';

type EventHandler = (data: unknown) => void;

const MAX_RETRY_DELAY = 30_000;
const MAX_RETRIES = 10;

export function createSSEClient(
  path: string,
  handlers: Record<string, EventHandler>,
) {
  let lastEventId: string | undefined;
  let source: EventSource | null = null;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let retryCount = 0;
  let closed = false;

  function disconnect() {
    if (reconnectTimer) { clearTimeout(reconnectTimer); reconnectTimer = null; }
    if (source) { source.close(); source = null; }
  }

  function connect() {
    if (closed || !isTabVisible()) return;
    disconnect();
    retryCount = 0;

    const url = lastEventId ? `${path}?lastEventId=${lastEventId}` : path;
    source = new EventSource(url);

    source.onopen = () => { retryCount = 0; };

    for (const [eventType, handler] of Object.entries(handlers)) {
      source.addEventListener(eventType, ((event: MessageEvent) => {
        if (event.lastEventId) lastEventId = event.lastEventId;
        try { handler(JSON.parse(event.data)); }
        catch { handler(event.data); }
      }) as EventListener);
    }

    source.onerror = () => {
      source?.close();
      source = null;
      if (closed || !isTabVisible()) return;
      retryCount++;
      if (retryCount >= MAX_RETRIES) return;
      const delay = Math.min(1000 * Math.pow(2, retryCount), MAX_RETRY_DELAY);
      reconnectTimer = setTimeout(connect, delay * (0.5 + Math.random() * 0.5));
    };
  }

  function handleUnload() { closed = true; disconnect(); }
  window.addEventListener('beforeunload', handleUnload);

  const unsubVisibility = onVisibilityChange((visible) => {
    if (closed) return;
    if (visible) connect();
    else disconnect();
  });

  reconnectTimer = setTimeout(connect, 500);

  return {
    close() {
      closed = true;
      window.removeEventListener('beforeunload', handleUnload);
      unsubVisibility();
      disconnect();
    },
  };
}
