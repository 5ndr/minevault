import React from 'react';
import {
  CheckCircle,
  AlertTriangle,
  X,
  Maximize2,
  Clock,
  HardDrive,
  FileText,
  StopCircle,
} from 'lucide-react';
import { BackupProgressEvent, RunResult } from '../types';
import { formatBytes, formatDuration } from '../utils/format';

interface FloatingBackupProgressProps {
  isRunning: boolean;
  progress: BackupProgressEvent | null;
  backupResult: RunResult | null;
  backupError: string | null;
  activeProfileName?: string;
  onExpand: () => void;
  onCancel: () => void;
  onDismissResult: () => void;
}

export const FloatingBackupProgress: React.FC<FloatingBackupProgressProps> = ({
  isRunning,
  progress,
  backupResult,
  backupError,
  activeProfileName = 'Minecraft Backup',
  onExpand,
  onCancel,
  onDismissResult,
}) => {
  // Only render if running, or if there's a recent result/error to show
  if (!isRunning && !backupResult && !backupError) {
    return null;
  }

  // Format estimated time remaining nicely
  const renderRemainingTime = () => {
    if (!progress) return 'Estimating remaining time...';
    if (progress.seconds_remaining !== undefined && progress.seconds_remaining !== null) {
      if (progress.seconds_remaining === 0) return 'Almost done...';
      const mins = Math.floor(progress.seconds_remaining / 60);
      const secs = progress.seconds_remaining % 60;
      if (mins > 0) {
        return `~${mins}m ${secs}s remaining`;
      }
      return `~${secs}s remaining`;
    }
    // Fallback: estimate from elapsed and percent
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

  return (
    <div className="floating-backup-dock" role="region" aria-label="Backup Progress Monitor">
      <div className="floating-dock-card">
        {/* Header */}
        <div className="dock-header">
          <div className="dock-title-group">
            {isRunning ? (
              <span className="pulse-indicator-dot" />
            ) : backupResult?.status === 'success' ? (
              <CheckCircle size={16} className="text-success" />
            ) : (
              <AlertTriangle size={16} className="text-warning" />
            )}
            <strong className="dock-title truncate">{activeProfileName}</strong>
          </div>

          <div className="dock-actions">
            <button
              onClick={onExpand}
              className="dock-icon-btn"
              title="Expand full backup details window"
              aria-label="Expand backup details"
            >
              <Maximize2 size={13} />
            </button>
            {!isRunning && (
              <button
                onClick={onDismissResult}
                className="dock-icon-btn"
                title="Dismiss progress notification"
                aria-label="Dismiss notification"
              >
                <X size={14} />
              </button>
            )}
          </div>
        </div>

        {/* Running state progress */}
        {isRunning && (
          <div className="dock-body">
            <div className="dock-progress-row">
              <div className="dock-progress-track">
                <div
                  className="dock-progress-fill"
                  style={{ width: `${progress ? Math.min(100, Math.max(2, progress.percent_done)) : 4}%` }}
                />
              </div>
              <span className="dock-percent-text">
                {progress ? `${progress.percent_done.toFixed(0)}%` : '0%'}
              </span>
            </div>

            <div className="dock-metrics-grid">
              <div className="dock-metric" title="Estimated time to complete backup">
                <Clock size={12} className="text-primary flex-shrink-0" />
                <span className="metric-text font-semibold">{renderRemainingTime()}</span>
              </div>

              {progress && (
                <div className="dock-metric" title="Elapsed execution time">
                  <span className="metric-text text-muted">Elapsed: {progress.seconds_elapsed}s</span>
                </div>
              )}

              {progress && (
                <div className="dock-metric col-span-2" title="Transfer size processed">
                  <HardDrive size={12} className="text-muted flex-shrink-0" />
                  <span className="metric-text">
                    {formatBytes(progress.bytes_done)} / {formatBytes(progress.total_bytes)}
                    {progress.total_files > 0 ? ` (${progress.files_done}/${progress.total_files} files)` : ''}
                  </span>
                </div>
              )}
            </div>

            {progress?.current_file && (
              <div className="dock-current-file truncate" title={progress.current_file}>
                <FileText size={11} className="flex-shrink-0 text-muted" />
                <span>{progress.current_file}</span>
              </div>
            )}

            <div className="dock-footer-row">
              <button
                onClick={onCancel}
                className="btn-cancel-compact"
                title="Cancel running backup"
              >
                <StopCircle size={13} />
                <span>Cancel</span>
              </button>
              <button
                onClick={onExpand}
                className="btn-expand-compact"
                title="Open detailed view"
              >
                View Details
              </button>
            </div>
          </div>
        )}

        {/* Finished Success state */}
        {!isRunning && backupResult && (
          <div className="dock-body dock-result-body">
            <div className="dock-result-message">
              <strong>
                {backupResult.status === 'success'
                  ? 'Backup Completed!'
                  : backupResult.status === 'partial_success'
                  ? 'Completed with Warnings'
                  : 'Backup Failed'}
              </strong>
              <p className="dock-result-summary">
                Added {formatBytes(backupResult.data_added_bytes)} in {formatDuration(backupResult.duration_seconds)}.
                {backupResult.snapshot_id && (
                  <span className="dock-snapshot-tag"> #{backupResult.snapshot_id.substring(0, 8)}</span>
                )}
              </p>
            </div>
            <div className="dock-footer-row">
              <button onClick={onExpand} className="btn-expand-compact">
                View Summary
              </button>
              <button onClick={onDismissResult} className="btn-dismiss-compact">
                Dismiss
              </button>
            </div>
          </div>
        )}

        {/* Finished Error state */}
        {!isRunning && backupError && (
          <div className="dock-body dock-error-body">
            <div className="dock-error-message truncate" title={backupError}>
              {backupError}
            </div>
            <div className="dock-footer-row">
              <button onClick={onExpand} className="btn-expand-compact">
                Inspect Error
              </button>
              <button onClick={onDismissResult} className="btn-dismiss-compact">
                Dismiss
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
