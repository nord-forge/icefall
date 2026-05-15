type Callback = (visible: boolean) => void;

const listeners = new Set<Callback>();

if (typeof document !== 'undefined') {
  document.addEventListener('visibilitychange', () => {
    const visible = !document.hidden;
    listeners.forEach((cb) => cb(visible));
  });
}

export function onVisibilityChange(cb: Callback): () => void {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

export function isTabVisible(): boolean {
  return typeof document !== 'undefined' ? !document.hidden : true;
}

/**
 * Creates an interval that automatically pauses when the tab is hidden
 * and resumes when visible. Returns a cleanup function.
 */
export function createVisibleInterval(
  callback: () => void,
  ms: number,
): () => void {
  let timer: ReturnType<typeof setInterval> | null = null;

  function start() {
    if (timer) return;
    timer = setInterval(callback, ms);
  }

  function stop() {
    if (timer) {
      clearInterval(timer);
      timer = null;
    }
  }

  const unsubscribe = onVisibilityChange((visible) => {
    if (visible) {
      callback();
      start();
    } else {
      stop();
    }
  });

  if (isTabVisible()) start();

  return () => {
    stop();
    unsubscribe();
  };
}
