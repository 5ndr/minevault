import React, { useState } from 'react';
import {
  Cpu,
  Shield,
  ShieldCheck,
  CheckCircle,
  AlertTriangle,
  Trash2,
  X,
  Info,
} from 'lucide-react';
import { Profile, SchedulerStatus, SystemInfo } from '../types';
import {
  MinecraftLauncherSelector,
  LauncherSelectionState,
} from './MinecraftLauncherSelector';

interface AutomaticBackupWizardProps {
  isOpen: boolean;
  onClose: () => void;
  profile: Profile | null;
  systemInfo: SystemInfo | null;
  schedulerStatus: SchedulerStatus | null;
  onInstallScheduler: (profile: Profile) => Promise<SchedulerStatus>;
  onUninstallScheduler: (profileId: string) => Promise<void>;
  installing: boolean;
  uninstalling: boolean;
}

export const AutomaticBackupWizard: React.FC<AutomaticBackupWizardProps> = ({
  isOpen,
  onClose,
  profile,
  systemInfo,
  schedulerStatus,
  onInstallScheduler,
  onUninstallScheduler,
  installing,
  uninstalling,
}) => {
  if (!isOpen) return null;

  const [step, setStep] = useState<number>(1);
  const [frequency, setFrequency] = useState<'hourly' | 'daily' | 'weekly'>('daily');
  const [hour, setHour] = useState<number>(4);
  const [minute, setMinute] = useState<number>(0);
  const [dayOfWeek, setDayOfWeek] = useState<number>(1); // Monday
  const [keepLast, setKeepLast] = useState<number>(profile?.keep_last ?? 10);
  const [tagTimestamp, setTagTimestamp] = useState<boolean>(profile?.tag_timestamp ?? true);
  const [launcherId, setLauncherId] = useState<string>('prism');
  const [launcherName, setLauncherName] = useState<string>('Prism Launcher');
  const [launcherScope, setLauncherScope] = useState<'entire' | 'instance'>('entire');
  const [selectedInstance, setSelectedInstance] = useState<string>('');
  const [customTagsInput, setCustomTagsInput] = useState<string>(
    profile?.tags
      ? profile.tags
          .filter((t) => !['minecraft', 'manual', 'automatic', 'pinned', 'important', 'scope:launcher', 'scope:instance'].includes(t) && !t.startsWith('launcher:') && !t.startsWith('instance:'))
          .join(', ')
      : ''
  );
  const [installSuccessMessage, setInstallSuccessMessage] = useState<string | null>(null);
  const [installError, setInstallError] = useState<string | null>(null);

  // Minecraft selection state for the profile
  const [resolvedSources, setResolvedSources] = useState<string[]>(
    profile?.source_paths && profile.source_paths.length > 0
      ? profile.source_paths
      : systemInfo?.prism_launcher_path
      ? [systemInfo.prism_launcher_path]
      : []
  );
  const [exclusions, setExclusions] = useState<string[]>(
    profile?.exclusions && profile.exclusions.length > 0
      ? profile.exclusions
      : ['*.lock', 'logs/', 'crash-reports/', 'cache/']
  );

  const platform = systemInfo?.platform || 'macos';
  const baseDir = systemInfo?.persistent_app_dir || '~/Library/Application Support/resticdashboard';
  const profileId = profile?.id || 'minecraft-auto-profile';

  // Computed persistent local paths that will be created
  const persistentRunnerPath = `${baseDir}/bin/resticdashboard-runner`;
  const persistentResticPath = `${baseDir}/bin/restic`;
  const persistentConfigPath = `${baseDir}/configs/${profileId}.json`;
  const persistentSecretPath = `${baseDir}/secrets/${profileId}.env`;

  const schedulerFilePath =
    platform === 'macos'
      ? `~/Library/LaunchAgents/com.resticdashboard.${profileId}.plist`
      : platform === 'linux'
      ? `~/.config/systemd/user/resticdashboard-${profileId}.timer`
      : `Task Scheduler: ResticDashboard_${profileId.replace(/-/g, '_')}`;

  const handleLauncherSelectionChange = (selection: LauncherSelectionState) => {
    if (selection.resolvedSourcePaths.length > 0) {
      setResolvedSources(selection.resolvedSourcePaths);
    }
    if (selection.recommendedExclusions.length > 0) {
      setExclusions(selection.recommendedExclusions);
    }
    if (selection.launcherId) {
      setLauncherId(selection.launcherId);
      setLauncherName(selection.launcherName);
    }
    setLauncherScope(selection.scope);
    setSelectedInstance(selection.selectedInstance);
  };

  const handleInstall = async () => {
    setInstallError(null);
    try {
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
          'automatic',
          launcherTag,
          launcherId,
          scopeTag,
          ...(instanceTag ? [instanceTag] : []),
          ...customTags,
        ])
      );

      const updatedProfile: Profile = {
        id: profile?.id || 'minecraft-auto-profile',
        name: profile?.name || 'Automatic Minecraft Backup',
        repo_id: profile?.repo_id || 'primary-repo',
        machine_id: systemInfo?.machine_id || 'local-host',
        machine_name: systemInfo?.machine_name || 'Host',
        source_paths: resolvedSources,
        exclusions,
        tags: allTags,
        keep_last: keepLast > 0 ? keepLast : undefined,
        tag_timestamp: tagTimestamp,
        schedule: {
          frequency,
          hour,
          minute,
          day_of_week: frequency === 'weekly' ? dayOfWeek : undefined,
          active: true,
        },
      };

      const res = await onInstallScheduler(updatedProfile);
      setInstallSuccessMessage(
        `Automatic background backup successfully registered! ${res.notes}`
      );
      setStep(5);
    } catch (e: any) {
      setInstallError(e?.toString() || 'Installation failed');
    }
  };

  const handleUninstall = async () => {
    const idToUninstall = profile?.id || 'minecraft-auto-profile';
    if (
      window.confirm(
        'Are you sure you want to uninstall the local automatic backup runner? This stops future background runs. Your existing snapshots and repository data will NOT be deleted.'
      )
    ) {
      await onUninstallScheduler(idToUninstall);
      onClose();
    }
  };

  return (
    <div className="modal-backdrop">
      <div className="modal-box modal-lg">
        <div className="modal-header">
          <div className="modal-header-title">
            <Cpu size={18} className="text-primary" />
            <h3>
              {schedulerStatus?.installed
                ? 'Local Automatic Backup Configuration'
                : 'Install Unattended Automatic Backups'}
            </h3>
          </div>
          <button onClick={onClose} className="close-btn">
            <X size={18} />
          </button>
        </div>

        <div className="modal-body">
          {/* If already installed, show status & manage controls */}
          {schedulerStatus?.installed && (
            <div className="installed-status-view">
              <div className="alert-banner alert-success">
                <CheckCircle size={18} />
                <div>
                  <strong>Local Unattended Runner is Active</strong>
                  <p>{schedulerStatus.notes}</p>
                </div>
              </div>

              <div className="scheduler-path-details">
                <h4>Installed Persistent Local Components (Continues Without USB):</h4>
                <ul>
                  <li>
                    <strong>Runner Executable:</strong> <code>{schedulerStatus.target_binary_path}</code>
                  </li>
                  <li>
                    <strong>Persistent Config:</strong> <code>{schedulerStatus.config_path}</code>
                  </li>
                  <li>
                    <strong>OS Schedule Entry:</strong> <code>{schedulerStatus.schedule_file_path}</code>
                  </li>
                </ul>

                <div className="safe-world-protection-notice mt-3">
                  <ShieldCheck size={16} className="text-emerald-400 flex-shrink-0 mt-0.5" />
                  <div>
                    <strong className="text-xs text-emerald-400">Live Game Protection Active:</strong>
                    <p className="text-[11px] text-muted mt-0.5 leading-relaxed">
                      The runner automatically checks that Minecraft and your launcher are closed before running. If you are actively playing, the backup is safely postponed until you exit your world.
                    </p>
                  </div>
                </div>
              </div>

              <div className="scheduler-danger-zone">
                <h4>Uninstall Scheduler</h4>
                <p>
                  Unregister the background task and remove persistent runner binaries. Existing repository backup data
                  will remain intact.
                </p>
                <button
                  type="button"
                  onClick={handleUninstall}
                  disabled={uninstalling}
                  className="btn-danger"
                >
                  <Trash2 size={14} />
                  <span>{uninstalling ? 'Uninstalling...' : 'Uninstall Local Scheduler'}</span>
                </button>
              </div>
            </div>
          )}

          {/* Wizard Steps (if not installed or re-configuring) */}
          {!schedulerStatus?.installed && (
            <div className="wizard-stepper-view">
              <div className="wizard-steps-header">
                <span className={`step-pill ${step >= 1 ? 'active' : ''}`}>1. Launcher</span>
                <span className={`step-pill ${step >= 2 ? 'active' : ''}`}>2. Frequency</span>
                <span className={`step-pill ${step >= 3 ? 'active' : ''}`}>3. Paths</span>
                <span className={`step-pill ${step >= 4 ? 'active' : ''}`}>4. Security</span>
              </div>

              {/* Step 1: Minecraft Launcher & Scope Selection */}
              {step === 1 && (
                <div className="wizard-step-content">
                  <MinecraftLauncherSelector
                    initialBasePath={resolvedSources[0]}
                    onSelectionChange={handleLauncherSelectionChange}
                    disabled={installing}
                  />
                </div>
              )}

              {/* Step 2: Frequency & Schedule */}
              {step === 2 && (
                <div className="wizard-step-content">
                  <h4>Select Backup Frequency ({launcherName})</h4>
                  <p className="text-muted">
                    Backups will run in the background using the native OS scheduler even when the USB is unplugged and
                    the dashboard GUI is closed.
                  </p>

                  <div className="frequency-options-grid">
                    <label className={`freq-card ${frequency === 'daily' ? 'selected' : ''}`}>
                      <input
                        type="radio"
                        name="freq"
                        checked={frequency === 'daily'}
                        onChange={() => setFrequency('daily')}
                      />
                      <strong>Daily (Recommended)</strong>
                      <span>Runs once per day at selected time</span>
                    </label>

                    <label className={`freq-card ${frequency === 'hourly' ? 'selected' : ''}`}>
                      <input
                        type="radio"
                        name="freq"
                        checked={frequency === 'hourly'}
                        onChange={() => setFrequency('hourly')}
                      />
                      <strong>Hourly</strong>
                      <span>Runs every 60 minutes</span>
                    </label>

                    <label className={`freq-card ${frequency === 'weekly' ? 'selected' : ''}`}>
                      <input
                        type="radio"
                        name="freq"
                        checked={frequency === 'weekly'}
                        onChange={() => setFrequency('weekly')}
                      />
                      <strong>Weekly</strong>
                      <span>Runs on a designated day each week</span>
                    </label>
                  </div>

                  {frequency !== 'hourly' && (
                    <div className="time-select-row">
                      <div className="form-group">
                        <label>Execution Time (24h format):</label>
                        <div className="time-inputs">
                          <input
                            type="number"
                            min="0"
                            max="23"
                            value={hour}
                            onChange={(e) => setHour(parseInt(e.target.value) || 0)}
                          />
                          <span>:</span>
                          <input
                            type="number"
                            min="0"
                            max="59"
                            value={minute}
                            onChange={(e) => setMinute(parseInt(e.target.value) || 0)}
                          />
                        </div>
                      </div>

                      {frequency === 'weekly' && (
                        <div className="form-group">
                          <label>Day of Week:</label>
                          <select
                            value={dayOfWeek}
                            onChange={(e) => setDayOfWeek(parseInt(e.target.value))}
                          >
                            <option value={1}>Monday</option>
                            <option value={2}>Tuesday</option>
                            <option value={3}>Wednesday</option>
                            <option value={4}>Thursday</option>
                            <option value={5}>Friday</option>
                            <option value={6}>Saturday</option>
                            <option value={0}>Sunday</option>
                          </select>
                        </div>
                      )}
                    </div>
                  )}

                  <div className="safe-world-protection-notice mt-4">
                    <ShieldCheck size={16} className="text-emerald-400 flex-shrink-0 mt-0.5" />
                    <div>
                      <strong className="text-xs text-emerald-400">Automatic Live World Protection:</strong>
                      <p className="text-[11px] text-muted mt-0.5 leading-relaxed">
                        The background task automatically verifies that your Minecraft launcher and game instances are closed first. If you are actively playing when a backup is scheduled, it is safely postponed so world saves are never corrupted.
                      </p>
                    </div>
                  </div>

                  {/* Retention Policy & Timestamping */}
                  <div className="retention-auto-card">
                    <div className="form-group">
                      <label><strong>Retention: How many backups to keep at once?</strong></label>
                      <select
                        value={keepLast}
                        onChange={(e) => setKeepLast(parseInt(e.target.value) || 0)}
                        className="retention-select"
                      >
                        <option value={5}>Keep last 5 backups (Auto-prune older)</option>
                        <option value={10}>Keep last 10 backups (Recommended)</option>
                        <option value={20}>Keep last 20 backups</option>
                        <option value={50}>Keep last 50 backups</option>
                        <option value={0}>Unlimited (Keep all backups)</option>
                      </select>
                      <span className="form-hint">
                        Important and Pinned snapshots are immune from retention deletion and will never be pruned.
                      </span>
                    </div>

                    {/* Custom Tags */}
                    <div className="form-group mt-3">
                      <label><strong>Custom Tags (comma-separated):</strong></label>
                      <input
                        type="text"
                        value={customTagsInput}
                        onChange={(e) => setCustomTagsInput(e.target.value)}
                        placeholder="e.g. daily, smp, auto, survival"
                        className="distinction-input"
                      />
                      <div className="flex items-center gap-1.5 mt-1 text-xs text-muted">
                        <span>Schedule tags:</span>
                        <span className="tag-pill text-[11px]">#minecraft</span>
                        <span className="tag-pill text-[11px]">#automatic</span>
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

                    <label className="checkbox-row mt-3">
                      <input
                        type="checkbox"
                        checked={tagTimestamp}
                        onChange={(e) => setTagTimestamp(e.target.checked)}
                      />
                      <span>Append execution timestamp tag in <code>MM-DD-YY</code> format (e.g. <code>#11-09-26_04-00</code>)</span>
                    </label>
                  </div>

                  <div className="os-scheduler-explanation">
                    <Info size={16} className="text-info flex-shrink-0" />
                    <div>
                      <strong>Scheduler Behavior on {platform.toUpperCase()}:</strong>
                      {platform === 'macos' ? (
                        <p>
                          Uses macOS <code>launchd</code> user agent. If your Mac is asleep at {hour}:{minute < 10 ? `0${minute}` : minute}, launchd will immediately execute the missed backup upon system wake. It does not wake a powered-off machine.
                        </p>
                      ) : platform === 'linux' ? (
                        <p>
                          Uses <code>systemd --user</code> timer with <code>Persistent=true</code>. Missed runs are queued and executed promptly after boot or user login.
                        </p>
                      ) : (
                        <p>
                          Uses Windows Task Scheduler. Missed runs will trigger as soon as the computer is turned on or awake.
                        </p>
                      )}
                    </div>
                  </div>
                </div>
              )}

              {/* Step 3: Local Path Inspection */}
              {step === 3 && (
                <div className="wizard-step-content">
                  <h4>Inspect Persistent Local Paths</h4>
                  <p className="text-muted">
                    To ensure automatic backups continue when the USB drive is detached, the following absolute local
                    files will be installed onto this computer:
                  </p>

                  <div className="paths-preview-box">
                    <div className="path-row">
                      <span className="path-tag">Runner Binary:</span>
                      <code>{persistentRunnerPath}</code>
                    </div>
                    <div className="path-row">
                      <span className="path-tag">Restic Engine:</span>
                      <code>{persistentResticPath}</code>
                    </div>
                    <div className="path-row">
                      <span className="path-tag">Profile Config:</span>
                      <code>{persistentConfigPath}</code>
                    </div>
                    <div className="path-row">
                      <span className="path-tag">Secret Credentials:</span>
                      <code>{persistentSecretPath}</code>
                    </div>
                    <div className="path-row">
                      <span className="path-tag">OS Scheduler Entry:</span>
                      <code>{schedulerFilePath}</code>
                    </div>
                    <div className="path-row">
                      <span className="path-tag">Selected Source:</span>
                      <code>{resolvedSources.join(', ') || 'No source selected'}</code>
                    </div>
                    <div className="path-row">
                      <span className="path-tag">Retention Policy:</span>
                      <code>{keepLast > 0 ? `Keep last ${keepLast} backups (prunes older non-pinned)` : 'Keep all backups (unlimited)'}</code>
                    </div>
                  </div>

                  <div className="independence-guarantee-box">
                    <CheckCircle size={16} className="text-success flex-shrink-0" />
                    <p>
                      <strong>Zero USB Path Dependencies:</strong> All scheduler triggers reference absolute paths on the local drive. You can safely unplug your USB drive after installation.
                    </p>
                  </div>
                </div>
              )}

              {/* Step 4: Security Disclosure & Install */}
              {step === 4 && (
                <div className="wizard-step-content">
                  <h4>Security & Credential Disclosure</h4>
                  <div className="security-disclosure-card">
                    <Shield size={20} className="text-primary flex-shrink-0" />
                    <div>
                      <strong>Unattended Credential Access:</strong>
                      <p>
                        For background execution to work without interactive prompts or GUI popups, repository
                        credentials are saved to a dedicated local secrets file at <code>{persistentSecretPath}</code>.
                      </p>
                      <p>
                        On Unix (macOS & Linux), this file is enforced with restrictive <code>0600</code> permissions (readable
                        only by your user account). On Windows, access is restricted to your user account profile.
                      </p>
                    </div>
                  </div>

                  {installError && (
                    <div className="alert-banner alert-error">
                      <AlertTriangle size={16} />
                      <span>{installError}</span>
                    </div>
                  )}
                </div>
              )}

              {/* Step 5: Completed */}
              {step === 5 && (
                <div className="wizard-step-content">
                  <div className="install-complete-banner">
                    <CheckCircle size={32} className="text-success" />
                    <h3>Installation Complete!</h3>
                    <p>{installSuccessMessage}</p>
                    <span>You can now safely disconnect the USB drive. Automatic backups will run according to your schedule.</span>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        <div className="modal-footer">
          {schedulerStatus?.installed ? (
            <button onClick={onClose} className="btn-secondary">
              Close
            </button>
          ) : step === 5 ? (
            <button onClick={onClose} className="btn-primary">
              Done
            </button>
          ) : (
            <>
              {step > 1 && (
                <button onClick={() => setStep(step - 1)} className="btn-secondary">
                  Back
                </button>
              )}
              {step < 4 ? (
                <button
                  onClick={() => setStep(step + 1)}
                  className="btn-primary"
                  disabled={step === 1 && resolvedSources.length === 0}
                >
                  Next
                </button>
              ) : (
                <button onClick={handleInstall} disabled={installing} className="btn-primary">
                  {installing ? 'Installing Local Runner...' : 'Confirm & Install Scheduler'}
                </button>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
};
