import { useEffect, useState } from 'preact/hooks';
import { api } from '@lib/api';
import Button from '@islands/shared/Button/Button';
import Select from '@islands/shared/Select/Select';
import { Table, Play, ArrowUpDown, ChevronLeft, ChevronRight, Terminal, ChevronsUpDown, ChevronsDownUp } from 'lucide-preact';
import Textarea from '@islands/shared/Textarea/Textarea';
import styles from './database-browser.module.css';

type Props = {
  dbId: string;
  dbType: string;
}

type TableResult = {
  columns: string[];
  rows: string[][];
  row_count: number;
}

type DocumentResult = {
  documents: any[];
  row_count: number;
}

type SortState = {
  column: number;
  direction: 'asc' | 'desc';
}

const PAGE_SIZE = 25;

const LABELS: Record<string, { items: string; placeholder: string }> = {
  postgres: { items: 'Tables', placeholder: 'SELECT * FROM users WHERE active = true LIMIT 50' },
  mysql: { items: 'Tables', placeholder: 'SELECT * FROM users WHERE active = true LIMIT 50' },
  mongo: { items: 'Collections', placeholder: '{ "active": true }' },
  redis: { items: 'Keys', placeholder: 'GET mykey' },
};

const REDIS_COMMANDS: Record<string, (key: string) => string> = {
  string: (k) => `GET ${k}`,
  hash: (k) => `HGETALL ${k}`,
  list: (k) => `LRANGE ${k} 0 99`,
  set: (k) => `SMEMBERS ${k}`,
  zset: (k) => `ZRANGE ${k} 0 99 WITHSCORES`,
};

