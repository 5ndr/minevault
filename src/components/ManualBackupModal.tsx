import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Play,
  AlertTriangle,
  CheckCircle,
  XCircle,
  X,
  Settings2,
  Clock,
  Pin,
  Tag,
  Archive,
  Minus,
  Edit3,
  Box,
  Layers,
  Power,
  RefreshCw,
} from 'lucide-react';
import { Profile, BackupProgressEvent, RunResult, SystemInfo, ProcessCheckResult } from '../types';
import { formatBytes, formatDuration, formatDateShort } from '../utils/format';
import {
  MinecraftLauncherSelector,
  LauncherSelectionState,
} from './MinecraftLauncherSelector';

interface ManualBackupModalProps {
  isOpen: boolean;
  onClose: () => void;
  onMinimize?: () => void;
  profiles: Profile[];
  selectedProfile: Profile | null;
  onSelectProfile: (p: Profile) => void;
  systemInfo: SystemInfo | null;
  onStartBackup: (profile: Profile, force?: boolean) => Promise<RunResult | null>;
  onCancelBackup: () => void;
  isRunning: boolean;
  progress: BackupProgressEvent | null;
  backupResult: RunResult | null;
  backupError: string | null;
}

export const ManualBackupModal: React.FC<ManualBackupModalProps> = ({
  isOpen,
  onClose,
  onMinimize,
  profiles,
  selectedProfile,
  onSelectProfile,
  systemInfo,
  onStartBackup,
  onCancelBackup,
  isRunning,
  progress,
  backupResult,
  backupError,
}) => {
  if (!isOpen) return null;

  const [activeProfile] = useState<Profile>(
    selectedProfile ||
      profiles[0] || {
        id: 'manual-profile',
        name: 'Manual Minecraft Backup',
        repo_id: 'default-repo',
        machine_id: systemInfo?.machine_id || 'local-host',
        machine_name: systemInfo?.machine_name || 'Host',
        source_paths: systemInfo?.prism_launcher_path ? [systemInfo.prism_launcher_path] : ['/Users/declan'],
        exclusions: ['*.lock', 'logs/', 'crash-reports/', 'cache/'],
        tags: ['minecraft', 'manual'],
        keep_last: 10,
        tag_timestamp: true,
      }
  );

  // Check if manual backup was previously configured
  const hasSavedConfig = Boolean(
    localStorage.getItem('minevault_manual_configured') === 'true' ||
      (selectedProfile?.source_paths && selectedProfile.source_paths.length > 0)
  );

  const [isEditingConfig, setIsEditingConfig] = useState<boolean>(!hasSavedConfig);
  const [resolvedSources, setResolvedSources] = useState<string[]>(
    activeProfile.source_paths.length > 0
      ? activeProfile.source_paths
      : systemInfo?.prism_launcher_path
      ? [systemInfo.prism_launcher_path]
      : []
  );
  const [exclusionsInput, setExclusionsInput] = useState<string>(
    activeProfile.exclusions.join(', ') || '*.lock, logs/, crash-reports/, cache/'
  );
  const [distinctionLabel, setDistinctionLabel] = useState<string>(
    activeProfile.distinction_label || ''
  );
  const [launcherId, setLauncherId] = useState<string>('prism');
  const [launcherName, setLauncherName] = useState<string>('Prism Launcher');
  const [launcherScope, setLauncherScope] = useState<'entire' | 'instance'>('entire');
  const [selectedInstance, setSelectedInstance] = useState<string>('');
  const [customTagsInput, setCustomTagsInput] = useState<string>(
    activeProfile.tags
      ? activeProfile.tags
          .filter((t) => !['minecraft', 'manual', 'automatic', 'pinned', 'important', 'scope:launcher', 'scope:instance'].includes(t) && !t.startsWith('launcher:') && !t.startsWith('instance:'))
          .join(', ')
      : ''
  );
  const [keepLast, setKeepLast] = useState<number>(activeProfile.keep_last ?? 10);
  const [tagTimestamp, setTagTimestamp] = useState<boolean>(activeProfile.tag_timestamp ?? true);
  const [isPinned, setIsPinned] = useState<boolean>(activeProfile.is_pinned ?? false);
  const [showAdvancedPaths, setShowAdvancedPaths] = useState<boolean>(false);

  // Minecraft Process Guard state
  const [procCheck, setProcCheck] = useState<ProcessCheckResult | null>(null);
  const [checkingProcs, setCheckingProcs] = useState<boolean>(false);
  const [closingProcs, setClosingProcs] = useState<boolean>(false);
  const [closeSuccessMsg, setCloseSuccessMsg] = useState<string | null>(null);
  const [forceBypass, setForceBypass] = useState<boolean>(false);

  const runProcessCheck = async () => {
    setCheckingProcs(true);
    try {
      const res: ProcessCheckResult = await invoke('check_minecraft_processes_cmd');
      setProcCheck(res);
    } catch (e) {
      console.error('Failed to check Minecraft processes:', e);
    } finally {
      setCheckingProcs(false);
    }
  };

  useEffect(() => {
    if (isOpen) {
      runProcessCheck();
    }
  }, [isOpen]);

  const handleCloseRunningProcesses = async () => {
    if (!procCheck || procCheck.processes.length === 0) return;
    setClosingProcs(true);
    setCloseSuccessMsg(null);
    try {
      const pids = procCheck.processes.map((p) => p.pid);
      const closedCount: number = await invoke('close_minecraft_processes_cmd', { pids, force: false });
      await new Promise((r) => setTimeout(r, 1200));
      const recheck: ProcessCheckResult = await invoke('check_minecraft_processes_cmd');
      setProcCheck(recheck);
      if (!recheck.is_running) {
        setCloseSuccessMsg(`Closed ${closedCount} process(es). Safe to backup!`);
      } else {
        setCloseSuccessMsg(`Sent close signal. ${recheck.processes.length} process(es) still open.`);
      }
    } catch (err: any) {
      console.error('Error closing processes:', err);
    } finally {
      setClosingProcs(false);
    }
  };

  // Calculate estimated time remaining
  const renderRemainingTime = () => {
    if (!progress) return 'Estimating remaining time...';
    if (progress.seconds_remaining !== undefined && progress.seconds_remaining !== null) {
      if (progress.seconds_remaining === 0) return 'Almost finished...';
      const mins = Math.floor(progress.seconds_remaining / 60);
      const secs = progress.seconds_remaining % 60;
      if (mins > 0) return `~${mins}m ${secs}s remaining`;
      return `~${secs}s remaining`;
    }
    if (progress.percent_done > 5 && progress.seconds_elapsed > 2) {
      const totalEstimated = progress.seconds_elapsed / (progress.percent_done / 100.0);
      const rem = Math.max(0, Math.round(totalEstimated - progress.seconds_elapsed));
      const mins = Math.floor(rem / 60);
      const secs = rem % 60;
      if (mins > 0) return `~${mins}m ${secs}s remaining`;
      return `~${secs}s remaining`;
    }
    return 'Calculating time remaining...';
  };

  const handleLauncherSelectionChange = (selection: LauncherSelectionState) => {
    if (selection.resolvedSourcePaths.length > 0) {
      setResolvedSources(selection.resolvedSourcePaths);
    }
    if (selection.recommendedExclusions.length > 0) {
      setExclusionsInput(selection.recommendedExclusions.join(', '));
    }
    if (selection.launcherId) {
      setLauncherId(selection.launcherId);
      setLauncherName(selection.launcherName);
    }
    setLauncherScope(selection.scope);
    setSelectedInstance(selection.selectedInstance);
  };

  const handleLaunch = async () => {
    const exclusions = exclusionsInput
      .split(',')
      .map((s: string) => s.trim())
      .filter((s: string) => s.length > 0);

    const customTags = customTagsInput
      .split(',')
      .map((t: string) => t.trim().toLowerCase())
      .filter((t: string) => t.length > 0 && !t.includes(' '));

    const launcherTag = `launcher:${launcherId}`;
    const scopeTag = launcherScope === 'instance' ? 'scope:instance' : 'scope:launcher';
    const instanceTag = launcherScope === 'instance' && selectedInstance ? `instance:${selectedInstance}` : null;
    const allTags = Array.from(
      new Set([
        'minecraft',
        'manual',
        launcherTag,
        launcherId,
        scopeTag,
        ...(instanceTag ? [instanceTag] : []),
        ...customTags,
      ])
    );

    const updatedProfile: Profile = {
      ...activeProfile,
      source_paths: resolvedSources,
      exclusions,
      tags: allTags,
      distinction_label: distinctionLabel.trim() || undefined,
      keep_last: keepLast > 0 ? keepLast : undefined,
      tag_timestamp: tagTimestamp,
      is_pinned: isPinned,
    };

    // Save manual configuration state
    localStorage.setItem('minevault_manual_configured', 'true');
    setIsEditingConfig(false);

    // Check if Minecraft or Launcher is active
    if (procCheck?.is_running && !forceBypass) {
      alert(
        `Cannot start backup: Minecraft is currently running (${procCheck.processes.map((p) => p.name).join(', ')}). Please close Minecraft to prevent world corruption, or check "Ignore check and backup anyway".`
      );
      return;
    }

    onSelectProfile(updatedProfile);
    await onStartBackup(updatedProfile, forceBypass);
  };

  const handleHeaderCloseOrMinimize = () => {
    if (isRunning && onMinimize) {
      onMinimize();
    } else {
      onClose();
    }
  };

  return (
    <div className="modal-backdrop">
      <div className="modal-box modal-lg">
        <div className="modal-header">
          <div className="modal-header-title">
            <Play size={18} className="text-primary" />
            <h3>Manual Minecraft Backup</h3>
          </div>
          <div className="modal-header-actions">
            {isRunning && onMinimize && (
              <button
                onClick={onMinimize}
                className="minimize-btn"
                title="Minimize to floating window (backup continues running)"
              >
                <Minus size={16} />
              </button>
            )}
            <button onClick={handleHeaderCloseOrMinimize} className="close-btn" title="Close window">
              <X size={18} />
            </button>
          </div>
        </div>

        <div className="modal-body">
          {/* Active Process Guard Warning */}
          {procCheck?.is_running && !isRunning && (
            <div className="process-guard-card">
              <div className="process-guard-header">
                <AlertTriangle size={20} className="text-warning flex-shrink-0 mt-0.5" />
                <div className="flex-1">
                  <h4 className="text-warning font-semibold text-sm">
                    Minecraft or Launcher is Currently Running!
                  </h4>
                  <p className="text-xs text-muted mt-0.5 leading-relaxed">
                    Backing up while a Minecraft world is open can corrupt chunk files and player inventories. Please save, exit your world, and close the launcher before backing up.
                  </p>
                </div>
              </div>

              <div className="process-list-chips mt-2.5">
                <span className="text-xs text-muted block mb-1 font-medium">Detected Running Processes:</span>
                <div className="flex items-center gap-1.5 flex-wrap">
                  {procCheck.processes.map((p) => (
                    <span key={p.pid} className="process-chip">
                      {p.kind === 'game_instance' ? (
                        <Box size={12} className="text-purple-400" />
                      ) : (
                        <Layers size={12} className="text-cyan-400" />
                      )}
                      <strong>{p.name}</strong>
                      <span className="text-muted text-[10px]">PID: {p.pid}</span>
                    </span>
                  ))}
                </div>
              </div>

              <div className="process-guard-actions mt-3 pt-2.5 border-t border-warning/20 flex items-center justify-between gap-3 flex-wrap">
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={handleCloseRunningProcesses}
                    disabled={closingProcs || checkingProcs}
                    className="btn-danger-outline text-xs py-1.5 px-3 flex items-center gap-1.5 font-medium"
                  >
                    <Power size={13} />
                    <span>{closingProcs ? 'Closing...' : 'Close Minecraft & Launchers'}</span>
                  </button>

                  <button
                    type="button"
                    onClick={runProcessCheck}
                    disabled={checkingProcs || closingProcs}
                    className="btn-secondary text-xs py-1.5 px-3 flex items-center gap-1.5"
                  >
                    <RefreshCw size={13} className={checkingProcs ? 'animate-spin' : ''} />
                    <span>Re-check</span>
                  </button>
                </div>

                <label className="checkbox-row text-xs text-muted cursor-pointer">
                  <input
                    type="checkbox"
                    checked={forceBypass}
                    onChange={(e) => setForceBypass(e.target.checked)}
                  />
                  <span>Ignore check and backup anyway (Risk of corruption)</span>
                </label>
              </div>

              {closeSuccessMsg && (
                <div className="text-xs text-emerald-400 mt-2 font-medium">
                  ✓ {closeSuccessMsg}
                </div>
              )}
            </div>
          )}

          {/* If already configured and not editing, show 1-Click Fast Launch Card */}
          {!isEditingConfig && (
            <div className="one-time-setup-card">
              <div className="setup-card-top">
                <div className="setup-badge">
                  <Archive size={16} className="text-primary" />
                  <strong>Saved Minecraft Backup Profile</strong>
                </div>
                <button
                  type="button"
                  onClick={() => setIsEditingConfig(true)}
                  disabled={isRunning}
                  className="btn-edit-setup"
                  title="Modify Minecraft launcher, paths, or instance scope"
                >
                  <Edit3 size={13} />
                  <span>Change Setup</span>
                </button>
              </div>

              <div className="setup-details-grid">
                <div>
                  <span className="text-muted text-xs block">Source Launcher:</span>
                  <span className="badge-launcher-tag mt-0.5">
                    🎮 {launcherName}
                  </span>
                </div>
                <div>
                  <span className="text-muted text-xs block">Source Folder:</span>
                  <code className="setup-path truncate" title={resolvedSources.join(', ')}>
                    {resolvedSources.join(', ') || 'No source configured'}
                  </code>
                </div>
                <div>
                  <span className="text-muted text-xs block">Retention:</span>
                  <span className="setup-retention-pill">
                    {keepLast > 0 ? `Keeping last ${keepLast} backups` : 'Unlimited backups (Keep all)'}
                  </span>
                </div>
              </div>
            </div>
          )}

          {/* Minecraft Launcher & Scope Selection (shown when editing setup or first time) */}
          {isEditingConfig && (
            <div className="setup-editor-section">
              <MinecraftLauncherSelector
                initialBasePath={resolvedSources[0]}
                onSelectionChange={handleLauncherSelectionChange}
                disabled={isRunning}
              />

              <div className="editor-done-row">
                <button
                  type="button"
                  onClick={() => setIsEditingConfig(false)}
                  disabled={resolvedSources.length === 0 || isRunning}
                  className="btn-save-setup"
                >
                  <CheckCircle size={14} />
                  <span>Save Profile Configuration</span>
                </button>
              </div>
            </div>
          )}

          {/* Distinction Label, Retention, and Protection Options */}
          <div className="backup-options-container">
            {/* 1. Distinction / Custom Renaming */}
            <div className="form-group">
              <label className="flex items-center gap-1">
                <Tag size={14} className="text-primary" />
                <span>Backup Distinction / Custom Label (Optional):</span>
              </label>
              <input
                type="text"
                value={distinctionLabel}
                onChange={(e) => setDistinctionLabel(e.target.value)}
                disabled={isRunning}
                placeholder="e.g. Before Ender Dragon, Pre-1.21 Update, Nether Portal"
                className="distinction-input"
              />
              <span className="form-hint">
                Adds a distinct custom label and tag to this snapshot so you can identify it easily in the backup browser.
              </span>
            </div>

            {/* 2. Custom Tags & Launcher Tag */}
            <div className="form-group">
              <label className="flex items-center gap-1">
                <Tag size={14} className="text-primary" />
                <span>Custom Tags (comma-separated):</span>
              </label>
              <input
                type="text"
                value={customTagsInput}
                onChange={(e) => setCustomTagsInput(e.target.value)}
                disabled={isRunning}
                placeholder="e.g. survival, hardcore, season2, testing"
                className="distinction-input"
              />
              <div className="flex items-center gap-1.5 mt-1 text-xs text-muted">
                <span>Active tags:</span>
                <span className="tag-pill text-[11px]">#minecraft</span>
                <span className="tag-pill text-[11px]">#manual</span>
                <span className="tag-pill text-[11px] text-primary">#launcher:{launcherId}</span>
                {customTagsInput
                  .split(',')
                  .map((t) => t.trim().toLowerCase())
                  .filter((t) => t.length > 0)
                  .map((t) => (
                    <span key={t} className="tag-pill text-[11px] text-emerald-400">
                      #{t}
                    </span>
                  ))}
              </div>
            </div>

            {/* 3. Retention Settings */}
            <div className="form-row-grid">
              <div className="form-group">
                <label className="flex items-center gap-1">
                  <Archive size={14} className="text-primary" />
                  <span>Backups to Retain (Auto-delete older):</span>
                </label>
                <select
                  value={keepLast}
                  onChange={(e) => setKeepLast(parseInt(e.target.value) || 0)}
                  disabled={isRunning}
                  className="retention-select"
                >
                  <option value={5}>Keep last 5 backups</option>
                  <option value={10}>Keep last 10 backups (Recommended)</option>
                  <option value={20}>Keep last 20 backups</option>
                  <option value={50}>Keep last 50 backups</option>
                  <option value={0}>Unlimited (Keep all backups)</option>
                </select>
              </div>

              {/* 4. Pinned / Important Protection Checkbox */}
              <div className="form-group">
                <label className="flex items-center gap-1">
                  <Pin size={14} className="text-warning" />
                  <span>Protection Status:</span>
                </label>
                <label className="checkbox-row-styled">
                  <input
                    type="checkbox"
                    checked={isPinned}
                    onChange={(e) => setIsPinned(e.target.checked)}
                    disabled={isRunning}
                  />
                  <div>
                    <strong>Protect as Important / Pinned</strong>
                    <span className="block text-xs text-muted">
                      Immune from retention cleanup. Will never be auto-deleted.
                    </span>
                  </div>
                </label>
              </div>
            </div>

            {/* 5. Timestamp Tagging in MM-DD-YY Format */}
            <label className="checkbox-row">
              <input
                type="checkbox"
                checked={tagTimestamp}
                onChange={(e) => setTagTimestamp(e.target.checked)}
                disabled={isRunning}
              />
              <span>
                Append execution timestamp tag in <code>MM-DD-YY</code> format (e.g. <code>#{formatDateShort(new Date().toISOString()).replace(' ', '_').replace(/:/g, '-')}</code>)
              </span>
            </label>
          </div>

          {/* Advanced Path & Exclusions Details Toggle */}
          <div className="advanced-paths-accordion">
            <button
              type="button"
              className="accordion-toggle-btn"
              onClick={() => setShowAdvancedPaths(!showAdvancedPaths)}
            >
              <Settings2 size={15} />
              <span>{showAdvancedPaths ? 'Hide Exact Path & Exclusion Details' : 'View / Edit Resolved Source Paths'}</span>
            </button>

            {showAdvancedPaths && (
              <div className="accordion-content">
                <div className="form-group mt-2">
                  <label>Resolved Source Path(s) to Back Up:</label>
                  <textarea
                    rows={2}
                    value={resolvedSources.join('\n')}
                    onChange={(e) =>
                      setResolvedSources(
                        e.target.value
                          .split('\n')
                          .map((s) => s.trim())
                          .filter((s) => s.length > 0)
                      )
                    }
                    disabled={isRunning}
                  />
                </div>

                <div className="form-group">
                  <label>Exclusion Patterns (comma-separated):</label>
                  <input
                    type="text"
                    value={exclusionsInput}
                    onChange={(e) => setExclusionsInput(e.target.value)}
                    disabled={isRunning}
                  />
                </div>
              </div>
            )}
          </div>

          {/* Live Streaming Progress Area */}
          {isRunning && (
            <div className="backup-progress-card">
              <div className="progress-header">
                <div className="flex items-center gap-2">
                  <span className="pulse-indicator-dot" />
                  <strong>Backing Up Minecraft Files...</strong>
                </div>
                <span className="progress-percent">
                  {progress ? `${progress.percent_done.toFixed(1)}%` : 'Initializing...'}
                </span>
              </div>

              <div className="progress-bar-track">
                <div
                  className="progress-bar-fill"
                  style={{ width: `${progress ? Math.min(100, Math.max(3, progress.percent_done)) : 4}%` }}
                />
              </div>

              {progress && (
                <div className="progress-stats-grid">
                  <div className="stat-box highlight-time">
                    <Clock size={14} className="text-primary flex-shrink-0" />
                    <div>
                      <span className="text-xs text-muted block">Estimated Time Remaining:</span>
                      <strong>{renderRemainingTime()}</strong>
                    </div>
                  </div>
                  <div>
                    <span>Elapsed:</span> {progress.seconds_elapsed}s
                  </div>
                  <div>
                    <span>Files:</span> {progress.files_done} / {progress.total_files}
                  </div>
                  <div>
                    <span>Processed:</span> {formatBytes(progress.bytes_done)} / {formatBytes(progress.total_bytes)}
                  </div>
                </div>
              )}

              {progress?.current_file && (
                <div className="current-file-label truncate">Current: {progress.current_file}</div>
              )}

              <div className="progress-minimize-tip">
                <Minus size={13} />
                <span>You can minimize this window at any time. Progress will continue in the bottom dock.</span>
              </div>
            </div>
          )}

          {/* Backup Result Summary */}
          {backupResult && (
            <div
              className={`backup-result-card ${
                backupResult.status === 'success'
                  ? 'res-success'
                  : backupResult.status === 'partial_success'
                  ? 'res-warning'
                  : 'res-failed'
              }`}
            >
              <div className="result-header">
                {backupResult.status === 'success' ? (
                  <CheckCircle size={18} className="text-success" />
                ) : backupResult.status === 'partial_success' ? (
                  <AlertTriangle size={18} className="text-warning" />
                ) : (
                  <XCircle size={18} className="text-danger" />
                )}
                <strong>
                  {backupResult.status === 'success'
                    ? 'Backup Completed Successfully'
                    : backupResult.status === 'partial_success'
                    ? 'Completed with Warnings (Exit Code 3)'
                    : 'Backup Failed'}
                </strong>
              </div>

              <div className="result-stats">
                <div>
                  Duration: <strong>{formatDuration(backupResult.duration_seconds)}</strong>
                </div>
                <div>
                  Files Processed: <strong>{backupResult.total_files_processed}</strong>
                </div>
                <div>
                  New Data Added: <strong>{formatBytes(backupResult.data_added_bytes)}</strong>
                </div>
                {backupResult.snapshot_id && (
                  <div>
                    Snapshot ID: <code>{backupResult.snapshot_id.substring(0, 8)}</code>
                  </div>
                )}
              </div>

              {backupResult.error_message && (
                <p className="result-warning-text">{backupResult.error_message}</p>
              )}
            </div>
          )}

          {backupError && (
            <div className="alert-banner alert-error">
              <AlertTriangle size={16} />
              <span>{backupError}</span>
            </div>
          )}
        </div>

        <div className="modal-footer">
          {isRunning ? (
            <div className="running-footer-row">
              {onMinimize && (
                <button onClick={onMinimize} className="btn-secondary">
                  <Minus size={14} />
                  <span>Minimize to Background</span>
                </button>
              )}
              <button onClick={onCancelBackup} className="btn-danger">
                Cancel Backup
              </button>
            </div>
          ) : (
            <>
              <button onClick={onClose} className="btn-secondary">
                {backupResult ? 'Close' : 'Cancel'}
              </button>
              <button
                onClick={handleLaunch}
                className={`btn-primary ${procCheck?.is_running && !forceBypass ? 'btn-locked-warning' : ''}`}
                disabled={resolvedSources.length === 0}
              >
                {procCheck?.is_running && !forceBypass ? (
                  <span className="flex items-center gap-1.5">
                    <AlertTriangle size={14} />
                    <span>Close Minecraft to Backup</span>
                  </span>
                ) : (
                  <span className="flex items-center gap-1.5">
                    <Play size={14} />
                    <span>Start Minecraft Backup Now</span>
                  </span>
                )}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
};
