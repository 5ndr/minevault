import React, { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Folder,
  File,
  RotateCcw,
  Search,
  CheckCircle,
  AlertTriangle,
  FolderOpen,
  Filter,
  CheckSquare,
  Square,
  Pin,
  Archive,
  Trash2,
  AlertOctagon,
  X,
  ChevronRight,
  Home,
  CornerLeftUp,
  ChevronLeft,
  RefreshCw,
  Gamepad2,
  FolderArchive,
  Info,
  Box,
  Layers,
} from 'lucide-react';
import { ResticSnapshot, ResticNode, RestoreOptions } from '../types';
import {
  formatBytes,
  formatExactTimestamp,
  formatRelativeTime,
  formatDateShort,
} from '../utils/format';
import { detectLauncherFromSnapshot } from '../utils/launcher';

interface SnapshotBrowserProps {
  snapshots: ResticSnapshot[];
  selectedSnapshot: ResticSnapshot | null;
  onSelectSnapshot: (snapshot: ResticSnapshot) => void;
  nodes: ResticNode[];
  loadingNodes: boolean;
  nodesError?: string | null;
  onRetryNodes?: () => void;
  onRestore: (options: RestoreOptions) => Promise<void>;
  restoring: boolean;
  restoreResult: string | null;
  restoreError: string | null;
  onRecordRestoreTest: (snapshotId: string, dest: string) => void;
  onTogglePin?: (snapshotId: string, pinned: boolean) => Promise<void>;
  onRunForget?: (keepLast: number) => Promise<void>;
  onDeleteSnapshot?: (snapshotId: string, prune: boolean) => Promise<void>;
  defaultRestoreBaseDir?: string;
}

const FOLDER_PAGE_SIZE = 100;
const SEARCH_PAGE_SIZE = 50;

