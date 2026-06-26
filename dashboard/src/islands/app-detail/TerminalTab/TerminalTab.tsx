import { useEffect, useRef, useState, useCallback } from 'preact/hooks';
import type { ComponentChildren } from 'preact';
import { Terminal as XTerm } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import 'xterm/css/xterm.css';
import styles from './terminal-tab.module.css';

type ConnectionStatus = 'disconnected' | 'connecting' | 'connected';

type Props = {
  appId: string;
  /** WebSocket endpoint path (after /api/v1). Defaults to the app terminal. */
  wsPath?: string;
  /** Optional warning banner shown above the terminal (e.g. live-database notice). */
  warning?: string;
  /** Copy for the empty-state hint before connecting. */
  emptyHint?: ComponentChildren;
};

function getWebSocketUrl(path: string): string {
  const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${proto}//${window.location.host}/api/v1${path}`;
}

export default function TerminalTab({ appId, wsPath, warning, emptyHint }: Props) {
  const resolvedPath = wsPath ?? `/apps/${appId}/terminal`;
  const terminalRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const [status, setStatus] = useState<ConnectionStatus>('disconnected');

  // Connecting just flips status. The terminal itself is created in an effect
  // AFTER the connected-view DOM (and thus terminalRef) is committed — calling
  // term.open() synchronously here would hit a still-null terminalRef (the empty
  // state had unmounted that div), leaving an empty container that can't focus.
  const connect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    if (xtermRef.current) {
      xtermRef.current.dispose();
      xtermRef.current = null;
    }
    setStatus('connecting');
  }, []);

  const openTerminal = useCallback(() => {
    const term = new XTerm({
      cursorBlink: true,
      fontFamily: 'var(--font-mono), "Fira Code", "JetBrains Mono", "Cascadia Code", monospace',
      fontSize: 14,
      lineHeight: 1.4,
      scrollback: 1000,
      theme: {
        background: '#1c1f2b',
        foreground: '#e2e4ed',
        cursor: '#e2e4ed',
        selectionBackground: '#3a3f55',
        black: '#1c1f2b',
        red: '#e06c75',
        green: '#98c379',
        yellow: '#e5c07b',
        blue: '#61afef',
        magenta: '#c678dd',
        cyan: '#56b6c2',
        white: '#e2e4ed',
        brightBlack: '#7c8294',
        brightRed: '#e06c75',
        brightGreen: '#98c379',
        brightYellow: '#e5c07b',
        brightBlue: '#61afef',
        brightMagenta: '#c678dd',
        brightCyan: '#56b6c2',
        brightWhite: '#ffffff',
      },
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);

    xtermRef.current = term;
    fitAddonRef.current = fitAddon;

    if (terminalRef.current) {
      term.open(terminalRef.current);
      // The container can still be 0×0 on the frame after open() (flex/grid
      // layout not settled), which makes fit() compute a 0-row grid — xterm then
      // renders no rows and no focusable textarea, so the terminal looks blank
      // and can't be clicked into. Retry fit until the container has real
      // dimensions, then focus so the user can type immediately.
      const tryFit = (attempt: number) => {
        const el = terminalRef.current;
        if (!el) return;
        if (el.clientHeight > 0 && el.clientWidth > 0) {
          try {
            fitAddon.fit();
          } catch {
            // ignore transient fit errors
          }
          term.focus();
          // Push the freshly-fit size to the backend so the PTY matches.
          const dims = fitAddon.proposeDimensions();
          if (dims && wsRef.current?.readyState === WebSocket.OPEN) {
            wsRef.current.send(JSON.stringify({ type: 'resize', cols: dims.cols, rows: dims.rows }));
          }
          return;
        }
        if (attempt < 20) {
          requestAnimationFrame(() => tryFit(attempt + 1));
        }
      };
      requestAnimationFrame(() => tryFit(0));
    }

    const url = getWebSocketUrl(resolvedPath);
    const ws = new WebSocket(url);
    ws.binaryType = 'arraybuffer';
    wsRef.current = ws;

    ws.onopen = () => {
      setStatus('connected');
      // Send initial terminal size
      try {
        fitAddon.fit();
      } catch {
        // Ignore fit errors during connect
      }
      const dims = fitAddon.proposeDimensions();
      if (dims) {
        ws.send(JSON.stringify({
          type: 'resize',
          cols: dims.cols,
          rows: dims.rows,
        }));
      }
      term.focus();
    };

    ws.onmessage = (event: MessageEvent) => {
      if (event.data instanceof ArrayBuffer) {
        term.write(new Uint8Array(event.data));
      } else if (typeof event.data === 'string') {
        term.write(event.data);
      }
    };

    ws.onclose = () => {
      setStatus('disconnected');
      term.write('\r\n\x1b[33m--- Session ended ---\x1b[0m\r\n');
    };

    ws.onerror = () => {
      setStatus('disconnected');
    };

    // Pipe terminal input to WebSocket
    term.onData((data: string) => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(data);
      }
    });

    // Handle binary data from terminal (e.g., paste)
    term.onBinary((data: string) => {
      if (ws.readyState === WebSocket.OPEN) {
        const bytes = new Uint8Array(data.length);
        for (let i = 0; i < data.length; i++) {
          bytes[i] = data.charCodeAt(i);
        }
        ws.send(bytes.buffer);
      }
    });
  }, [resolvedPath]);

  // Once status flips to 'connecting', the connected view (with terminalRef) has
  // rendered — now it's safe to create and open the terminal. Guard with the ref
  // so we only open once per connection.
  useEffect(() => {
    if (status === 'connecting' && terminalRef.current && !xtermRef.current) {
      openTerminal();
    }
  }, [status, openTerminal]);

  // Handle terminal resize
  useEffect(() => {
    function handleResize() {
      const fitAddon = fitAddonRef.current;
      const ws = wsRef.current;
      if (!fitAddon) return;

      try {
        fitAddon.fit();
      } catch {
        return;
      }

      if (ws && ws.readyState === WebSocket.OPEN) {
        const dims = fitAddon.proposeDimensions();
        if (dims) {
          ws.send(JSON.stringify({
            type: 'resize',
            cols: dims.cols,
            rows: dims.rows,
          }));
        }
      }
    }

    window.addEventListener('resize', handleResize);

    // Also observe the container element for size changes
    const container = terminalRef.current;
    let resizeObserver: ResizeObserver | null = null;
    if (container && typeof ResizeObserver !== 'undefined') {
      resizeObserver = new ResizeObserver(handleResize);
      resizeObserver.observe(container);
    }

    return () => {
      window.removeEventListener('resize', handleResize);
      resizeObserver?.disconnect();
    };
  }, []);

  // Clean up on unmount
  useEffect(() => {
    return () => {
      wsRef.current?.close();
      xtermRef.current?.dispose();
    };
  }, []);

  function handleDisconnect() {
    wsRef.current?.close();
    wsRef.current = null;
    // Dispose and clear the xterm instance too, so the empty-state guard
    // (status === 'disconnected' && !xtermRef.current) tears the terminal UI
    // back down to the Connect screen instead of leaving a dead black panel.
    xtermRef.current?.dispose();
    xtermRef.current = null;
    setStatus('disconnected');
  }

  const statusLabel =
    status === 'connected' ? 'Connected' :
    status === 'connecting' ? 'Connecting...' :
    'Disconnected';

  const statusDotClass =
    status === 'connected' ? styles.statusConnected :
    status === 'connecting' ? styles.statusConnecting :
    styles.statusDisconnected;

  if (status === 'disconnected' && !xtermRef.current) {
    return (
      <div class={styles.terminal}>
        <div class={styles.emptyState}>
          <p class={styles.emptyStateTitle}>Terminal</p>
          <p class={styles.emptyStateHint}>
            {emptyHint ?? (
              <>Open a shell session into your running container. The app must have at least one <a href={`/apps/${appId}/deploys`} data-astro-prefetch="hover">running deploy</a>.</>
            )}
          </p>
          <button
            type="button"
            class={styles.connectButton}
            onClick={connect}
          >
            Connect
          </button>
        </div>
      </div>
    );
  }

  return (
    <div class={styles.terminal}>
      {warning && (
        // a11y [WCAG 4.1.3]: surfaced as an alert so AT announces the live-DB risk
        <div class={styles.warningBanner} role="alert">{warning}</div>
      )}
      <div class={styles.toolbar}>
        <div class={styles.toolbarLeft}>
          {/* a11y [WCAG 4.1.3]: announce connection status to AT */}
          <div class={styles.statusIndicator} role="status" aria-live="polite">
            <span class={statusDotClass} aria-hidden="true" />
            {statusLabel}
          </div>
        </div>
        <div class={styles.toolbarRight}>
          {status === 'disconnected' && (
            <button
              type="button"
              class={styles.ghostButton}
              onClick={connect}
            >
              Reconnect
            </button>
          )}
          {/* After a session ends the xterm panel lingers; offer Close to tear it
              back down to the Connect screen, not just Reconnect. */}
          {(status === 'connected' || (status === 'disconnected' && xtermRef.current)) && (
            <button
              type="button"
              class={styles.ghostButton}
              onClick={handleDisconnect}
            >
              Close terminal
            </button>
          )}
        </div>
      </div>
      {/* a11y [WCAG 4.1.2]: xterm container is an interactive terminal region */}
      <div
        ref={terminalRef}
        class={styles.terminalContainer}
        role="application"
        aria-label="Terminal session"
      />
    </div>
  );
}
