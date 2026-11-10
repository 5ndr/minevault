import React, { useState } from 'react';
import {
  AlertTriangle,
  AlertOctagon,
  Info,
  CheckCircle,
  XCircle,
  ChevronDown,
  ChevronUp,
  Clock,
  Terminal,
  FileCheck,
} from 'lucide-react';
import { DashboardAlert, RunResult } from '../types';
import { formatBytes, formatDuration, formatExactTimestamp, formatRelativeTime } from '../utils/format';

interface ActivityAlertsProps {
  alerts: DashboardAlert[];
  recentRuns: RunResult[];
}

export const ActivityAlerts: React.FC<ActivityAlertsProps> = ({ alerts, recentRuns }) => {
  const [expandedLogRunId, setExpandedLogRunId] = useState<string | null>(null);

  const toggleLogs = (runId: string) => {
    setExpandedLogRunId(expandedLogRunId === runId ? null : runId);
  };

  return (
    <div className="activity-alerts-container">
      {/* Active Alerts Banner */}
      {alerts.length > 0 && (
        <div className="alerts-list">
          {alerts.map((alert, idx) => (
            <div key={idx} className={`alert-banner alert-${alert.level}`}>
              <div className="alert-icon">
                {alert.level === 'error' ? (
                  <AlertOctagon size={18} />
                ) : alert.level === 'warning' ? (
                  <AlertTriangle size={18} />
                ) : (
                  <Info size={18} />
                )}
              </div>
              <div className="alert-content">
                <strong className="alert-title">{alert.title}</strong>
                <p className="alert-message">{alert.message}</p>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Recent Runs Table */}
      <div className="card recent-runs-card">
        <div className="card-header">
          <div className="card-header-left">
            <Clock size={18} />
            <span className="card-title">Recent Backup Runs & Execution History</span>
          </div>
          <span className="badge-pill count-pill">{recentRuns.length} Recorded Runs</span>
        </div>

        <div className="card-body p-0">
          {recentRuns.length === 0 ? (
            <div className="empty-runs-state">
              <FileCheck size={28} className="text-muted" />
              <p>No recent backup runs recorded on this machine yet.</p>
              <span>Click "Run Manual Backup" or configure automatic scheduling to start protecting your data.</span>
            </div>
          ) : (
            <div className="table-responsive">
              <table className="runs-table">
                <thead>
                  <tr>
                    <th>Status</th>
                    <th>Profile</th>
                    <th>Start Time</th>
                    <th>Duration</th>
                    <th>Files Processed</th>
                    <th>New Data Added</th>
                    <th>Snapshot ID</th>
                    <th>Sanitized Logs</th>
                  </tr>
                </thead>
                <tbody>
                  {recentRuns.map((run) => (
                    <React.Fragment key={run.run_id}>
                      <tr className={`run-row row-status-${run.status}`}>
                        <td>
                          <span className={`status-badge badge-${run.status}`}>
                            {run.status === 'success' && <CheckCircle size={13} />}
                            {run.status === 'partial_success' && <AlertTriangle size={13} />}
                            {run.status === 'failed' && <XCircle size={13} />}
                            <span>
                              {run.status === 'partial_success' ? 'Partial (Exit 3)' : run.status}
                            </span>
                          </span>
                        </td>
                        <td className="font-medium">{run.profile_name}</td>
                        <td title={formatExactTimestamp(run.start_time)}>
                          {formatRelativeTime(run.start_time)}
                        </td>
                        <td>{formatDuration(run.duration_seconds)}</td>
                        <td>
                          {run.total_files_processed} ({run.files_new} new)
                        </td>
                        <td>{formatBytes(run.data_added_bytes)}</td>
                        <td>
                          {run.snapshot_id ? (
                            <code className="snapshot-tag">{run.snapshot_id.substring(0, 8)}</code>
                          ) : (
                            <span className="text-muted">—</span>
                          )}
                        </td>
                        <td>
                          <button
                            onClick={() => toggleLogs(run.run_id)}
                            className="btn-tiny btn-logs"
                            title="Expand sanitized execution logs"
                          >
                            <Terminal size={12} />
                            <span>Logs</span>
                            {expandedLogRunId === run.run_id ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
                          </button>
                        </td>
                      </tr>

                      {expandedLogRunId === run.run_id && (
                        <tr className="log-detail-row">
                          <td colSpan={8}>
                            <div className="sanitized-log-viewer">
                              <div className="log-viewer-header">
                                <span>Sanitized Process Output (Secrets Redacted)</span>
                                <span>Exit Code: {run.exit_code}</span>
                              </div>
                              <pre className="log-terminal">
                                {run.sanitized_logs || 'No output recorded.'}
                              </pre>
                            </div>
                          </td>
                        </tr>
                      )}
                    </React.Fragment>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