export const SnapshotBrowser: React.FC<SnapshotBrowserProps> = ({
  snapshots,
  selectedSnapshot,
  onSelectSnapshot,
  nodes,
  loadingNodes,
  nodesError,
  onRetryNodes,
  onRestore,
  restoring,
  restoreResult,
  restoreError,
  onRecordRestoreTest,
  onTogglePin,
  onRunForget,
  onDeleteSnapshot,
  defaultRestoreBaseDir = '~/Restored_Backups',
}) => {
  // Left: Snapshots list filters
  const [hostFilter, setHostFilter] = useState<string>('all');
  const [scopeFilter, setScopeFilter] = useState<'all' | 'launcher' | 'instance'>('all');
  const [searchQuery, setSearchQuery] = useState<string>('');

  // Right: Directory navigation and search
  const [currentPath, setCurrentPath] = useState<string>('');
  const [searchInSnapshot, setSearchInSnapshot] = useState<string>('');
  const [folderPage, setFolderPage] = useState<number>(1);
  const [searchPage, setSearchPage] = useState<number>(1);

  // Restore selection & mode
  const [restoreMode, setRestoreMode] = useState<'launcher' | 'backup_folder' | 'custom'>('launcher');
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const [targetDir, setTargetDir] = useState<string>('');
  const [overwriteConfirmed, setOverwriteConfirmed] = useState<boolean>(true);
  const [showRestoreModal, setShowRestoreModal] = useState<boolean>(false);

  // Prune confirmation modal
  const [showPruneModal, setShowPruneModal] = useState<boolean>(false);
  const [pruneKeepLast, setPruneKeepLast] = useState<number>(10);
  const [pruning, setPruning] = useState<boolean>(false);
  const [pruneResult, setPruneResult] = useState<string | null>(null);

  // Single snapshot deletion modal
  const [showDeleteModal, setShowDeleteModal] = useState<boolean>(false);
  const [deleteTargetSnapshot, setDeleteTargetSnapshot] = useState<ResticSnapshot | null>(null);
  const [deletePrune, setDeletePrune] = useState<boolean>(true);
  const [deleting, setDeleting] = useState<boolean>(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  // Pinning state
  const [pinning, setPinning] = useState<boolean>(false);

  // Set default restore target directory and sensible current path when snapshot changes
  useEffect(() => {
    if (selectedSnapshot) {
      const snapShort = selectedSnapshot.short_id || selectedSnapshot.id?.substring(0, 8) || 'snapshot';
      const defaultSafeFolder = `${defaultRestoreBaseDir.replace(/[/\\]$/, '')}/${snapShort}`;
      const launcherInfo = detectLauncherFromSnapshot(selectedSnapshot);

      if (launcherInfo.originalPath) {
        setRestoreMode('launcher');
        setTargetDir(launcherInfo.originalPath);
        setOverwriteConfirmed(true);
      } else {
        setRestoreMode('backup_folder');
        setTargetDir(defaultSafeFolder);
        setOverwriteConfirmed(false);
      }

      setSelectedPaths(new Set());
      setSearchInSnapshot('');
      setFolderPage(1);
      setSearchPage(1);

      // Default currentPath to snapshot's first source path if available
      const firstPath = selectedSnapshot.paths?.[0];
      if (firstPath && firstPath.startsWith('/')) {
        setCurrentPath(firstPath);
      } else {
        setCurrentPath('');
      }
    }
  }, [selectedSnapshot]);

  const handlePickCustomFolder = async () => {
    try {
      const picked: string | null = await invoke('pick_folder');
      if (picked) {
        setTargetDir(picked);
      }
    } catch (e) {
      console.error('Pick folder error:', e);
    }
  };

  const uniqueHosts = Array.from(new Set(snapshots.map((s) => s.hostname || 'unknown')));

  const filteredSnapshots = snapshots.filter((s) => {
    if (hostFilter !== 'all' && s.hostname !== hostFilter) return false;
    const launcher = detectLauncherFromSnapshot(s);
    if (scopeFilter !== 'all' && launcher.scopeType !== scopeFilter) return false;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      const matchId = (s.short_id || s.id || '').toLowerCase().includes(q);
      const matchPath = s.paths?.some((p) => p.toLowerCase().includes(q));
      const matchTag = s.tags?.some((t) => t.toLowerCase().includes(q));
      const matchLauncher = launcher.name.toLowerCase().includes(q);
      const matchInstance = launcher.instanceName?.toLowerCase().includes(q);
      if (!matchId && !matchPath && !matchTag && !matchLauncher && !matchInstance) return false;
    }
    return true;
  });

  // Calculate direct children of current directory with memoization
  const directChildren = useMemo(() => {
    if (!nodes || nodes.length === 0) return [];

    const normCurrent = currentPath ? currentPath.replace(/\/+$/, '') : '';
    const prefix = normCurrent ? normCurrent + '/' : '/';
    const itemsMap = new Map<string, ResticNode>();

    for (let i = 0; i < nodes.length; i++) {
      const n = nodes[i];
      let p = n.path || '';
      if (!p.startsWith('/')) p = '/' + p;

      if (normCurrent === '') {
        // At root: extract top-level directory/file
        const parts = p.split('/').filter(Boolean);
        if (parts.length > 0) {
          const topDir = '/' + parts[0];
          const isExactTop = parts.length === 1;
          const existing = itemsMap.get(topDir);
          if (!existing) {
            itemsMap.set(topDir, {
              name: parts[0],
              type: isExactTop && n.type === 'file' ? 'file' : 'dir',
              path: topDir,
              size: isExactTop ? n.size : undefined,
              mtime: isExactTop ? n.mtime : undefined,
            });
          } else if (isExactTop && (existing.mtime === undefined || existing.size === undefined)) {
            itemsMap.set(topDir, n);
          }
        }
        continue;
      }

      if (p === normCurrent) {
        // Don't list the directory itself inside itself
        continue;
      }

      if (p.startsWith(prefix)) {
        const rel = p.slice(prefix.length);
        const slashIdx = rel.indexOf('/');
        if (slashIdx === -1) {
          // Direct child of currentPath
          // Overwrite any placeholder that was previously created for this folder
          itemsMap.set(p, n);
        } else {
          // Subdirectory of currentPath
          const dirName = rel.slice(0, slashIdx);
          const dirPath = prefix + dirName;
          if (!itemsMap.has(dirPath)) {
            itemsMap.set(dirPath, {
              name: dirName,
              type: 'dir',
              path: dirPath,
            });
          }
        }
      }
    }

    const items = Array.from(itemsMap.values());

    // Sort: directories first, then alphabetical
    items.sort((a, b) => {
      if (a.type === 'dir' && b.type !== 'dir') return -1;
      if (a.type !== 'dir' && b.type === 'dir') return 1;
      return a.name.localeCompare(b.name);
    });

    return items;
  }, [nodes, currentPath]);

  // Calculate search results across all nodes if searchInSnapshot is active
  const searchResults = useMemo(() => {
    const q = searchInSnapshot.trim().toLowerCase();
    if (!q || !nodes || nodes.length === 0) return [];

    return nodes.filter((n) => {
      const name = n.name?.toLowerCase() || '';
      const path = n.path?.toLowerCase() || '';
      return name.includes(q) || path.includes(q);
    });
  }, [nodes, searchInSnapshot]);

  // Breadcrumbs breakdown
  const breadcrumbs = useMemo(() => {
    if (!currentPath) return [];
    const parts = currentPath.split('/').filter(Boolean);
    let accum = '';
    return parts.map((part) => {
      accum += '/' + part;
      return {
        name: part,
        path: accum,
      };
    });
  }, [currentPath]);

  // Pagination for Folder View
  const totalFolderPages = Math.max(1, Math.ceil(directChildren.length / FOLDER_PAGE_SIZE));
  const displayedFolderItems = useMemo(() => {
    const start = (folderPage - 1) * FOLDER_PAGE_SIZE;
    return directChildren.slice(start, start + FOLDER_PAGE_SIZE);
  }, [directChildren, folderPage]);

  // Pagination for Search View
  const totalSearchPages = Math.max(1, Math.ceil(searchResults.length / SEARCH_PAGE_SIZE));
  const displayedSearchItems = useMemo(() => {
    const start = (searchPage - 1) * SEARCH_PAGE_SIZE;
    return searchResults.slice(start, start + SEARCH_PAGE_SIZE);
  }, [searchResults, searchPage]);

  const handleNavigateToFolder = (folderPath: string) => {
    setCurrentPath(folderPath);
    setFolderPage(1);
  };

  const handleNavigateUp = () => {
    if (!currentPath) return;
    const norm = currentPath.replace(/\/+$/, '');
    const lastSlash = norm.lastIndexOf('/');
    if (lastSlash <= 0) {
      setCurrentPath('');
    } else {
      setCurrentPath(norm.substring(0, lastSlash));
    }
    setFolderPage(1);
  };

  const toggleSelectPath = (path: string) => {
    const next = new Set(selectedPaths);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    setSelectedPaths(next);
  };

  const handleStartRestore = async () => {
    if (!selectedSnapshot) return;
    const launcherInfo = detectLauncherFromSnapshot(selectedSnapshot);

    let effectiveSnapshotId = selectedSnapshot.id;
    let effectiveTargetDir = targetDir;

    if (restoreMode === 'launcher') {
      if (selectedPaths.size === 0 && launcherInfo.originalPath) {
        effectiveSnapshotId = `${selectedSnapshot.id}:${launcherInfo.originalPath}`;
        effectiveTargetDir = launcherInfo.originalPath;
      } else {
        effectiveSnapshotId = selectedSnapshot.id;
        effectiveTargetDir = '/';
      }
    } else if (restoreMode === 'backup_folder') {
      if (selectedPaths.size === 0 && launcherInfo.originalPath) {
        effectiveSnapshotId = `${selectedSnapshot.id}:${launcherInfo.originalPath}`;
        effectiveTargetDir = targetDir;
      }
    }

    await onRestore({
      snapshot_id: effectiveSnapshotId,
      target_dir: effectiveTargetDir,
      include_paths: Array.from(selectedPaths),
      overwrite_confirmed: overwriteConfirmed,
    });
    if (onRecordRestoreTest) {
      onRecordRestoreTest(selectedSnapshot.id, effectiveTargetDir);
    }
  };

  const isSnapshotPinned = Boolean(
    selectedSnapshot?.tags?.includes('pinned') || selectedSnapshot?.tags?.includes('important')
  );

  const handleTogglePinClick = async () => {
    if (!selectedSnapshot || !onTogglePin) return;
    setPinning(true);
    try {
      await onTogglePin(selectedSnapshot.id, !isSnapshotPinned);
    } finally {
      setPinning(false);
    }
  };

  const handleExecutePrune = async () => {
    if (!onRunForget) return;
    setPruning(true);
    setPruneResult(null);
    try {
      await onRunForget(pruneKeepLast);
      setPruneResult(`Pruned repository successfully! Kept last ${pruneKeepLast} backups. Pinned backups preserved.`);
    } catch (err: any) {
      alert(`Prune error: ${err?.toString()}`);
    } finally {
      setPruning(false);
    }
  };

  const handleOpenDeleteModal = (snap: ResticSnapshot, e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    setDeleteTargetSnapshot(snap);
    setDeletePrune(true);
    setDeleteError(null);
    setShowDeleteModal(true);
  };

  const handleExecuteDelete = async () => {
    if (!deleteTargetSnapshot || !onDeleteSnapshot) return;
    setDeleting(true);
    setDeleteError(null);
    try {
      await onDeleteSnapshot(deleteTargetSnapshot.id, deletePrune);
      setShowDeleteModal(false);
      setDeleteTargetSnapshot(null);
    } catch (err: any) {
      setDeleteError(err?.toString() || 'Failed to delete snapshot');
    } finally {
      setDeleting(false);
    }
  };

  const isSearchActive = searchInSnapshot.trim().length > 0;

  return (
    <div className="snapshot-browser-container">
      <div className="browser-split-layout">
        {/* Left: Snapshots List */}
        <div className="card snapshots-list-card">
          <div className="card-header">
            <div className="filter-controls-row">
              <div className="search-box">
                <Search size={14} className="text-muted" />
                <input
                  type="text"
                  placeholder="Filter snapshots by tag, ID..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                />
              </div>

              <div className="host-select-group">
                <Layers size={14} className="text-muted" />
                <select value={scopeFilter} onChange={(e) => setScopeFilter(e.target.value as any)}>
                  <option value="all">All Types ({snapshots.length})</option>
                  <option value="launcher">🏢 Full Launcher</option>
                  <option value="instance">📦 Instances</option>
                </select>
              </div>

              <div className="host-select-group">
                <Filter size={14} className="text-muted" />
                <select value={hostFilter} onChange={(e) => setHostFilter(e.target.value)}>
                  <option value="all">All Hosts ({snapshots.length})</option>
                  {uniqueHosts.map((h) => (
                    <option key={h} value={h}>
                      Host: {h}
                    </option>
                  ))}
                </select>
              </div>

              {onRunForget && (
                <button
                  type="button"
                  onClick={() => setShowPruneModal(true)}
                  className="btn-header-prune"
                  title="Run retention policy to remove older non-pinned backups"
                >
                  <Archive size={13} />
                  <span>Prune</span>
                </button>
              )}
            </div>
          </div>

          <div className="card-body p-0 snapshot-items-list">
            {filteredSnapshots.length === 0 ? (
              <div className="empty-state">No matching snapshots found.</div>
            ) : (
              filteredSnapshots.map((s) => {
                const isSelected = selectedSnapshot?.id === s.id;
                const sPinned = s.tags?.includes('pinned') || s.tags?.includes('important');
                const launcher = detectLauncherFromSnapshot(s);
                const distinctionTag = s.tags?.find(
                  (t) =>
                    !['minecraft', 'manual', 'automatic', 'pinned', 'important', 'prism'].includes(t.toLowerCase()) &&
                    !t.toLowerCase().startsWith('launcher:') &&
                    !t.includes(':') &&
                    isNaN(Number(t))
                );

                return (
                  <div
                    key={s.id}
                    onClick={() => onSelectSnapshot(s)}
                    className={`snapshot-item ${isSelected ? 'active' : ''}`}
                  >
                    <div className="snapshot-item-top">
                      <div className="flex items-center gap-1.5 flex-wrap">
                        <span className="snapshot-short-id">{s.short_id || s.id.substring(0, 8)}</span>
                        {launcher.scopeType === 'instance' ? (
                          <span className="badge-scope-instance" title={`Instance Backup: ${launcher.instanceName || 'Instance'}`}>
                            <Box size={10} />
                            <span>{launcher.instanceName ? `Instance: ${launcher.instanceName}` : 'Instance'}</span>
                          </span>
                        ) : (
                          <span className="badge-scope-launcher" title="Full Launcher Backup (All Instances & Configs)">
                            <Layers size={10} />
                            <span>Full Launcher</span>
                          </span>
                        )}
                        {launcher.name && (
                          <span className="badge-launcher-tag" title={`Launcher: ${launcher.name}`}>
                            🎮 {launcher.name}
                          </span>
                        )}
                        {sPinned && (
                          <span className="badge-pinned-star" title="Protected from retention deletion">
                            <Pin size={10} /> Pinned
                          </span>
                        )}
                      </div>
                      <div className="flex items-center gap-2">
                        <span className="snapshot-host-tag">{s.hostname}</span>
                        <span className="snapshot-time" title={formatExactTimestamp(s.time)}>
                          {formatDateShort(s.time)} ({formatRelativeTime(s.time)})
                        </span>
                        {onDeleteSnapshot && (
                          <button
                            type="button"
                            className="snapshot-item-delete-btn"
                            title="Delete this backup snapshot"
                            onClick={(e) => handleOpenDeleteModal(s, e)}
                          >
                            <Trash2 size={12} />
                          </button>
                        )}
                      </div>
                    </div>

                    {/* Distinction Label / Primary Paths */}
                    {distinctionTag ? (
                      <div className="snapshot-distinction-title truncate">
                        🏷️ {distinctionTag}
                      </div>
                    ) : launcher.scopeType === 'instance' && launcher.instanceName ? (
                      <div className="snapshot-instance-title truncate" title={s.paths?.join(', ')}>
                        <Box size={12} className="text-purple-400 inline mr-1 flex-shrink-0" />
                        <strong className="truncate">{launcher.instanceName}</strong>
                        <span className="text-muted text-xs ml-1.5 flex-shrink-0">({launcher.name})</span>
                      </div>
                    ) : (
                      <div className="snapshot-paths truncate" title={s.paths?.join(', ')}>
                        {s.paths?.join(', ')}
                      </div>
                    )}

                    {s.tags && s.tags.length > 0 && (
                      <div className="snapshot-tags-row">
                        {s.tags.map((t) => {
                          const low = t.toLowerCase();
                          const isSpecial =
                            low === 'pinned' ||
                            low === 'important' ||
                            low === 'minecraft' ||
                            low === 'manual' ||
                            low === 'automatic' ||
                            low.startsWith('launcher:') ||
                            low.startsWith('instance:') ||
                            low.startsWith('scope:') ||
                            low === launcher.id;
                          if (isSpecial) return null;
                          return (
                            <span key={t} className={`tag-pill ${t.includes('_') || t.includes('-') ? 'tag-time' : ''}`}>
                              #{t}
                            </span>
                          );
                        })}
                      </div>
                    )}

                    {s.summary && (
                      <div className="snapshot-item-stats">
                        <span>{s.summary.total_files_processed || 0} files</span>
                        <span>•</span>
                        <span>{formatBytes(s.summary.total_bytes_processed || 0)}</span>
                      </div>
                    )}
                  </div>
                );
              })
            )}
          </div>
        </div>

        {/* Right: Snapshot File Tree Browser & Restore Controls */}
        <div className="card file-browser-card">
          <div className="card-header file-browser-header">
            <div className="file-browser-title flex items-center gap-2 flex-wrap">
              <FolderOpen size={18} className="text-primary" />
              <span>
                Snapshot:{' '}
                <strong>
                  {selectedSnapshot
                    ? selectedSnapshot.short_id || selectedSnapshot.id.substring(0, 8)
                    : 'Select a snapshot'}
                </strong>
              </span>
              {selectedSnapshot && (() => {
                const launcher = detectLauncherFromSnapshot(selectedSnapshot);
                return (
                  <div className="flex items-center gap-1.5 flex-wrap">
                    {launcher.scopeType === 'instance' ? (
                      <span className="badge-scope-instance-lg" title="Minecraft Instance Backup">
                        <Box size={13} />
                        <span>Instance: <strong>{launcher.instanceName || 'Minecraft Instance'}</strong></span>
                      </span>
                    ) : (
                      <span className="badge-scope-launcher-lg" title="Full Launcher Backup (All Instances & Configs)">
                        <Layers size={13} />
                        <span><strong>Full Launcher Backup</strong></span>
                      </span>
                    )}
                    <span className="badge-launcher-tag" title={`Source: ${launcher.originalPath}`}>
                      🎮 {launcher.name}
                    </span>
                  </div>
                );
              })()}
              {selectedSnapshot?.time && (
                <span className="text-muted text-xs">
                  • {formatDateShort(selectedSnapshot.time)}
                </span>
              )}
            </div>

            <div className="file-browser-actions">
              {selectedSnapshot && onTogglePin && (
                <button
                  type="button"
                  onClick={handleTogglePinClick}
                  disabled={pinning}
                  className={`btn-pin-toggle ${isSnapshotPinned ? 'active' : ''}`}
                  title={
                    isSnapshotPinned
                      ? 'Protected: this backup will never be auto-deleted by retention (click to unprotect)'
                      : 'Protect this backup from being pruned by retention limits'
                  }
                >
                  <Pin size={13} />
                  <span>{isSnapshotPinned ? 'Pinned (Protected)' : 'Pin as Important'}</span>
                </button>
              )}

              {selectedSnapshot && onDeleteSnapshot && (
                <button
                  type="button"
                  onClick={() => handleOpenDeleteModal(selectedSnapshot)}
                  className="btn-delete-snapshot"
                  title="Permanently delete this backup snapshot"
                >
                  <Trash2 size={13} />
                  <span>Delete Backup</span>
                </button>
              )}

              <button
                onClick={() => setShowRestoreModal(true)}
                disabled={!selectedSnapshot || restoring}
                className="btn-primary"
              >
                <RotateCcw size={14} />
                <span>
                  {selectedPaths.size > 0
                    ? `Restore Selected (${selectedPaths.size})`
                    : 'Restore Full Snapshot'}
                </span>
              </button>
            </div>
          </div>

          {/* Subbar: Search files inside snapshot & Selection Tracker */}
          <div className="browser-subbar">
            <div className="browser-search-input-wrap">
              <Search size={14} className="text-muted" />
              <input
                type="text"
                placeholder="Search files inside this snapshot (e.g. *.jar, options.txt, saves)..."
                value={searchInSnapshot}
                onChange={(e) => {
                  setSearchInSnapshot(e.target.value);
                  setSearchPage(1);
                }}
              />
              {isSearchActive && (
                <button
                  type="button"
                  onClick={() => setSearchInSnapshot('')}
                  className="btn-ghost p-0"
                  title="Clear search filter"
                >
                  <X size={13} />
                </button>
              )}
            </div>

            <div className="flex items-center gap-2">
              {isSearchActive && (
                <span className="browser-search-badge">
                  🔍 {searchResults.length} matching file(s)
                </span>
              )}

              {selectedPaths.size > 0 ? (
                <div className="flex items-center gap-1.5">
                  <span className="selected-count-badge">
                    ✓ {selectedPaths.size} item(s) selected
                  </span>
                  <button
                    type="button"
                    onClick={() => setSelectedPaths(new Set())}
                    className="btn-secondary text-xs py-0.5 px-2"
                  >
                    Clear
                  </button>
                </div>
              ) : (
                <span className="text-muted text-xs">Click items or checkboxes to select for selective restore</span>
              )}
            </div>
          </div>

          {/* Breadcrumb Path Bar (Shown in Folder View) */}
          {!isSearchActive && (
            <div className="browser-breadcrumbs">
              <button
                type="button"
                className="crumb-btn"
                onClick={() => handleNavigateToFolder('')}
                title="Go to Snapshot Root"
              >
                <Home size={13} />
                <span>Root</span>
              </button>

              {breadcrumbs.map((crumb, idx) => {
                const isLast = idx === breadcrumbs.length - 1;
                return (
                  <React.Fragment key={crumb.path}>
                    <ChevronRight size={12} className="crumb-separator" />
                    {isLast ? (
                      <span className="crumb-current truncate max-w-[200px]" title={crumb.path}>
                        {crumb.name}
                      </span>
                    ) : (
                      <button
                        type="button"
                        className="crumb-btn truncate max-w-[160px]"
                        onClick={() => handleNavigateToFolder(crumb.path)}
                        title={crumb.path}
                      >
                        {crumb.name}
                      </button>
                    )}
                  </React.Fragment>
                );
              })}
            </div>
          )}

          {/* Main Viewport: Either Folder Hierarchy or Search Results */}
          <div className="card-body p-0 file-tree-viewport">
            {loadingNodes ? (
              <div className="tree-loading flex items-center justify-center p-8 gap-2 text-muted">
                <RefreshCw size={16} className="animate-spin text-primary" />
                <span>Loading snapshot file tree from repository...</span>
              </div>
            ) : nodesError ? (
              <div className="empty-state p-6 text-danger">
                <AlertTriangle size={24} className="mb-2" />
                <div>{nodesError}</div>
                {onRetryNodes && (
                  <button onClick={onRetryNodes} className="btn-secondary mt-3">
                    Retry Loading Files
                  </button>
                )}
              </div>
            ) : !selectedSnapshot ? (
              <div className="empty-state p-6">Select a backup snapshot from the left to explore files.</div>
            ) : isSearchActive ? (
              /* --- SEARCH RESULTS VIEW (Paginated) --- */
              searchResults.length === 0 ? (
                <div className="empty-state p-6">No files matching &quot;{searchInSnapshot}&quot; found in this snapshot.</div>
              ) : (
                <div className="file-tree-list">
                  <div className="file-tree-header-row">
                    <span className="col-check">Select</span>
                    <span className="col-name">Matching File / Path</span>
                    <span className="col-size">Logical Size</span>
                    <span className="col-mtime">Modified Time</span>
                  </div>

                  {displayedSearchItems.map((node) => {
                    const isChecked = selectedPaths.has(node.path);
                    return (
                      <div
                        key={node.path}
                        className={`tree-row ${isChecked ? 'row-selected' : ''}`}
                        onClick={() => toggleSelectPath(node.path)}
                      >
                        <span className="col-check" onClick={(e) => e.stopPropagation()}>
                          <button
                            type="button"
                            className="checkbox-icon-btn"
                            onClick={() => toggleSelectPath(node.path)}
                          >
                            {isChecked ? (
                              <CheckSquare size={14} className="text-primary" />
                            ) : (
                              <Square size={14} className="text-muted" />
                            )}
                          </button>
                        </span>

                        <span className="col-name truncate" title={node.path}>
                          {node.type === 'dir' ? (
                            <Folder size={14} className="folder-icon flex-shrink-0" />
                          ) : (
                            <File size={14} className="file-icon flex-shrink-0" />
                          )}
                          <span className="truncate">{node.path}</span>
                        </span>

                        <span className="col-size">
                          {node.size !== undefined ? formatBytes(node.size) : '—'}
                        </span>

                        <span className="col-mtime">
                          {node.mtime ? formatRelativeTime(node.mtime) : '—'}
                        </span>
                      </div>
                    );
                  })}
                </div>
              )
            ) : (
              /* --- FOLDER DRILL-DOWN VIEW (Paginated per directory) --- */
              <div className="file-tree-list">
                <div className="file-tree-header-row">
                  <span className="col-check">Select</span>
                  <span className="col-name">File / Folder Name</span>
                  <span className="col-size">Logical Size</span>
                  <span className="col-mtime">Modified Time</span>
                </div>

                {/* Parent Directory Link (if inside a subdirectory) */}
                {currentPath !== '' && (
                  <div className="tree-row tree-row-parent" onClick={handleNavigateUp}>
                    <span className="col-check" />
                    <span className="col-name">
                      <CornerLeftUp size={14} className="text-muted flex-shrink-0" />
                      <span>.. (Up to parent folder)</span>
                    </span>
                    <span className="col-size">—</span>
                    <span className="col-mtime">—</span>
                  </div>
                )}

                {displayedFolderItems.length === 0 ? (
                  <div className="empty-state py-8">
                    {currentPath === ''
                      ? 'No items found in snapshot.'
                      : 'This directory is empty or contains no subfolders.'}
                  </div>
                ) : (
                  displayedFolderItems.map((node) => {
                    const isDir = node.type === 'dir';
                    const isChecked = selectedPaths.has(node.path);

                    return (
                      <div
                        key={node.path}
                        className={`tree-row ${isDir ? 'tree-row-dir' : ''} ${isChecked ? 'row-selected' : ''}`}
                        onClick={() => {
                          if (isDir) {
                            handleNavigateToFolder(node.path);
                          } else {
                            toggleSelectPath(node.path);
                          }
                        }}
                      >
                        <span className="col-check" onClick={(e) => e.stopPropagation()}>
                          <button
                            type="button"
                            className="checkbox-icon-btn"
                            onClick={() => toggleSelectPath(node.path)}
                            title={isDir ? 'Select entire folder for restore' : 'Select file'}
                          >
                            {isChecked ? (
                              <CheckSquare size={14} className="text-primary" />
                            ) : (
                              <Square size={14} className="text-muted" />
                            )}
                          </button>
                        </span>

                        <span className="col-name truncate" title={node.path}>
                          {isDir ? (
                            <Folder size={14} className="folder-icon flex-shrink-0" />
                          ) : (
                            <File size={14} className="file-icon flex-shrink-0" />
                          )}
                          <span className="truncate">{node.name}</span>
                          {isDir && <span className="folder-badge-tag">folder</span>}
                        </span>

                        <span className="col-size">
                          {node.size !== undefined ? formatBytes(node.size) : '—'}
                        </span>

                        <span className="col-mtime">
                          {node.mtime ? formatRelativeTime(node.mtime) : '—'}
                        </span>
                      </div>
                    );
                  })
                )}
              </div>
            )}
          </div>

          {/* Footer Pagination Bar */}
          {!loadingNodes && (
            <div className="browser-pagination-bar">
              {isSearchActive ? (
                <>
                  <span>
                    Showing {searchResults.length === 0 ? 0 : (searchPage - 1) * SEARCH_PAGE_SIZE + 1}–
                    {Math.min(searchPage * SEARCH_PAGE_SIZE, searchResults.length)} of {searchResults.length} matches
                  </span>
                  {totalSearchPages > 1 && (
                    <div className="browser-pagination-controls">
                      <button
                        type="button"
                        className="btn-page-nav"
                        disabled={searchPage <= 1}
                        onClick={() => setSearchPage((p) => Math.max(1, p - 1))}
                      >
                        <ChevronLeft size={13} />
                        <span>Prev</span>
                      </button>
                      <span>
                        Page {searchPage} of {totalSearchPages}
                      </span>
                      <button
                        type="button"
                        className="btn-page-nav"
                        disabled={searchPage >= totalSearchPages}
                        onClick={() => setSearchPage((p) => Math.min(totalSearchPages, p + 1))}
                      >
                        <span>Next</span>
                        <ChevronRight size={13} />
                      </button>
                    </div>
                  )}
                </>
              ) : (
                <>
                  <span>
                    Showing {directChildren.length === 0 ? 0 : (folderPage - 1) * FOLDER_PAGE_SIZE + 1}–
                    {Math.min(folderPage * FOLDER_PAGE_SIZE, directChildren.length)} of {directChildren.length} items
                  </span>
                  {totalFolderPages > 1 && (
                    <div className="browser-pagination-controls">
                      <button
                        type="button"
                        className="btn-page-nav"
                        disabled={folderPage <= 1}
                        onClick={() => setFolderPage((p) => Math.max(1, p - 1))}
                      >
                        <ChevronLeft size={13} />
                        <span>Prev</span>
                      </button>
                      <span>
                        Page {folderPage} of {totalFolderPages}
                      </span>
                      <button
                        type="button"
                        className="btn-page-nav"
                        disabled={folderPage >= totalFolderPages}
                        onClick={() => setFolderPage((p) => Math.min(totalFolderPages, p + 1))}
                      >
                        <span>Next</span>
                        <ChevronRight size={13} />
                      </button>
                    </div>
                  )}
                </>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Prune Retention Policy Modal */}
      {showPruneModal && (
        <div className="modal-backdrop">
          <div className="modal-box modal-md">
            <div className="modal-header">
              <div className="modal-header-title">
                <Archive size={18} className="text-primary" />
                <h3>Prune Stored Backups</h3>
              </div>
              <button onClick={() => setShowPruneModal(false)} className="close-btn">
                <X size={18} />
              </button>
            </div>

            <div className="modal-body">
              <p className="text-muted">
                Run restic retention cleanup to delete older backups and free up repository space.
              </p>

              <div className="form-group mt-3">
                <label>Number of Backups to Keep:</label>
                <select
                  value={pruneKeepLast}
                  onChange={(e) => setPruneKeepLast(parseInt(e.target.value) || 10)}
                  disabled={pruning}
                  className="retention-select"
                >
                  <option value={3}>Keep last 3 backups</option>
                  <option value={5}>Keep last 5 backups</option>
                  <option value={10}>Keep last 10 backups (Recommended)</option>
                  <option value={20}>Keep last 20 backups</option>
                  <option value={30}>Keep last 30 backups</option>
                </select>
              </div>

              <div className="pinned-protection-notice">
                <Pin size={16} className="text-warning flex-shrink-0" />
                <div>
                  <strong>Protected Snapshots Are Safe:</strong>
                  <p>
                    Any snapshot tagged as <strong>Pinned</strong> or <strong>Important</strong> will never be deleted, even if it is older than the keep limit.
                  </p>
                </div>
              </div>

              {pruneResult && (
                <div className="alert-banner alert-success mt-3">
                  <CheckCircle size={16} />
                  <span>{pruneResult}</span>
                </div>
              )}
            </div>

            <div className="modal-footer">
              <button
                onClick={() => setShowPruneModal(false)}
                className="btn-secondary"
                disabled={pruning}
              >
                {pruneResult ? 'Close' : 'Cancel'}
              </button>
              <button
                onClick={handleExecutePrune}
                disabled={pruning}
                className="btn-danger"
              >
                {pruning ? 'Pruning Repository...' : `Delete Older Backups (Keep Last ${pruneKeepLast})`}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Restore Confirmation Modal */}
      {showRestoreModal && selectedSnapshot && (
        <div className="modal-backdrop">
          <div className="modal-box modal-md">
            <div className="modal-header">
              <div className="modal-header-title">
                <RotateCcw size={18} className="text-primary" />
                <h3>Restore Minecraft Backup</h3>
              </div>
              <button onClick={() => setShowRestoreModal(false)} className="close-btn">
                <X size={18} />
              </button>
            </div>

            <div className="modal-body">
              {(() => {
                const launcherInfo = detectLauncherFromSnapshot(selectedSnapshot);
                const snapShort = selectedSnapshot.short_id || selectedSnapshot.id.substring(0, 8);
                const defaultSafeFolder = `${defaultRestoreBaseDir.replace(/[/\\]$/, '')}/${snapShort}`;

                return (
                  <>
                    <div className="restore-summary-box">
                      <div>
                        Snapshot: <strong>{snapShort}</strong>
                      </div>
                      <div>
                        Recorded: <strong>{formatExactTimestamp(selectedSnapshot.time)}</strong>
                      </div>
                      <div className="flex items-center gap-2">
                        <span>Original Launcher:</span>
                        <span className="badge-launcher-tag">
                          🎮 {launcherInfo.name}
                        </span>
                      </div>
                      <div className="flex items-center gap-2">
                        <span>Backup Type:</span>
                        {launcherInfo.scopeType === 'instance' ? (
                          <span className="badge-scope-instance">
                            <Box size={11} />
                            <span>Instance: {launcherInfo.instanceName || 'Unknown'}</span>
                          </span>
                        ) : (
                          <span className="badge-scope-launcher">
                            <Layers size={11} />
                            <span>Full Launcher Backup</span>
                          </span>
                        )}
                      </div>
                      <div>
                        Restore Scope:{' '}
                        <strong>
                          {selectedPaths.size > 0
                            ? `${selectedPaths.size} selected file(s)/folder(s)`
                            : launcherInfo.scopeType === 'instance'
                            ? `Entire Instance (${launcherInfo.instanceName || 'Instance'})`
                            : `Entire Launcher (${launcherInfo.name})`}
                        </strong>
                      </div>
                    </div>

                    {/* Restore Destination Option Tabs */}
                    <div className="form-group mt-3">
                      <label className="text-xs font-semibold block mb-2">Choose Restore Destination:</label>
                      <div className="restore-destination-tabs">
                        <button
                          type="button"
                          className={`restore-dest-tab ${restoreMode === 'launcher' ? 'active' : ''}`}
                          onClick={() => {
                            setRestoreMode('launcher');
                            if (launcherInfo.originalPath) {
                              setTargetDir(launcherInfo.originalPath);
                            }
                            setOverwriteConfirmed(true);
                          }}
                        >
                          <div className="flex items-center gap-1.5 font-semibold text-xs">
                            <Gamepad2 size={16} className="text-primary flex-shrink-0" />
                            <span>
                              {launcherInfo.scopeType === 'instance'
                                ? 'Restore to Launcher Instance'
                                : 'Restore to Launcher'}
                            </span>
                          </div>
                          <span className="dest-card-sub text-[11px] text-muted">
                            {launcherInfo.scopeType === 'instance'
                              ? `Restores instance "${launcherInfo.instanceName || ''}" directly into ${launcherInfo.name}`
                              : `Restores entire launcher configuration into ${launcherInfo.name}`}
                          </span>
                        </button>

                        <button
                          type="button"
                          className={`restore-dest-tab ${restoreMode === 'backup_folder' ? 'active' : ''}`}
                          onClick={() => {
                            setRestoreMode('backup_folder');
                            setTargetDir(defaultSafeFolder);
                            setOverwriteConfirmed(false);
                          }}
                        >
                          <div className="flex items-center gap-1.5 font-semibold text-xs">
                            <FolderArchive size={16} className="text-emerald-400 flex-shrink-0" />
                            <span>Safe Backup Folder</span>
                          </div>
                          <span className="dest-card-sub text-[11px] text-muted">
                            Extracts into /Restored_Backups/{snapShort}
                          </span>
                        </button>

                        <button
                          type="button"
                          className={`restore-dest-tab ${restoreMode === 'custom' ? 'active' : ''}`}
                          onClick={() => setRestoreMode('custom')}
                        >
                          <div className="flex items-center gap-1.5 font-semibold text-xs">
                            <FolderOpen size={16} className="text-warning flex-shrink-0" />
                            <span>Custom Directory</span>
                          </div>
                          <span className="dest-card-sub text-[11px] text-muted">
                            Select any folder on disk
                          </span>
                        </button>
                      </div>
                    </div>

                    {/* Destination Folder Path */}
                    <div className="form-group mt-3">
                      <label className="text-xs font-semibold">Destination Folder Path:</label>
                      <div className="flex gap-2 mt-1">
                        <input
                          type="text"
                          value={targetDir}
                          onChange={(e) => setTargetDir(e.target.value)}
                          placeholder={defaultRestoreBaseDir}
                          className="flex-1 font-mono text-xs"
                        />
                        <button
                          type="button"
                          onClick={handlePickCustomFolder}
                          className="btn-secondary text-xs px-3"
                          title="Browse system directory"
                        >
                          Browse...
                        </button>
                      </div>
                    </div>

                    {restoreMode === 'launcher' ? (
                      <div className="alert-banner alert-warning mt-3">
                        <AlertTriangle size={16} className="flex-shrink-0" />
                        <div className="text-xs">
                          <strong>Active Game Overwrite Notice:</strong>
                          <p>
                            Files will be restored directly back to <code>{launcherInfo.originalPath || targetDir}</code>.
                            Please ensure your Minecraft launcher and game client are closed before starting to prevent file lock errors.
                          </p>
                        </div>
                      </div>
                    ) : (
                      <div className="alert-banner alert-info mt-3">
                        <Info size={16} className="flex-shrink-0" />
                        <div className="text-xs">
                          <span>
                            Files will be extracted into an isolated backup folder. Your active Minecraft installation will NOT be altered.
                          </span>
                        </div>
                      </div>
                    )}

                    <label className="checkbox-row mt-3">
                      <input
                        type="checkbox"
                        checked={overwriteConfirmed}
                        onChange={(e) => setOverwriteConfirmed(e.target.checked)}
                      />
                      <span>Confirm overwriting existing files in destination folder</span>
                    </label>
                  </>
                );
              })()}

              {restoreResult && (
                <div className="alert-banner alert-success mt-3">
                  <CheckCircle size={16} />
                  <span>{restoreResult}</span>
                </div>
              )}

              {restoreError && (
                <div className="alert-banner alert-error mt-3">
                  <AlertTriangle size={16} />
                  <span>{restoreError}</span>
                </div>
              )}
            </div>

            <div className="modal-footer">
              <button
                onClick={() => setShowRestoreModal(false)}
                className="btn-secondary"
                disabled={restoring}
              >
                {restoreResult ? 'Close' : 'Cancel'}
              </button>
              <button
                onClick={handleStartRestore}
                disabled={restoring || !targetDir.trim()}
                className="btn-primary"
              >
                {restoring ? 'Restoring Files...' : 'Start Restore'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Delete Single Snapshot Confirmation Modal */}
      {showDeleteModal && deleteTargetSnapshot && (
        <div className="modal-backdrop">
          <div className="modal-box modal-md">
            <div className="modal-header">
              <div className="modal-header-title text-danger">
                <Trash2 size={18} />
                <h3>Delete Backup Snapshot</h3>
              </div>
              <button
                onClick={() => {
                  if (!deleting) setShowDeleteModal(false);
                }}
                className="close-btn"
                disabled={deleting}
              >
                <X size={18} />
              </button>
            </div>

            <div className="modal-body">
              {Boolean(
                deleteTargetSnapshot.tags?.includes('pinned') ||
                deleteTargetSnapshot.tags?.includes('important')
              ) && (
                <div className="delete-warning-banner">
                  <AlertOctagon size={20} className="flex-shrink-0" />
                  <div>
                    <strong>Important / Pinned Backup Notice</strong>
                    <p>
                      This backup is marked as <strong>Pinned</strong> (protected from auto-pruning).
                      Deleting it manually will permanently remove it from your storage repository.
                    </p>
                  </div>
                </div>
              )}

              <p className="text-muted">
                Are you sure you want to permanently delete this snapshot? This action cannot be undone.
              </p>

              <div className="restore-summary-box mt-3">
                <div>
                  Snapshot ID: <strong>{deleteTargetSnapshot.short_id || deleteTargetSnapshot.id.substring(0, 8)}</strong>
                </div>
                <div>
                  Recorded: <strong>{formatExactTimestamp(deleteTargetSnapshot.time)}</strong> ({formatRelativeTime(deleteTargetSnapshot.time)})
                </div>
                <div>
                  Host: <strong>{deleteTargetSnapshot.hostname}</strong>
                </div>
                <div>
                  Paths: <strong>{deleteTargetSnapshot.paths?.join(', ') || ''}</strong>
                </div>
                {deleteTargetSnapshot.tags && deleteTargetSnapshot.tags.length > 0 && (
                  <div className="mt-1">
                    Tags:{' '}
                    {deleteTargetSnapshot.tags.map((t) => (
                      <span key={t} className="tag-pill mr-1">#{t}</span>
                    ))}
                  </div>
                )}
              </div>

              <div className="form-group mt-3">
                <label className="checkbox-row">
                  <input
                    type="checkbox"
                    checked={deletePrune}
                    onChange={(e) => setDeletePrune(e.target.checked)}
                    disabled={deleting}
                  />
                  <span>
                    <strong>Prune repository immediately (--prune)</strong>
                  </span>
                </label>
                <span className="form-hint">
                  Immediately purges unreferenced data blobs from your storage repository to reclaim disk/cloud space.
                </span>
              </div>

              {deleteError && (
                <div className="alert-banner alert-error mt-3">
                  <AlertTriangle size={16} />
                  <span>{deleteError}</span>
                </div>
              )}
            </div>

            <div className="modal-footer">
              <button
                onClick={() => setShowDeleteModal(false)}
                className="btn-secondary"
                disabled={deleting}
              >
                Cancel
              </button>
              <button
                onClick={handleExecuteDelete}
                disabled={deleting}
                className="btn-danger"
              >
                <Trash2 size={14} />
                <span>
                  {deleting
                    ? (deletePrune ? 'Deleting & Pruning...' : 'Deleting Snapshot...')
                    : 'Permanently Delete'}
                </span>
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
