import React, { useState } from 'react';
import {
  PieChart as PieIcon,
  TrendingUp,
  HardDrive,
  RefreshCw,
  Info,
  Cloud,
  AlertTriangle,
} from 'lucide-react';
import { FolderSizeBreakdown, StorageOverview, ResticSnapshot } from '../types';
import { formatBytes, formatExactTimestamp, calculateB2Pricing } from '../utils/format';

interface StorageSectionProps {
  storage: StorageOverview | null;
  snapshots: ResticSnapshot[];
  selectedSnapshotId: string | null;
  onSelectSnapshot: (id: string) => void;
  folderBreakdown: FolderSizeBreakdown[];
  loadingBreakdown: boolean;
  breakdownError?: string | null;
  onRetryBreakdown?: () => void;
  onRefreshStorage: () => void;
  refreshing: boolean;
  localFreeSpace: number | null;
  restoreTargetDir: string;
  onOpenB2ManagementModal?: () => void;
}

export const StorageSection: React.FC<StorageSectionProps> = ({
  storage,
  snapshots,
  selectedSnapshotId,
  onSelectSnapshot,
  folderBreakdown,
  loadingBreakdown,
  breakdownError,
  onRetryBreakdown,
  onRefreshStorage,
  refreshing,
  localFreeSpace,
  restoreTargetDir,
  onOpenB2ManagementModal,
}) => {
  const [showTableAlt, setShowTableAlt] = useState(false);

  // Palette of high-contrast, accessible chart colors
  const CHART_COLORS = [
    '#3b82f6', // blue
    '#10b981', // emerald
    '#f59e0b', // amber
    '#8b5cf6', // purple
    '#ec4899', // pink
    '#06b6d4', // cyan
    '#64748b', // slate
  ];

  // Calculate SVG donut slices
  let cumulativePercent = 0;
  const donutSlices = folderBreakdown.map((item, idx) => {
    const startAngle = cumulativePercent * 3.6; // 360 deg
    cumulativePercent += item.percentage;
    const endAngle = cumulativePercent * 3.6;

    const startRad = ((startAngle - 90) * Math.PI) / 180;
    const endRad = ((endAngle - 90) * Math.PI) / 180;

    const x1 = 100 + 75 * Math.cos(startRad);
    const y1 = 100 + 75 * Math.sin(startRad);
    const x2 = 100 + 75 * Math.cos(endRad);
    const y2 = 100 + 75 * Math.sin(endRad);

    const largeArc = item.percentage > 50 ? 1 : 0;
    const pathData =
      item.percentage >= 99.9
        ? `M 100 25 A 75 75 0 1 1 99.9 25`
        : `M ${x1} ${y1} A 75 75 0 ${largeArc} 1 ${x2} ${y2}`;

    return {
      ...item,
      color: CHART_COLORS[idx % CHART_COLORS.length],
      pathData,
    };
  });

  const rawBytes = storage?.raw_data_size ?? 0;
  const restoreBytes = storage?.restore_size ?? 0;
  const dedupRatio = rawBytes > 0 ? (restoreBytes / rawBytes).toFixed(1) : '1.0';
  const b2Pricing = storage?.provider_b2_usage
    ? calculateB2Pricing(storage.provider_b2_usage)
    : (rawBytes > 0 ? calculateB2Pricing(rawBytes) : null);

  return (
    <div className="storage-section-container">
      <div className="section-header-row">
        <div className="section-title-group">
          <h2>Storage Statistics & Repository Growth</h2>
          <span className="section-subtitle">
            Accurate deduplication accounting. Physical storage is measured across shared repo blobs.
          </span>
        </div>
        <div className="section-actions">
          <button
            onClick={onRefreshStorage}
            disabled={refreshing}
            className="btn-secondary"
            title="Recalculate stats via restic stats in background"
          >
            <RefreshCw size={14} className={refreshing ? 'spin' : ''} />
            <span>{refreshing ? 'Calculating...' : 'Recalculate Stats'}</span>
          </button>
        </div>
      </div>

      {/* Storage Overview Metrics Banner */}
      <div className="storage-metrics-banner">
        <div className="metric-box">
          <div className="metric-header">
            <span>Deduplicated Repository Size</span>
            <span className="method-pill" title="Measured via: restic stats --mode raw-data">
              raw-data
            </span>
          </div>
          <div className="metric-value">{formatBytes(rawBytes)}</div>
          <div className="metric-desc">Actual physical chunk blobs stored in Backblaze B2 / storage target</div>
        </div>

        <div className="metric-box">
          <div className="metric-header">
            <span>Logical Restore Size</span>
            <span className="method-pill" title="Measured via: restic stats --mode restore-size">
              restore-size
            </span>
          </div>
          <div className="metric-value">{formatBytes(restoreBytes)}</div>
          <div className="metric-desc">Uncompressed total file size if all snapshots were extracted</div>
        </div>

        <div className="metric-box">
          <div className="metric-header">
            <span>Deduplication Efficiency</span>
            <span className="method-pill">savings</span>
          </div>
          <div className="metric-value text-success">{dedupRatio}x</div>
          <div className="metric-desc">
            Logical data is {dedupRatio}x larger than raw repository storage due to content deduplication.
          </div>
        </div>

        <div className="metric-box">
          <div className="metric-header">
            <span>Provider B2 S3 Usage</span>
            <span className="method-pill">B2 direct</span>
          </div>

          <div className="flex items-baseline justify-between gap-2">
            <div className={`metric-value ${storage?.provider_b2_usage ? 'text-primary' : 'text-muted'}`}>
              {storage?.provider_b2_usage ? formatBytes(storage.provider_b2_usage) : 'Unavailable'}
            </div>
            {b2Pricing && (
              <div
                className="b2-pricing-pill"
                title={`Backblaze B2 Pricing Estimate: ${b2Pricing.rateDescription}`}
              >
                <span className="pricing-monthly text-emerald-400 font-bold text-sm">
                  {b2Pricing.monthlyFormatted}
                </span>
                <span className="pricing-yearly text-muted text-xs ml-1">
                  ({b2Pricing.yearlyFormatted})
                </span>
              </div>
            )}
          </div>

          <div className="metric-desc">
            {storage?.b2_provider_details ? (
              <div className="space-y-1">
                <div className="b2-pricing-info-row flex items-center gap-1.5 text-xs text-muted">
                  <span className="text-emerald-400 font-medium">~Rough Pricing:</span>
                  <span className="text-primary font-semibold">{b2Pricing?.monthlyFormatted}</span>
                  <span className="text-muted text-[11px]">({b2Pricing?.rateDescription})</span>
                </div>
                <span>
                  {storage.b2_provider_details.file_count.toLocaleString()} objects in bucket {storage.b2_provider_details.bucket_name}
                </span>
                {storage.b2_provider_details.hidden_version_bytes && storage.b2_provider_details.hidden_version_bytes > 0 ? (
                  <div className="mt-1 text-xs text-amber-400 bg-amber-500/10 border border-amber-500/20 rounded p-1.5">
                    <span>⚠️ <strong>{formatBytes(storage.b2_provider_details.hidden_version_bytes)}</strong> obsolete versions retained by Backblaze.</span>
                  </div>
                ) : null}
                {onOpenB2ManagementModal && (
                  <div>
                    <button
                      type="button"
                      onClick={onOpenB2ManagementModal}
                      className="btn-tiny text-xs mt-1 text-primary hover:underline flex items-center gap-1"
                    >
                      <RefreshCw size={11} />
                      <span>
                        {storage.b2_provider_details.hidden_version_bytes && storage.b2_provider_details.hidden_version_bytes > 0
                          ? 'Clean Up Obsolete B2 Versions'
                          : 'Update B2 Measurement'}
                      </span>
                    </button>
                  </div>
                )}
              </div>
            ) : (
              <div>
                <span>Direct provider bucket size requires B2 Management API credentials.</span>
                {b2Pricing && (
                  <div className="text-xs text-muted mt-1">
                    Est. rough pricing: <strong className="text-emerald-400">{b2Pricing.monthlyFormatted}</strong> ({b2Pricing.rateDescription})
                  </div>
                )}
                {onOpenB2ManagementModal && (
                  <button
                    type="button"
                    onClick={onOpenB2ManagementModal}
                    className="btn-tiny btn-primary-outline mt-1.5 flex items-center gap-1"
                  >
                    <Cloud size={12} />
                    <span>Configure & Measure B2</span>
                  </button>
                )}
              </div>
            )}
          </div>
        </div>
      </div>

      <div className="storage-charts-grid">
        {/* Left: Donut Chart Breakdown for Selected Snapshot */}
        <div className="card chart-card">
          <div className="chart-card-header">
            <div className="chart-title">
              <PieIcon size={18} />
              <span>Snapshot File-Size Breakdown</span>
            </div>
            <div className="chart-controls">
              <select
                value={selectedSnapshotId || ''}
                onChange={(e) => onSelectSnapshot(e.target.value)}
                className="snapshot-select"
                title="Select snapshot to inspect logical directory breakdown"
              >
                {snapshots.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.short_id} ({new Date(s.time).toLocaleDateString()} - {s.hostname})
                  </option>
                ))}
              </select>
              <button
                onClick={() => setShowTableAlt(!showTableAlt)}
                className="btn-tiny"
                title="Toggle accessible table view"
              >
                {showTableAlt ? 'Show Donut' : 'Show Table'}
              </button>
            </div>
          </div>

          <div className="chart-card-body">
            {loadingBreakdown ? (
              <div className="chart-loading flex items-center justify-center gap-2">
                <RefreshCw size={15} className="animate-spin text-primary" />
                <span>Analyzing snapshot directory sizes...</span>
              </div>
            ) : breakdownError ? (
              <div className="chart-empty text-warning">
                <AlertTriangle size={18} className="mx-auto mb-2 text-warning" />
                <div>Failed to analyze snapshot directories:</div>
                <div className="text-xs text-muted mt-1 max-w-sm mx-auto">{breakdownError}</div>
                {onRetryBreakdown && (
                  <button onClick={onRetryBreakdown} className="btn-tiny mt-3 mx-auto">
                    Retry Analysis
                  </button>
                )}
              </div>
            ) : folderBreakdown.length === 0 ? (
              <div className="chart-empty">
                <div>No folder breakdown data available for this snapshot.</div>
                {onRetryBreakdown && (
                  <button onClick={onRetryBreakdown} className="btn-tiny mt-2 mx-auto">
                    Analyze Directory Breakdown
                  </button>
                )}
              </div>
            ) : showTableAlt ? (
              <div className="table-wrapper">
                <table className="data-table">
                  <thead>
                    <tr>
                      <th>Source Category / Folder</th>
                      <th>Logical Size</th>
                      <th>Files</th>
                      <th>Share</th>
                    </tr>
                  </thead>
                  <tbody>
                    {folderBreakdown.map((item, idx) => (
                      <tr key={item.folder_path}>
                        <td>
                          <span
                            className="color-dot"
                            style={{ backgroundColor: CHART_COLORS[idx % CHART_COLORS.length] }}
                          />
                          {item.folder_path}
                        </td>
                        <td>{formatBytes(item.size_bytes)}</td>
                        <td>{item.file_count}</td>
                        <td>{item.percentage.toFixed(1)}%</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="donut-container">
                <svg viewBox="0 0 200 200" className="donut-svg" role="img" aria-label="Snapshot breakdown chart">
                  <circle cx="100" cy="100" r="75" fill="none" stroke="var(--border-color)" strokeWidth="24" />
                  {donutSlices.map((slice) => (
                    <path
                      key={slice.folder_path}
                      d={slice.pathData}
                      fill="none"
                      stroke={slice.color}
                      strokeWidth="24"
                      strokeLinecap="butt"
                    />
                  ))}
                  <text x="100" y="95" textAnchor="middle" className="donut-center-top">
                    {folderBreakdown.length} Categories
                  </text>
                  <text x="100" y="115" textAnchor="middle" className="donut-center-bottom">
                    Logical Total
                  </text>
                </svg>

                <div className="donut-legend">
                  {folderBreakdown.slice(0, 6).map((item, idx) => (
                    <div key={item.folder_path} className="legend-item">
                      <span
                        className="legend-color"
                        style={{ backgroundColor: CHART_COLORS[idx % CHART_COLORS.length] }}
                      />
                      <span className="legend-label truncate" title={item.folder_path}>
                        {item.folder_path}
                      </span>
                      <span className="legend-value">{formatBytes(item.size_bytes)}</span>
                      <span className="legend-pct">({item.percentage.toFixed(0)}%)</span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
          <div className="chart-card-footer">
            <Info size={14} className="text-muted" />
            <span>
              Logical file sizes represent uncompressed restore volume. Deduplicated chunks are shared across snapshots.
            </span>
          </div>
        </div>

        {/* Right: Real Growth History Graph & Local Space */}
        <div className="card chart-card">
          <div className="chart-card-header">
            <div className="chart-title">
              <TrendingUp size={18} />
              <span>Verified Repository Growth History</span>
              {storage?.is_demo && <span className="demo-tag">DEMO DATA</span>}
            </div>
            <div className="measurement-ts" title="Last measured timestamp">
              {formatExactTimestamp(storage?.calculated_at)}
            </div>
          </div>

          <div className="chart-card-body">
            {(!storage || storage.growth_history.length === 0) ? (
              <div className="chart-empty">
                No growth measurements recorded yet. Growth data accumulates with each verified calculation.
              </div>
            ) : (
              <div className="growth-history-chart">
                <div className="bars-container">
                  {storage.growth_history.map((pt, i) => {
                    const maxBytes = Math.max(
                      ...storage.growth_history.map((p) => Math.max(p.raw_bytes, p.restore_bytes)),
                      1
                    );
                    const rawHeight = Math.min(100, Math.max(8, (pt.raw_bytes / maxBytes) * 100));
                    const restoreHeight = Math.min(100, Math.max(8, (pt.restore_bytes / maxBytes) * 100));

                    return (
                      <div key={i} className="history-bar-col">
                        <div className="bars-pair">
                          <div
                            className="bar-restore"
                            style={{ height: `${restoreHeight}%` }}
                            title={`Logical Restore Size: ${formatBytes(pt.restore_bytes)}`}
                          />
                          <div
                            className="bar-raw"
                            style={{ height: `${rawHeight}%` }}
                            title={`Raw Repository Size: ${formatBytes(pt.raw_bytes)}`}
                          />
                        </div>
                        <span className="history-date">
                          {new Date(pt.timestamp).toLocaleDateString(undefined, { month: 'numeric', day: 'numeric' })}
                        </span>
                      </div>
                    );
                  })}
                </div>

                <div className="chart-legend-row">
                  <div className="legend-indicator">
                    <span className="legend-swatch bar-raw-swatch" />
                    <span>Deduplicated Raw Blobs</span>
                  </div>
                  <div className="legend-indicator">
                    <span className="legend-swatch bar-restore-swatch" />
                    <span>Logical Restore Size</span>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Local Free Disk Space on Restore Destination */}
          <div className="chart-card-footer local-space-footer">
            <div className="local-space-info">
              <HardDrive size={15} />
              <span>Restore Destination Free Space ({restoreTargetDir || 'Default Target'}):</span>
              <strong>{localFreeSpace ? formatBytes(localFreeSpace) : 'Calculating...'}</strong>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
