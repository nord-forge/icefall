import { useState, useMemo } from 'preact/hooks';
import { ChevronRight, ChevronDown } from 'lucide-preact';
import styles from './smart-log-filter.module.css';

export type FilteredLine = {
  text: string;
  count: number;
  isHighlighted: boolean;
  isCollapsed: boolean;
  gapBefore: number | null;
};

type Props = {
  lines: string[];
  noisePatterns: string[];
  highlightPatterns: string[];
};

/** Time gap threshold in seconds before showing a gap marker. */
const GAP_THRESHOLD = 30;

/** Regex to extract an ISO-ish timestamp from the start of a log line. */
const TS_REGEX = /^(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2})/;

/** Check if a line looks like part of a stack trace. */
function isStackLine(line: string): boolean {
  return /^\s+at\s/.test(line) || /^\s+\.\.\./.test(line) || /^\s+Caused by:/.test(line);
}

function extractTimestamp(line: string): number | null {
  const match = line.match(TS_REGEX);
  if (!match) return null;
  const ms = Date.parse(match[1]);
  return isNaN(ms) ? null : ms;
}

/** Patterns longer than this are matched literally — a guard against ReDoS
 *  from a catastrophic-backtracking regex in user-supplied log patterns. */
const MAX_REGEX_PATTERN_LENGTH = 200;

function matchesAny(line: string, patterns: string[]): boolean {
  return patterns.some((p) => {
    if (!p) return false;
    if (p.length > MAX_REGEX_PATTERN_LENGTH) {
      return line.toLowerCase().includes(p.toLowerCase());
    }
    try {
      return new RegExp(p, 'i').test(line);
    } catch {
      return line.toLowerCase().includes(p.toLowerCase());
    }
  });
}

export default function SmartLogFilter({ lines, noisePatterns, highlightPatterns }: Props) {
  const [smartMode, setSmartMode] = useState(true);
  const [expandedStacks, setExpandedStacks] = useState<Set<number>>(new Set());

  const filteredLines = useMemo<FilteredLine[]>(() => {
    if (!smartMode) {
      return lines.map((text) => ({
        text,
        count: 1,
        isHighlighted: false,
        isCollapsed: false,
        gapBefore: null,
      }));
    }

    const result: FilteredLine[] = [];
    let lastTimestamp: number | null = null;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];

      // Skip noise lines
      if (noisePatterns.length > 0 && matchesAny(line, noisePatterns)) {
        continue;
      }

      // Calculate time gap
      const ts = extractTimestamp(line);
      let gapBefore: number | null = null;
      if (ts !== null && lastTimestamp !== null) {
        const gapSec = (ts - lastTimestamp) / 1000;
        if (gapSec >= GAP_THRESHOLD) {
          gapBefore = gapSec;
        }
      }
      if (ts !== null) lastTimestamp = ts;

      // Group consecutive identical lines
      const prev = result[result.length - 1];
      if (prev && prev.text === line && !prev.isCollapsed) {
        prev.count += 1;
        continue;
      }

      const isHighlighted = highlightPatterns.length > 0 && matchesAny(line, highlightPatterns);
      const isCollapsed = isStackLine(line);

      result.push({
        text: line,
        count: 1,
        isHighlighted,
        isCollapsed,
        gapBefore,
      });
    }

    return result;
  }, [lines, noisePatterns, highlightPatterns, smartMode]);

  function toggleStack(index: number) {
    setExpandedStacks((prev) => {
      const next = new Set(prev);
      if (next.has(index)) {
        next.delete(index);
      } else {
        next.add(index);
      }
      return next;
    });
  }

  function formatGap(seconds: number): string {
    if (seconds < 60) return `${Math.round(seconds)}s gap`;
    if (seconds < 3600) return `${Math.round(seconds / 60)}m gap`;
    return `${(seconds / 3600).toFixed(1)}h gap`;
  }

  return (
    <div>
      <div class={styles.toolbar} role="toolbar" aria-label="Log display mode">
        <div class={styles.modeSwitch} role="radiogroup" aria-label="Log filter mode">
          <button
            type="button"
            role="radio"
            aria-checked={smartMode}
            class={`${styles.modeButton} ${smartMode ? styles.modeButtonActive : ''}`}
            onClick={() => setSmartMode(true)}
          >
            Smart
          </button>
          <button
            type="button"
            role="radio"
            aria-checked={!smartMode}
            class={`${styles.modeButton} ${!smartMode ? styles.modeButtonActive : ''}`}
            onClick={() => setSmartMode(false)}
          >
            Raw
          </button>
        </div>
        {/* a11y [WCAG 4.1.3]: line count announced to AT */}
        <span class={styles.lineCount} role="status" aria-live="polite">
          {filteredLines.length} line{filteredLines.length !== 1 ? 's' : ''}
        </span>
      </div>

      <div role="log" aria-label="Filtered log output" aria-live="polite">
        {filteredLines.map((line, i) => {
          // Time gap marker
          const gapMarker = line.gapBefore !== null ? (
            <div class={styles.timeGap} aria-label={formatGap(line.gapBefore)}>
              <span>{formatGap(line.gapBefore)}</span>
            </div>
          ) : null;

          // Stack trace line
          if (line.isCollapsed && !expandedStacks.has(i)) {
            return (
              <div key={i}>
                {gapMarker}
                <div
                  class={styles.stackTrace}
                  onClick={() => toggleStack(i)}
                  onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleStack(i); } }}
                  role="button"
                  tabIndex={0}
                  aria-expanded={false}
                  aria-label="Expand stack trace line"
                >
                  <ChevronRight size={12} aria-hidden="true" />
                  <span>{line.text.trim().slice(0, 60)}...</span>
                </div>
              </div>
            );
          }

          if (line.isCollapsed && expandedStacks.has(i)) {
            return (
              <div key={i}>
                {gapMarker}
                <div
                  class={styles.stackTrace}
                  onClick={() => toggleStack(i)}
                  onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleStack(i); } }}
                  role="button"
                  tabIndex={0}
                  aria-expanded={true}
                  aria-label="Collapse stack trace line"
                >
                  <ChevronDown size={12} aria-hidden="true" />
                </div>
                <div class={styles.stackTraceContent}>
                  <span class={styles.line}>{line.text}</span>
                </div>
              </div>
            );
          }

          return (
            <div key={i}>
              {gapMarker}
              <div class={`${styles.line} ${line.isHighlighted ? styles.highlighted : ''}`}>
                <span>{line.text}</span>
                {line.count > 1 && (
                  <span class={styles.countBadge} aria-label={`Repeated ${line.count} times`}>
                    x{line.count}
                  </span>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
