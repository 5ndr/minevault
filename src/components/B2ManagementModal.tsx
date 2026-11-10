import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Cloud, CheckCircle, AlertTriangle, X, RefreshCw, Key, ShieldCheck, Sparkles } from 'lucide-react';
import { RepoConfig, SessionCredentials, B2ProviderUsage, B2PurgeResult } from '../types';
import { formatBytes, formatExactTimestamp, calculateB2Pricing } from '../utils/format';

interface B2ManagementModalProps {
  isOpen: boolean;
  onClose: () => void;
  repo: RepoConfig | null;
  creds: SessionCredentials | null;
  onMeasure: (keyId?: string, appKey?: string) => Promise<B2ProviderUsage>;
  onPurgeVersions?: () => Promise<B2PurgeResult>;
  currentUsage?: B2ProviderUsage | null;
}

export const B2ManagementModal: React.FC<B2ManagementModalProps> = ({
  isOpen,
  onClose,
  repo,
  creds,
  onMeasure,
  onPurgeVersions,
  currentUsage,
}) => {
  if (!isOpen) return null;

  const [useRepoCreds, setUseRepoCreds] = useState<boolean>(
    !repo?.b2_management_key_id && Boolean(repo?.b2_key_id)
  );
  const [managementKeyId, setManagementKeyId] = useState<string>(
    repo?.b2_management_key_id || repo?.b2_key_id || ''
  );
  const [managementAppKey, setManagementAppKey] = useState<string>(
    creds?.b2_management_app_key || creds?.b2_application_key || ''
  );
  const [measuring, setMeasuring] = useState<boolean>(false);
  const [purging, setPurging] = useState<boolean>(false);
  const [purgeSuccessMessage, setPurgeSuccessMessage] = useState<string | null>(null);
  const [saveToUsb, setSaveToUsb] = useState<boolean>(true);
  const [result, setResult] = useState<B2ProviderUsage | null>(currentUsage || null);
  const [error, setError] = useState<string | null>(null);

  const handleStartMeasurement = async () => {
    setMeasuring(true);
    setError(null);
    setPurgeSuccessMessage(null);
    try {
      const keyId = useRepoCreds ? repo?.b2_key_id : managementKeyId;
      const appKey = useRepoCreds ? creds?.b2_application_key : managementAppKey;
      const usage = await onMeasure(keyId, appKey);
      setResult(usage);

      if (saveToUsb && repo) {
        try {
          await invoke('save_usb_vault_cmd', {
            repo: {
              ...repo,
              b2_management_key_id: keyId || repo.b2_management_key_id,
            },
            creds: {
              ...creds,
              b2_management_app_key: appKey || creds?.b2_management_app_key,
            },
            passphrase: null,
          });
        } catch (_) {}
      }
    } catch (e: any) {
      setError(e?.toString() || 'Failed to query B2 bucket usage');
    } finally {
      setMeasuring(false);
    }
  };

  const handleExecutePurge = async () => {
    if (!onPurgeVersions) return;
    setPurging(true);
    setPurgeSuccessMessage(null);
    setError(null);
    try {
      const purgeRes = await onPurgeVersions();
      setResult(purgeRes.updated_usage);
      setPurgeSuccessMessage(
        `Successfully deleted ${purgeRes.deleted_files_count} obsolete files and freed ${formatBytes(purgeRes.freed_bytes)}! Physical bucket size is now ${formatBytes(purgeRes.updated_usage.total_bytes)}.`
      );
    } catch (e: any) {
      setError(e?.toString() || 'Failed to purge obsolete B2 versions');
    } finally {
      setPurging(false);
    }
  };

  return (
    <div className="modal-backdrop">
      <div className="modal-box modal-md">
        <div className="modal-header">
          <div className="modal-header-title">
            <Cloud size={18} className="text-primary" />
            <h3>Direct B2 Provider Bucket Measurement</h3>
          </div>
          <button onClick={onClose} className="close-btn" disabled={measuring}>
            <X size={18} />
          </button>
        </div>

        <div className="modal-body">
          <p className="text-muted">
            Queries the Backblaze B2 Native API directly to calculate total physical bucket storage,
            independent of restic's local content-addressing and deduplication math.
          </p>

          <div className="form-group mt-3">
            <label className="text-xs font-semibold block mb-2">Authentication Method:</label>
            <div className="radio-group-tabs">
              <button
                type="button"
                className={`tab-btn ${useRepoCreds ? 'active' : ''}`}
                onClick={() => setUseRepoCreds(true)}
                disabled={measuring}
              >
                <ShieldCheck size={15} />
                <span>Use Repository B2 Key</span>
              </button>
              <button
                type="button"
                className={`tab-btn ${!useRepoCreds ? 'active' : ''}`}
                onClick={() => setUseRepoCreds(false)}
                disabled={measuring}
              >
                <Key size={15} />
                <span>Dedicated Management Token</span>
              </button>
            </div>
          </div>

          {useRepoCreds ? (
            <div className="repo-b2-summary-box mt-3">
              <div className="text-xs text-muted">Active Repository B2 Key:</div>
              <div className="font-mono text-sm mt-1">
                {repo?.b2_key_id ? repo.b2_key_id : 'No Key ID saved on repository profile'}
              </div>
              <span className="form-hint mt-1">
                Uses the same Application Key ID and Secret Key currently configured for repository backup & restore.
              </span>
            </div>
          ) : (
            <div className="management-inputs-group mt-3">
              <div className="form-group">
                <label>B2 Management Key ID (or Master Key ID):</label>
                <input
                  type="text"
                  value={managementKeyId}
                  onChange={(e) => setManagementKeyId(e.target.value)}
                  placeholder="004..."
                  disabled={measuring}
                />
              </div>

              <div className="form-group">
                <label>B2 Secret Application Key:</label>
                <input
                  type="password"
                  value={managementAppKey}
                  onChange={(e) => setManagementAppKey(e.target.value)}
                  placeholder="K004..."
                  disabled={measuring}
                />
                <span className="form-hint">
                  Requires <code>listBuckets</code> and <code>listFileNames</code> capabilities to calculate physical object storage.
                </span>
              </div>
            </div>
          )}

          <label className="checkbox-row mt-3">
            <input
              type="checkbox"
              checked={saveToUsb}
              onChange={(e) => setSaveToUsb(e.target.checked)}
              disabled={measuring}
            />
            <span className="text-xs">Keep B2 credentials encrypted on USB drive (AES-256-GCM)</span>
          </label>

          {result && (() => {
            const pricing = calculateB2Pricing(result.total_bytes);
            const hasObsoleteVersions = Boolean(
              result.hidden_version_bytes && result.hidden_version_bytes > 0
            );

            return (
              <div className="space-y-3 mt-3">
                <div className="b2-measurement-results-card">
                  <div className="flex items-center gap-2 text-success font-semibold text-sm">
                    <CheckCircle size={16} />
                    <span>Direct Provider Measurement Verified</span>
                  </div>
                  <div className="b2-stats-grid mt-2">
                    <div className="stat-pill">
                      <span className="text-xs text-muted block">Provider Physical Size:</span>
                      <strong className="text-base text-primary">{formatBytes(result.total_bytes)}</strong>
                    </div>
                    <div className="stat-pill">
                      <span className="text-xs text-muted block">~Rough Estimated Cost:</span>
                      <strong className="text-base text-emerald-400">{pricing.monthlyFormatted}</strong>
                      <span className="text-[11px] text-muted block">({pricing.yearlyFormatted})</span>
                    </div>
                    <div className="stat-pill">
                      <span className="text-xs text-muted block">Total Remote Objects:</span>
                      <strong className="text-base">{result.file_count.toLocaleString()} files</strong>
                    </div>
                    <div className="stat-pill">
                      <span className="text-xs text-muted block">Bucket / Prefix:</span>
                      <span className="text-xs font-mono">{result.bucket_name} {result.prefix ? `(${result.prefix})` : ''}</span>
                    </div>
                  </div>
                  <div className="text-xs text-muted mt-2 pt-2 border-t border-emerald-500/20 flex items-center justify-between">
                    <span>Pricing breakdown: {pricing.rateDescription}</span>
                    <span className="text-[11px] text-muted">Measured: {formatExactTimestamp(result.measured_at)}</span>
                  </div>
                  {result.is_partial && (
                    <div className="text-warning text-xs mt-2">
                      * Large bucket detected: measurement capped at 25,000 objects for performance.
                    </div>
                  )}
                </div>

                {/* Obsolete Hidden Versions Alert & Purge Card */}
                {hasObsoleteVersions && (
                  <div className="b2-version-diagnostic-box">
                    <div className="flex items-center gap-2 text-warning font-semibold text-sm">
                      <AlertTriangle size={16} />
                      <span>Obsolete &amp; Deleted Versions Inflating Storage</span>
                    </div>
                    <p className="text-xs text-muted mt-1 leading-relaxed">
                      Backblaze B2 bucket lifecycle defaults to &quot;Keep all versions forever&quot;. When restic deletes or prunes old backup snapshots, Backblaze marks them hidden rather than deleting the physical data, keeping dead files in your bucket!
                    </p>

                    <div className="version-breakdown-row mt-2.5">
                      <div className="breakdown-stat">
                        <span className="text-[11px] text-muted block">Active Backup Blobs:</span>
                        <strong className="text-sm text-primary">{formatBytes(result.active_bytes || 0)}</strong>
                        <span className="text-[10px] text-muted block">({result.active_file_count || 0} active files)</span>
                      </div>
                      <div className="breakdown-stat">
                        <span className="text-[11px] text-amber-400 block">Obsolete Hidden Versions:</span>
                        <strong className="text-sm text-amber-400">{formatBytes(result.hidden_version_bytes || 0)}</strong>
                        <span className="text-[10px] text-muted block">({result.hidden_version_count || 0} obsolete files)</span>
                      </div>
                      <div className="breakdown-stat">
                        <span className="text-[11px] text-muted block">Backblaze Panel Total:</span>
                        <strong className="text-sm">{formatBytes(result.total_bytes)}</strong>
                        <span className="text-[10px] text-muted block">({result.file_count} total objects)</span>
                      </div>
                    </div>

                    {onPurgeVersions && (
                      <div className="mt-3 pt-2 border-t border-amber-500/20 flex items-center justify-between gap-3">
                        <span className="text-[11px] text-muted">
                          Enables Backblaze 1-day auto-delete rule and immediately purges obsolete versions.
                        </span>
                        <button
                          type="button"
                          onClick={handleExecutePurge}
                          disabled={purging || measuring}
                          className="btn-warning text-xs py-1.5 px-3 flex items-center gap-1.5 flex-shrink-0"
                          title="Purge dead versions and save B2 storage"
                        >
                          {purging ? (
                            <>
                              <RefreshCw size={13} className="animate-spin" />
                              <span>Purging Dead Versions...</span>
                            </>
                          ) : (
                            <>
                              <Sparkles size={13} />
                              <span>Purge {formatBytes(result.hidden_version_bytes || 0)} Obsolete Files</span>
                            </>
                          )}
                        </button>
                      </div>
                    )}
                  </div>
                )}
              </div>
            );
          })()}

          {purgeSuccessMessage && (
            <div className="alert-banner alert-success mt-3">
              <CheckCircle size={16} className="flex-shrink-0" />
              <span>{purgeSuccessMessage}</span>
            </div>
          )}

          {error && (
            <div className="alert-banner alert-error mt-3">
              <AlertTriangle size={16} className="flex-shrink-0" />
              <span>{error}</span>
            </div>
          )}
        </div>

        <div className="modal-footer">
          <button onClick={onClose} className="btn-secondary" disabled={measuring}>
            {result ? 'Done' : 'Cancel'}
          </button>
          <button
            onClick={handleStartMeasurement}
            disabled={measuring || (!useRepoCreds && (!managementKeyId.trim() || !managementAppKey.trim()))}
            className="btn-primary"
          >
            <RefreshCw size={14} className={measuring ? 'animate-spin' : ''} />
            <span>{measuring ? 'Measuring B2 Bucket...' : result ? 'Measure Again' : 'Measure Provider Bucket Size'}</span>
          </button>
        </div>
      </div>
    </div>
  );
};