export default function DatabaseBrowser({ dbId, dbType }: Props) {
  const [items, setItems] = useState<string[]>([]);
  const [itemTypes, setItemTypes] = useState<Record<string, string>>({});
  const [selectedItem, setSelectedItem] = useState('');
  const [tableResult, setTableResult] = useState<TableResult | null>(null);
  const [docResult, setDocResult] = useState<DocumentResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [customQuery, setCustomQuery] = useState('');
  const [showCustom, setShowCustom] = useState(false);
  const [sort, setSort] = useState<SortState | null>(null);
  const [filterColumn, setFilterColumn] = useState('');
  const [filterValue, setFilterValue] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const [page, setPage] = useState(0);
  const [expandedDocs, setExpandedDocs] = useState<Set<number>>(new Set());

  const isMongo = dbType === 'mongo';
  const labels = LABELS[dbType] || LABELS.postgres;

  useEffect(() => {
    api.listDbTables(dbId).then((res) => {
      setItems(res.data);
      if (res.types) setItemTypes(res.types);
    }).catch(() => {});
  }, [dbId]);

  function defaultQuery(item: string): string {
    switch (dbType) {
      case 'redis': {
        const type = itemTypes[item] || 'string';
        return (REDIS_COMMANDS[type] || REDIS_COMMANDS.string)(item);
      }
      default: return `SELECT * FROM "${item}" LIMIT 100`;
    }
  }

  async function browseItem(item: string) {
    setSelectedItem(item);
    setSort(null);
    setFilterColumn('');
    setFilterValue('');
    setSearchQuery('');
    setPage(0);
    setExpandedDocs(new Set());
    if (isMongo) {
      // Mongo browse = a find({}) on the collection. The server takes a
      // structured query, never a raw string.
      await runMongoQuery(item, {});
    } else {
      await runQuery(defaultQuery(item));
    }
  }

  async function runQuery(q: string) {
    setLoading(true);
    setError('');
    setTableResult(null);
    setDocResult(null);
    try {
      const data = await api.queryDb(dbId, q);
      if (data.documents) {
        setDocResult({ documents: data.documents, row_count: data.row_count });
      } else {
        setTableResult({ columns: data.columns || [], rows: data.rows || [], row_count: data.row_count });
      }
    } catch (e: any) {
      setError(e.message || 'Query failed');
    }
    setLoading(false);
  }

  async function runMongoQuery(collection: string, filter: unknown) {
    setLoading(true);
    setError('');
    setTableResult(null);
    setDocResult(null);
    try {
      const data = await api.queryMongo(dbId, { collection, filter, limit: 100 });
      setDocResult({ documents: data.documents || [], row_count: data.row_count });
    } catch (e: any) {
      setError(e.message || 'Query failed');
    }
    setLoading(false);
  }

  async function runCustom() {
    setSort(null);
    setPage(0);
    setExpandedDocs(new Set());

    if (isMongo) {
      // The custom panel for Mongo is a JSON filter against the selected
      // collection — parsed and validated client-side before sending.
      if (!selectedItem) {
        setError('Select a collection first.');
        return;
      }
      let filter: unknown = {};
      const text = customQuery.trim();
      if (text) {
        try {
          filter = JSON.parse(text);
        } catch {
          setError('Filter must be valid JSON, e.g. { "active": true }');
          return;
        }
      }
      await runMongoQuery(selectedItem, filter);
      return;
    }

    if (!customQuery.trim()) return;
    setSelectedItem('');
    await runQuery(customQuery);
  }

  function toggleDoc(idx: number) {
    setExpandedDocs(prev => {
      const next = new Set(prev);
      if (next.has(idx)) next.delete(idx); else next.add(idx);
      return next;
    });
  }

  function expandAll() {
    const filtered = getFilteredDocs();
    setExpandedDocs(new Set(filtered.map((_, i) => i)));
  }

  function collapseAll() {
    setExpandedDocs(new Set());
  }

  function getFilteredDocs(): any[] {
    if (!docResult) return [];
    if (!searchQuery) return docResult.documents;
    const lower = searchQuery.toLowerCase();
    return docResult.documents.filter(doc => JSON.stringify(doc).toLowerCase().includes(lower));
  }

  function docPreview(doc: any): string {
    const id = doc._id?.$oid || doc._id || doc.id || '';
    const keys = Object.keys(doc).filter(k => k !== '_id').slice(0, 3);
    const parts = keys.map(k => {
      const v = doc[k];
      const display = typeof v === 'string' ? `"${v}"` : JSON.stringify(v);
      return `${k}: ${display.length > 30 ? display.slice(0, 30) + '...' : display}`;
    });
    return id ? `{ _id: "${id}", ${parts.join(', ')} }` : `{ ${parts.join(', ')} }`;
  }

  // Table view helpers
  function handleSort(colIdx: number) {
    const newDir = sort?.column === colIdx && sort.direction === 'asc' ? 'desc' : 'asc';
    setSort({ column: colIdx, direction: newDir });
    setPage(0);
  }

  function getSortedRows(): string[][] {
    if (!tableResult) return [];
    let rows = [...tableResult.rows];
    if (filterColumn && filterValue) {
      const colIdx = tableResult.columns.indexOf(filterColumn);
      if (colIdx >= 0) {
        const lower = filterValue.toLowerCase();
        rows = rows.filter(r => r[colIdx]?.toLowerCase().includes(lower));
      }
    }
    if (sort) {
      rows.sort((a, b) => {
        const aVal = a[sort.column] || '';
        const bVal = b[sort.column] || '';
        const numA = Number(aVal);
        const numB = Number(bVal);
        const cmp = !isNaN(numA) && !isNaN(numB) ? numA - numB : aVal.localeCompare(bVal);
        return sort.direction === 'asc' ? cmp : -cmp;
      });
    }
    return rows;
  }

  const filteredDocs = getFilteredDocs();
  const sortedRows = getSortedRows();
  const totalItems = isMongo ? filteredDocs.length : sortedRows.length;
  const totalPages = Math.ceil(totalItems / PAGE_SIZE);
  const pagedRows = sortedRows.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE);
  const pagedDocs = filteredDocs.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE);

  return (
    <div class={styles.browser}>
      <div class={styles.header}>
        <h2 class={styles.title}>Browse Data</h2>
        <Button variant={showCustom ? 'primary' : 'ghost'} size="sm" onClick={() => setShowCustom(!showCustom)}>
          <Terminal size={14} aria-hidden="true" /> Custom Query
        </Button>
      </div>

      {showCustom && (
        <div class={styles.queryCard}>
          <Textarea
            label={
              dbType === 'redis'
                ? 'Redis command (read-only)'
                : dbType === 'mongo'
                  ? `JSON filter for ${selectedItem || 'a selected collection'} (read-only)`
                  : 'SQL query (read-only)'
            }
            name="custom-query"
            id="custom-query"
            value={customQuery}
            onChange={setCustomQuery}
            placeholder={labels.placeholder}
            rows={3}
          />
          <div class={styles.queryActions}>
            <Button
              variant="primary"
              size="sm"
              onClick={runCustom}
              loading={loading}
              disabled={isMongo ? !selectedItem : !customQuery.trim()}
            >
              <Play size={12} aria-hidden="true" /> Run
            </Button>
          </div>
        </div>
      )}

      {items.length > 0 && (
        <div class={styles.tableList}>
          <span class={styles.tableListLabel}>{labels.items}</span>
          <div class={styles.tablePills}>
            {items.map(t => (
              <button
                key={t}
                type="button"
                onClick={() => browseItem(t)}
                class={`${styles.tablePill} ${selectedItem === t ? styles.tablePillActive : ''}`}
                aria-pressed={selectedItem === t}
              >
                <Table size={12} aria-hidden="true" />
                {t}
                {dbType === 'redis' && itemTypes[t] && (
                  <span class={`${styles.typeBadge} ${styles[`type_${itemTypes[t]}`] || ''}`}>
                    {itemTypes[t]}
                  </span>
                )}
              </button>
            ))}
          </div>
        </div>
      )}

      {items.length === 0 && !loading && (
        <p class={styles.emptyText}>
          {dbType === 'redis' ? 'No keys found.' : dbType === 'mongo' ? 'No collections found.' : 'No tables found.'}
        </p>
      )}

      {error && <div class={styles.error} role="alert">{error}</div>}

      {/* MongoDB document view */}
      {docResult && docResult.documents.length > 0 && (
        <>
          <div class={styles.docToolbar}>
            <div class={styles.docToolbarLeft}>
              <input
                class={styles.docSearch}
                type="text"
                value={searchQuery}
                onInput={e => { setSearchQuery((e.target as HTMLInputElement).value); setPage(0); setExpandedDocs(new Set()); }}
                placeholder="Search documents..."
                aria-label="Search documents"
              />
              <span class={styles.rowCount} role="status" aria-live="polite">
                {filteredDocs.length} document{filteredDocs.length !== 1 ? 's' : ''}
              </span>
            </div>
            <div class={styles.docToolbarRight}>
              <Button variant="ghost" size="sm" onClick={expandAll}>
                <ChevronsUpDown size={14} aria-hidden="true" /> Expand all
              </Button>
              <Button variant="ghost" size="sm" onClick={collapseAll}>
                <ChevronsDownUp size={14} aria-hidden="true" /> Collapse all
              </Button>
            </div>
          </div>

          <div class={styles.docList} role="list" aria-label="Documents">
            {pagedDocs.map((doc, i) => {
              const globalIdx = page * PAGE_SIZE + i;
              const isExpanded = expandedDocs.has(globalIdx);
              return (
                <div key={globalIdx} class={styles.docItem} role="listitem">
                  <button
                    type="button"
                    class={styles.docToggle}
                    onClick={() => toggleDoc(globalIdx)}
                    aria-expanded={isExpanded}
                  >
                    <span class={styles.docIndex}>{globalIdx + 1}</span>
                    <code class={styles.docSummary}>{docPreview(doc)}</code>
                  </button>
                  {isExpanded && (
                    <pre class={styles.docBody}><JsonHighlight value={doc} /></pre>
                  )}
                </div>
              );
            })}
          </div>

          {totalPages > 1 && (
            <Pagination page={page} totalPages={totalPages} setPage={setPage} />
          )}
        </>
      )}

      {docResult && docResult.documents.length === 0 && (
        <p class={styles.emptyText}>No documents found.</p>
      )}

      {/* Table view for SQL and Redis */}
      {tableResult && tableResult.columns && tableResult.columns.length > 0 && (
        <>
          {tableResult.columns.length > 1 && (
            <div class={styles.filterRow}>
              <Select
                options={[{ value: '', label: 'Filter by column...' }, ...tableResult.columns.map(c => ({ value: c, label: c }))]}
                value={filterColumn}
                onChange={(v) => { setFilterColumn(v); setPage(0); }}
                aria-label="Filter by column"
                size="sm"
                id="db-filter-col"
              />
              {filterColumn && (
                <input
                  class={styles.filterInput}
                  type="text"
                  value={filterValue}
                  onInput={e => { setFilterValue((e.target as HTMLInputElement).value); setPage(0); }}
                  placeholder={`Filter ${filterColumn}...`}
                  aria-label={`Filter value for ${filterColumn}`}
                />
              )}
              <span class={styles.rowCount}>{sortedRows.length} row{sortedRows.length !== 1 ? 's' : ''}</span>
            </div>
          )}

          <div class={styles.tableWrap}>
            <table class={styles.dataTable}>
              <thead>
                <tr>
                  {tableResult.columns.map((col, i) => (
                    <th key={col} class={styles.th}>
                      <button type="button" class={styles.sortButton} onClick={() => handleSort(i)} aria-label={`Sort by ${col}`}>
                        {col}
                        <ArrowUpDown size={10} aria-hidden="true" class={sort?.column === i ? styles.sortActive : styles.sortInactive} />
                      </button>
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {pagedRows.map((row, ri) => (
                  <tr key={ri} class={styles.dataRow}>
                    {row.map((cell, ci) => (
                      <td key={ci} class={styles.td} title={cell}>
                        {!cell || cell === 'null' ? <span class={styles.nullCell}>NULL</span> : cell}
                      </td>
                    ))}
                  </tr>
                ))}
                {pagedRows.length === 0 && (
                  <tr><td class={styles.emptyCell} colSpan={tableResult.columns.length}>No rows match the filter.</td></tr>
                )}
              </tbody>
            </table>
          </div>

          {totalPages > 1 && (
            <Pagination page={page} totalPages={totalPages} setPage={setPage} />
          )}
        </>
      )}

      {loading && !tableResult && !docResult && <p class={styles.loadingText}>Querying database...</p>}
    </div>
  );
}

function Pagination({ page, totalPages, setPage }: { page: number; totalPages: number; setPage: (fn: (p: number) => number) => void }) {
  return (
    <div class={styles.pagination}>
      <Button variant="ghost" size="sm" disabled={page === 0} onClick={() => setPage(p => p - 1)}>
        <ChevronLeft size={14} aria-hidden="true" /> Previous
      </Button>
      <span class={styles.pageInfo}>Page {page + 1} of {totalPages}</span>
      <Button variant="ghost" size="sm" disabled={page >= totalPages - 1} onClick={() => setPage(p => p + 1)}>
        Next <ChevronRight size={14} aria-hidden="true" />
      </Button>
    </div>
  );
}

function JsonHighlight({ value, indent = 0 }: { value: any; indent?: number }) {
  const pad = '  '.repeat(indent);
  const padInner = '  '.repeat(indent + 1);

  if (value === null) return <span class={styles.jsonNull}>null</span>;
  if (typeof value === 'boolean') return <span class={styles.jsonBool}>{value.toString()}</span>;
  if (typeof value === 'number') return <span class={styles.jsonNumber}>{value}</span>;
  if (typeof value === 'string') return <span class={styles.jsonString}>"{value}"</span>;

  if (Array.isArray(value)) {
    if (value.length === 0) return <span>{'[]'}</span>;
    return (
      <span>
        {'[\n'}
        {value.map((item, i) => (
          <span key={i}>
            {padInner}<JsonHighlight value={item} indent={indent + 1} />
            {i < value.length - 1 ? ',\n' : '\n'}
          </span>
        ))}
        {pad}{']'}
      </span>
    );
  }

  if (typeof value === 'object') {
    if (value.$oid) return <span class={styles.jsonString}>ObjectId("{value.$oid}")</span>;
    if (value.$date) return <span class={styles.jsonString}>ISODate("{value.$date}")</span>;

    const keys = Object.keys(value);
    if (keys.length === 0) return <span>{'{}'}</span>;
    return (
      <span>
        {'{\n'}
        {keys.map((key, i) => (
          <span key={key}>
            {padInner}<span class={styles.jsonKey}>"{key}"</span>: <JsonHighlight value={value[key]} indent={indent + 1} />
            {i < keys.length - 1 ? ',\n' : '\n'}
          </span>
        ))}
        {pad}{'}'}
      </span>
    );
  }

  return <span>{String(value)}</span>;
}
