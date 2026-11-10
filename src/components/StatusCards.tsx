import React from 'react';
import {
  CheckCircle,
  AlertTriangle,
  XCircle,
  Clock,
  Layers,
  ShieldCheck,
  Wifi,
  WifiOff,
  Cpu,
} from 'lucide-react';
import { DashboardSummary, Profile } from '../types';
import { formatRelativeTime, formatExactTimestamp, formatDuration } from '../utils/format';

interface StatusCardsProps {
  summary: DashboardSummary | null;
  selectedProfile: Profile | null;
  onOpenSchedulerWizard: () => void;
  onRunIntegrityCheck: () => void;
  onRecordRestoreTest: () => void;
  checkingIntegrity: boolean;
}

export const StatusCards: React.FC<StatusCardsProps> = ({
  summary,
  selectedProfile,
  onOpenSchedulerWizard,
  onRunIntegrityCheck,
  onRecordRestoreTest,
  checkingIntegrity,
}) => {
  const connStatus = summary?.connection_status || 'offline';
  const lastSuccess = summary?.last_successful_backup;
  const lastAttempt = summary?.last_attempted_backup;
  const autoInstalled = summary?.automatic_backups_installed || false;
  const totalSnapshots = summary?.total_retained_snapshots ?? 0;
  const oldestPoint = summary?.oldest_restore_point;
  const lastCheck = summary?.last_integrity_check;
  const lastRestoreTest = summary?.last_confirmed_restore_test;

  return (
    <div className="status-grid">
      {/* 1. Connection Status Card */}
      <div className={`card status-card conn-${connStatus}`}>
        <div className="card-header">
          <span className="card-title">Repository Connection</span>
          {connStatus === 'connected' ? (
            <Wifi size={18} className="text-success" />
          ) : (
            <WifiOff size={18} className="text-danger" />
          )}
        </div>
        <div className="card-body">
          <div className="status-val text-capitalize">{connStatus}</div>
          <div className="status-desc">
            {connStatus === 'connected'
              ? 'Read-only connection active'
              : summary?.connection_error || 'Repository not connected'}
          </div>
        </div>
        <div className="card-footer">
          <span className="footer-label">Target:</span>
          <span className="footer-value truncate">{selectedProfile?.name || 'All Profiles'}</span>
        </div>
      </div>

      {/* 2. Last Successful Backup */}
      <div className="card status-card">
        <div className="card-header">
          <span className="card-title">Last Verified Backup</span>
          {lastSuccess ? (
            <CheckCircle size={18} className="text-success" />
          ) : (
            <Clock size={18} className="text-muted" />
          )}
        </div>
        <div className="card-body">
          {lastSuccess ? (
            <>
              <div className="status-val">{formatRelativeTime(lastSuccess.end_time)}</div>
              <div className="status-desc" title={formatExactTimestamp(lastSuccess.end_time)}>
                {formatExactTimestamp(lastSuccess.end_time)}
              </div>
              <div className="badge-pill verified-pill" title="Execution verified by local runner recording">
                Local Runner Verified
              </div>
            </>
          ) : (
            <>
              <div className="status-val text-muted">No Run Record</div>
              <div className="status-desc">
                {totalSnapshots > 0
                  ? 'Historical snapshots exist, but no app-verified run recorded on this host.'
                  : 'No backups have run for this profile yet.'}
              </div>
            </>
          )}
        </div>
        <div className="card-footer">
          <span className="footer-label">Added Data:</span>
          <span className="footer-value">
            {lastSuccess ? `${(lastSuccess.data_added_bytes / (1024 * 1024)).toFixed(1)} MiB` : 'N/A'}
          </span>
        </div>
      </div>

      {/* 3. Last Attempted Run & Result */}
      <div className="card status-card">
        <div className="card-header">
          <span className="card-title">Last Attempted Run</span>
          {lastAttempt?.status === 'success' ? (
            <CheckCircle size={18} className="text-success" />
          ) : lastAttempt?.status === 'partial_success' ? (
            <AlertTriangle size={18} className="text-warning" />
          ) : lastAttempt?.status === 'failed' ? (
            <XCircle size={18} className="text-danger" />
          ) : (
            <Clock size={18} className="text-muted" />
          )}
        </div>
        <div className="card-body">
          {lastAttempt ? (
            <>
              <div className={`status-val status-text-${lastAttempt.status}`}>
                {lastAttempt.status === 'partial_success'
                  ? 'Partial Success (Exit 3)'
                  : lastAttempt.status.toUpperCase()}
              </div>
              <div className="status-desc">
                Duration: {formatDuration(lastAttempt.duration_seconds)} | Exit code: {lastAttempt.exit_code}
              </div>
              {lastAttempt.error_message && (
                <div className="status-error-hint truncate" title={lastAttempt.error_message}>
                  {lastAttempt.error_message}
                </div>
              )}
            </>
          ) : (
            <>
              <div className="status-val text-muted">None</div>
              <div className="status-desc">No attempt logged on this machine.</div>
            </>
          )}
        </div>
        <div className="card-footer">
          <span className="footer-label">Files Processed:</span>
          <span className="footer-value">{lastAttempt ? lastAttempt.total_files_processed : 0}</span>
        </div>
      </div>

      {/* 4. Automatic Scheduler */}
      <div className="card status-card">
        <div className="card-header">
          <span className="card-title">Automatic Backups</span>
          <Cpu size={18} className={autoInstalled ? 'text-primary' : 'text-muted'} />
        </div>
        <div className="card-body">
          <div className="status-val">
            {autoInstalled ? (
              <span className="text-success">Installed & Active</span>
            ) : (
              <span className="text-muted">Not Installed</span>
            )}
          </div>
          <div className="status-desc">
            {autoInstalled
              ? `Next scheduled: ${summary?.next_scheduled_run || 'Registered with OS scheduler'}`
              : 'Runs solely on manual trigger from USB controller'}
          </div>
        </div>
        <div className="card-footer">
          <button onClick={onOpenSchedulerWizard} className="link-button">
            {autoInstalled ? 'Manage Local Setup' : 'Set Up Auto-Backup'}
          </button>
        </div>
      </div>

      {/* 5. Snapshots & Retention */}
      <div className="card status-card">
        <div className="card-header">
          <span className="card-title">Retained Snapshots</span>
          <Layers size={18} className="text-info" />
        </div>
        <div className="card-body">
          <div className="status-val">{totalSnapshots}</div>
          <div className="status-desc">
            {oldestPoint ? `Oldest: ${formatRelativeTime(oldestPoint)}` : 'No snapshots found'}
          </div>
        </div>
        <div className="card-footer">
          <span className="footer-label">Oldest Point:</span>
          <span className="footer-value" title={oldestPoint ? formatExactTimestamp(oldestPoint) : 'N/A'}>
            {oldestPoint ? new Date(oldestPoint).toLocaleDateString() : 'None'}
          </span>
        </div>
      </div>

      {/* 6. Integrity & Restore Test Verification */}
      <div className="card status-card">
        <div className="card-header">
          <span className="card-title">Integrity & Restore Tests</span>
          <ShieldCheck size={18} className={lastCheck?.success ? 'text-success' : 'text-muted'} />
        </div>
        <div className="card-body">
          <div className="integrity-row">
            <span className="integrity-label">Integrity Check:</span>
            {lastCheck ? (
              <span className={lastCheck.success ? 'text-success' : 'text-danger'}>
                {lastCheck.success ? 'Passed' : 'Failed'} ({formatRelativeTime(lastCheck.timestamp)})
              </span>
            ) : (
              <span className="text-muted">Not yet run</span>
            )}
          </div>
          <div className="integrity-row">
            <span className="integrity-label">Restore Test:</span>
            {lastRestoreTest ? (
              <span className="text-success" title={lastRestoreTest.notes}>
                Verified ({formatRelativeTime(lastRestoreTest.timestamp)})
              </span>
            ) : (
              <span className="text-muted">Unverified test</span>
            )}
          </div>
        </div>
        <div className="card-footer buttons-footer">
          <button
            onClick={onRunIntegrityCheck}
            disabled={checkingIntegrity}
            className="btn-tiny"
            title="Run restic check structural consistency"
          >
            {checkingIntegrity ? 'Checking...' : 'Check Repo'}
          </button>
          <button onClick={onRecordRestoreTest} className="btn-tiny" title="Confirm a successful restore drill">
            Record Restore Test
          </button>
        </div>
      </div>
    </div>
  );
};
