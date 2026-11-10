import { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  LayoutDashboard,
  FolderOpen,
  Play,
  Cpu,
} from 'lucide-react';

import { Header } from './components/Header';
import { StatusCards } from './components/StatusCards';
import { StorageSection } from './components/StorageSection';
import { ActivityAlerts } from './components/ActivityAlerts';
import { SnapshotBrowser } from './components/SnapshotBrowser';
import { ManualBackupModal } from './components/ManualBackupModal';
import { AutomaticBackupWizard } from './components/AutomaticBackupWizard';
import { RepoConfigModal } from './components/RepoConfigModal';
import { FloatingBackupProgress } from './components/FloatingBackupProgress';
import { B2ManagementModal } from './components/B2ManagementModal';
import { UsbVaultModal } from './components/UsbVaultModal';
import { ErrorBoundary } from './components/ErrorBoundary';

import {
  Profile,
  RepoConfig,
  SessionCredentials,
  DashboardSummary,
  ResticSnapshot,
  ResticNode,
  FolderSizeBreakdown,
  StorageOverview,
  B2ProviderUsage,
  B2PurgeResult,
  UsbVaultStatus,
  EncryptedVaultData,
  RunResult,
  SystemInfo,
  SchedulerStatus,
  BackupProgressEvent,
  RestoreOptions,
} from './types';
import './App.css';

export function App() {
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');
  const [activeTab, setActiveTab] = useState<'dashboard' | 'browse'>('dashboard');
  const [isDemoMode, setIsDemoMode] = useState<boolean>(false);

  // System and OS Environment
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [localFreeSpace, setLocalFreeSpace] = useState<number | null>(null);

  const defaultRestoreDir = useMemo(() => {
    if (systemInfo?.platform === 'windows') {
      return 'C:\\MineVault_Restores';
    }
    if (systemInfo?.persistent_app_dir) {
      const parent = systemInfo.persistent_app_dir.replace(/[/\\][^/\\]+$/, '');
      return `${parent}/Restored_Backups`;
    }
    return '~/Restored_Backups';
  }, [systemInfo]);

  // Live User State (Never overwritten by Demo Mode)
  const [liveRepo, setLiveRepo] = useState<RepoConfig | null>(null);
  const [liveCreds, setLiveCreds] = useState<SessionCredentials>({
    password: '',
    b2_application_key: '',
    save_to_local_os: false,
  });
  const [liveProfiles, setLiveProfiles] = useState<Profile[]>([]);
  const [liveSelectedProfile, setLiveSelectedProfile] = useState<Profile | null>(null);

  // Demo State (Only active when isDemoMode is true)
  const [demoRepo, setDemoRepo] = useState<RepoConfig | null>(null);
  const [demoProfile, setDemoProfile] = useState<Profile | null>(null);

  // Active View Models
  const activeRepo = isDemoMode ? demoRepo : liveRepo;
  const activeProfile = isDemoMode ? demoProfile : liveSelectedProfile;

  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [snapshots, setSnapshots] = useState<ResticSnapshot[]>([]);
  const [selectedSnapshot, setSelectedSnapshot] = useState<ResticSnapshot | null>(null);
  const [nodes, setNodes] = useState<ResticNode[]>([]);
  const [loadingNodes, setLoadingNodes] = useState<boolean>(false);
  const [nodesError, setNodesError] = useState<string | null>(null);
  const [folderBreakdown, setFolderBreakdown] = useState<FolderSizeBreakdown[]>([]);
  const [loadingBreakdown, setLoadingBreakdown] = useState<boolean>(false);
  const [breakdownError, setBreakdownError] = useState<string | null>(null);
  const [storage, setStorage] = useState<StorageOverview | null>(null);
  const [refreshingStorage, setRefreshingStorage] = useState<boolean>(false);
  const [recentRuns, setRecentRuns] = useState<RunResult[]>([]);
  const [schedulerStatus, setSchedulerStatus] = useState<SchedulerStatus | null>(null);

  // Modals & Progress
  const [isRepoModalOpen, setIsRepoModalOpen] = useState<boolean>(false);
  const [isB2ModalOpen, setIsB2ModalOpen] = useState<boolean>(false);
  const [isUsbVaultModalOpen, setIsUsbVaultModalOpen] = useState<boolean>(false);
  const [usbVaultStatus, setUsbVaultStatus] = useState<UsbVaultStatus | null>(null);
  const [vaultUnlocked, setVaultUnlocked] = useState<boolean>(false);
  const [isManualBackupOpen, setIsManualBackupOpen] = useState<boolean>(false);
  const [isSchedulerWizardOpen, setIsSchedulerWizardOpen] = useState<boolean>(false);
  const [backupRunning, setBackupRunning] = useState<boolean>(false);
  const [backupProgress, setBackupProgress] = useState<BackupProgressEvent | null>(null);
  const [backupResult, setBackupResult] = useState<RunResult | null>(null);
  const [backupError, setBackupError] = useState<string | null>(null);

  // Restore State
  const [restoring, setRestoring] = useState<boolean>(false);
  const [restoreResult, setRestoreResult] = useState<string | null>(null);
  const [restoreError, setRestoreError] = useState<string | null>(null);
  const [checkingIntegrity, setCheckingIntegrity] = useState<boolean>(false);
  const [installingScheduler, setInstallingScheduler] = useState<boolean>(false);
  const [uninstallingScheduler, setUninstallingScheduler] = useState<boolean>(false);

  // Initialize theme
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
  }, [theme]);

  // Load system info and saved live profiles
  useEffect(() => {
    async function initApp() {
      try {
        const sys: SystemInfo = await invoke('get_system_info');
        setSystemInfo(sys);

        // Check restore destination free space
        const initialRestoreDir = sys.platform === 'windows'
          ? 'C:\\MineVault_Restores'
          : sys.persistent_app_dir
          ? `${sys.persistent_app_dir.replace(/[/\\][^/\\]+$/, '')}/Restored_Backups`
          : '~/Restored_Backups';
        try {
          const space: number = await invoke('get_free_disk_space_cmd', { targetDir: initialRestoreDir });
          setLocalFreeSpace(space);
        } catch (_) {}

        // Load saved live repos
        const savedRepos: RepoConfig[] = await invoke('load_repo_configs_cmd');
        if (savedRepos.length > 0) {
          setLiveRepo(savedRepos[0]);
        }

        // Load saved live profiles
        const savedProfiles: Profile[] = await invoke('load_profiles_cmd');
        setLiveProfiles(savedProfiles);
        if (savedProfiles.length > 0) {
          setLiveSelectedProfile(savedProfiles[0]);
        }

        // Check for encrypted USB key vault
        try {
          const vStatus: UsbVaultStatus = await invoke('check_usb_vault_status_cmd');
          setUsbVaultStatus(vStatus);
          if (vStatus.exists) {
            if (!vStatus.is_passphrase_protected) {
              // Auto-unlock with internal device key
              const unlocked: EncryptedVaultData = await invoke('unlock_usb_vault_cmd', { passphrase: null });
              setLiveCreds((prev) => ({
                ...prev,
                b2_application_key: unlocked.b2_application_key || prev.b2_application_key,
                b2_management_app_key: unlocked.b2_management_app_key || prev.b2_management_app_key,
                password: unlocked.repo_password || prev.password,
              }));
              setVaultUnlocked(true);
            } else {
              // Passphrase required - prompt modal
              setIsUsbVaultModalOpen(true);
            }
          }
        } catch (_) {}
      } catch (err) {
        console.error('Initialization error:', err);
      }
    }
    initApp();
  }, []);

  // Listen for live backup progress events from Rust
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<BackupProgressEvent>('backup-progress', (event) => {
      setBackupProgress(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // Refresh dashboard whenever live state or demoMode toggles
  useEffect(() => {
    if (isDemoMode) {
      loadDemoData();
    } else {
      refreshLiveDashboard();
    }
  }, [isDemoMode, liveRepo, liveCreds, liveSelectedProfile]);

  // Load live data from real repository
  async function refreshLiveDashboard() {
    // If no live repository is configured, show clear setup prompt
    if (!liveRepo) {
      setSnapshots([]);
      setStorage(null);
      setFolderBreakdown([]);
      setNodes([]);
      setSelectedSnapshot(null);
      setSummary({
        connection_status: 'offline',
        automatic_backups_installed: false,
        total_retained_snapshots: 0,
        missing_sources: [],
        alerts: [
          {
            level: 'info',
            title: 'Repository Not Configured',
            message: 'Click "Configure Repository" to connect your Backblaze B2 repository (s3:https://s3.<region>.backblazeb2.com/<bucket>/<path>) or switch to "Load Demo Data" to test the UI.',
            code: 'REPO_UNCONFIGURED',
          },
        ],
        current_machine_id: systemInfo?.machine_id || 'macbook',
        current_machine_name: systemInfo?.machine_name || 'Host',
      });
      return;
    }

    try {
      const sum: DashboardSummary = await invoke('get_dashboard_summary', {
        repo: liveRepo,
        creds: liveCreds,
        profile: liveSelectedProfile,
      });
      setSummary(sum);

      if (sum.scheduler_status) {
        setSchedulerStatus(sum.scheduler_status);
      }

      if (sum.connection_status === 'connected') {
        const snapList: ResticSnapshot[] = await invoke('list_snapshots', {
          repo: liveRepo,
          creds: liveCreds,
          hostFilter: null,
          pathFilter: null,
        });
        setSnapshots(snapList);
        if (snapList.length > 0) {
          const current = selectedSnapshot && snapList.some((s) => s.id === selectedSnapshot.id)
            ? selectedSnapshot
            : snapList[0];
          setSelectedSnapshot(current);
          loadSnapshotDetails(current.id, liveRepo, liveCreds);
        } else {
          setSelectedSnapshot(null);
          setFolderBreakdown([]);
          setNodes([]);
        }

        // Load storage overview
        try {
          const st: StorageOverview = await invoke('get_storage_overview', {
            repo: liveRepo,
            creds: liveCreds,
          });
          setStorage(st);
        } catch (_) {}
      }
    } catch (err) {
      console.error('Error refreshing live dashboard:', err);
    }
  }

  // Load Demo Data fixtures without modifying live state
  async function loadDemoData() {
    try {
      const demo: any = await invoke('get_demo_data');
      setDemoRepo(demo.repo);
      setDemoProfile(demo.profile);
      setSnapshots(demo.snapshots);
      setStorage(demo.storage_overview);
      setFolderBreakdown(demo.folder_breakdown);
      setNodes(demo.nodes);
      setRecentRuns(demo.runs);
      if (demo.snapshots.length > 0) {
        setSelectedSnapshot(demo.snapshots[0]);
      }
      setSummary({
        connection_status: 'connected',
        automatic_backups_installed: true,
        scheduler_status: {
          platform: 'macos',
          installed: true,
          enabled: true,
          target_binary_path: '~/.minevault/bin/resticdashboard-runner',
          config_path: '~/.minevault/configs/prism-demo-profile.json',
          schedule_file_path: '~/Library/LaunchAgents/com.minevault.prism-demo-profile.plist',
          notes: 'Active launchd user agent with local persistent runner.',
          last_run: demo.runs[0],
        },
        total_retained_snapshots: demo.snapshots.length,
        oldest_restore_point: demo.snapshots[demo.snapshots.length - 1]?.time,
        last_successful_backup: demo.runs[0],
        last_attempted_backup: demo.runs[0],
        next_scheduled_run: '04:00 (Daily)',
        missing_sources: [],
        alerts: [
          {
            level: 'info',
            title: 'Passwordless Repository in Demo Mode',
            message: 'Repository uses --insecure-no-password flag. Access does not require a passphrase.',
            code: 'PASSWORDLESS_INFO',
          },
        ],
        current_machine_id: 'demo-host',
        current_machine_name: 'Demo System',
      });
    } catch (err) {
      console.error('Error loading demo data:', err);
    }
  }

  // Load directory breakdown and nodes for selected snapshot
  async function loadSnapshotDetails(
    snapshotId: string,
    targetRepo?: RepoConfig | null,
    targetCreds?: SessionCredentials
  ) {
    if (isDemoMode) return;
    const r = targetRepo || liveRepo;
    const c = targetCreds || liveCreds;
    if (!r) return;

    setLoadingBreakdown(true);
    setBreakdownError(null);
    setLoadingNodes(true);
    setNodesError(null);

    const breakdownPromise = invoke<FolderSizeBreakdown[]>('get_snapshot_folder_breakdown', {
      repo: r,
      creds: c,
      snapshotId,
    })
      .then((breakdown) => {
        setFolderBreakdown(breakdown);
      })
      .catch((e: any) => {
        console.error('Folder breakdown error:', e);
        setBreakdownError(e?.toString() || 'Failed to analyze snapshot breakdown');
        setFolderBreakdown([]);
      })
      .finally(() => {
        setLoadingBreakdown(false);
      });

    const nodesPromise = invoke<ResticNode[]>('list_snapshot_nodes', {
      repo: r,
      creds: c,
      snapshotId,
      dirFilter: null,
    })
      .then((nodeList) => {
        setNodes(nodeList);
      })
      .catch((e: any) => {
        console.error('Node list error:', e);
        setNodesError(e?.toString() || 'Failed to load snapshot file tree');
        setNodes([]);
      })
      .finally(() => {
        setLoadingNodes(false);
      });

    await Promise.allSettled([breakdownPromise, nodesPromise]);
  }

  const handleSelectSnapshot = (snap: ResticSnapshot) => {
    setSelectedSnapshot(snap);
    if (!isDemoMode && liveRepo) {
      loadSnapshotDetails(snap.id, liveRepo, liveCreds);
    }
  };

  const handleRefreshStorage = async () => {
    if (!liveRepo || isDemoMode) return;
    setRefreshingStorage(true);
    try {
      const st: StorageOverview = await invoke('get_storage_overview', {
        repo: liveRepo,
        creds: liveCreds,
      });
      setStorage(st);
    } catch (e: any) {
      alert(`Stats calculation error: ${e?.toString()}`);
    } finally {
      setRefreshingStorage(false);
    }
  };

  const handleMeasureB2Usage = async (
    customKeyId?: string,
    customAppKey?: string
  ): Promise<B2ProviderUsage> => {
    if (!liveRepo) throw new Error('Repository not configured');
    const targetRepo = {
      ...liveRepo,
      b2_management_key_id: customKeyId || liveRepo.b2_management_key_id,
    };
    const targetCreds = {
      ...liveCreds,
      b2_management_app_key: customAppKey || liveCreds.b2_management_app_key,
    };

    const usage: B2ProviderUsage = await invoke('fetch_b2_provider_usage', {
      repo: targetRepo,
      creds: targetCreds,
    });

    setLiveRepo(targetRepo);
    setLiveCreds(targetCreds);
    setStorage((prev) =>
      prev
        ? {
            ...prev,
            provider_b2_usage: usage.total_bytes,
            b2_provider_details: usage,
          }
        : null
    );
    return usage;
  };

  const handlePurgeB2Versions = async (): Promise<B2PurgeResult> => {
    if (!liveRepo) throw new Error('Repository not configured');
    const result: B2PurgeResult = await invoke('purge_b2_hidden_versions_cmd', {
      repo: liveRepo,
      creds: liveCreds,
    });
    setStorage((prev) =>
      prev
        ? {
            ...prev,
            provider_b2_usage: result.updated_usage.total_bytes,
            b2_provider_details: result.updated_usage,
          }
        : null
    );
    return result;
  };

  const handleDownloadRestic = async () => {
    try {
      const path: string = await invoke('download_restic_engine_cmd');
      const sys: SystemInfo = await invoke('get_system_info');
      setSystemInfo(sys);
      alert(`Successfully installed native standalone restic engine to:\n${path}\nNo Homebrew or package manager required!`);
    } catch (e: any) {
      alert(`Failed to install restic engine: ${e?.toString()}`);
    }
  };

  const handleUnlockVault = async (passphrase: string) => {
    const unlocked: EncryptedVaultData = await invoke('unlock_usb_vault_cmd', { passphrase });
    setLiveCreds((prev) => ({
      ...prev,
      b2_application_key: unlocked.b2_application_key || prev.b2_application_key,
      b2_management_app_key: unlocked.b2_management_app_key || prev.b2_management_app_key,
      password: unlocked.repo_password || prev.password,
    }));
    if (unlocked.b2_key_id && liveRepo && !liveRepo.b2_key_id) {
      setLiveRepo({ ...liveRepo, b2_key_id: unlocked.b2_key_id });
    }
    setVaultUnlocked(true);
    refreshLiveDashboard();
  };

  const handleClearVault = async () => {
    await invoke('clear_usb_vault_cmd');
    setUsbVaultStatus(null);
    setVaultUnlocked(false);
  };

  const handleTestConnection = async (
    testRepo: RepoConfig,
    testCreds: SessionCredentials
  ): Promise<string> => {
    return await invoke('test_repo_connection', { repo: testRepo, creds: testCreds });
  };

  const handleSaveRepo = async (newRepo: RepoConfig, newCreds: SessionCredentials) => {
    setLiveRepo(newRepo);
    setLiveCreds(newCreds);
    await invoke('save_repo_config_cmd', { repo: newRepo });
    // Switch to live mode to immediately view the saved repository
    setIsDemoMode(false);
  };

  const handleForgetCredentials = () => {
    setLiveCreds({ password: '', b2_application_key: '', save_to_local_os: false });
    alert('Session credentials cleared from memory.');
  };

  const handleStartManualBackup = async (
    profileToRun: Profile,
    force?: boolean
  ): Promise<RunResult | null> => {
    if (!liveRepo) {
      alert('Please configure repository first.');
      return null;
    }
    setBackupRunning(true);
    setBackupProgress(null);
    setBackupResult(null);
    setBackupError(null);

    try {
      const result: RunResult = await invoke('start_manual_backup', {
        repo: liveRepo,
        profile: profileToRun,
        creds: liveCreds,
        force: force || false,
      });
      setBackupResult(result);
      setRecentRuns((prev) => [result, ...prev]);
      refreshLiveDashboard();
      return result;
    } catch (err: any) {
      setBackupError(err?.toString() || 'Backup failed');
      return null;
    } finally {
      setBackupRunning(false);
    }
  };

  const handleCancelBackup = async () => {
    try {
      await invoke('cancel_manual_backup');
    } catch (e) {
      console.error('Cancel error:', e);
    }
  };

  const handleRestore = async (options: RestoreOptions) => {
    if (!liveRepo) return;
    setRestoring(true);
    setRestoreResult(null);
    setRestoreError(null);
    try {
      const res: string = await invoke('restore_files', {
        repo: liveRepo,
        creds: liveCreds,
        options,
      });
      setRestoreResult(res);
    } catch (e: any) {
      setRestoreError(e?.toString() || 'Restore failed');
    } finally {
      setRestoring(false);
    }
  };

  const handleRunIntegrityCheck = async () => {
    if (!liveRepo || isDemoMode) {
      alert('Please configure a live repository to run structural check.');
      return;
    }
    setCheckingIntegrity(true);
    try {
      const chk: any = await invoke('run_integrity_check_cmd', {
        repo: liveRepo,
        creds: liveCreds,
        readData: false,
      });
      alert(
        chk.success
          ? 'Integrity Check Passed! Repository structure is consistent.'
          : `Check failed: ${chk.details}`
      );
      refreshLiveDashboard();
    } catch (e: any) {
      alert(`Check error: ${e?.toString()}`);
    } finally {
      setCheckingIntegrity(false);
    }
  };

  const handleRecordRestoreTest = async (snapshotId: string, dest: string) => {
    try {
      await invoke('record_restore_test', {
        record: {
          timestamp: new Date().toISOString(),
          snapshot_id: snapshotId,
          destination: dest,
          success: true,
          notes: 'User verified manual drill recovery',
        },
      });
      alert('Restore drill recorded as verified in dashboard.');
      if (!isDemoMode) {
        refreshLiveDashboard();
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleTogglePin = async (snapshotId: string, pinned: boolean) => {
    if (!liveRepo) return;
    try {
      await invoke('toggle_snapshot_pinned', {
        repo: liveRepo,
        creds: liveCreds,
        snapshotId,
        pinned,
      });
      refreshLiveDashboard();
    } catch (err: any) {
      alert(`Pinning error: ${err?.toString()}`);
    }
  };

  const handleRunForget = async (keepLast: number) => {
    if (!liveRepo) return;
    try {
      await invoke('run_forget_now', {
        repo: liveRepo,
        creds: liveCreds,
        keepLast,
      });
      refreshLiveDashboard();
    } catch (err: any) {
      alert(`Prune error: ${err?.toString()}`);
      throw err;
    }
  };

  const handleDeleteSnapshot = async (snapshotId: string, prune: boolean) => {
    if (isDemoMode) {
      setSnapshots((prev) => prev.filter((s) => s.id !== snapshotId));
      if (selectedSnapshot?.id === snapshotId) {
        setSelectedSnapshot(null);
        setNodes([]);
        setFolderBreakdown([]);
      }
      return;
    }
    if (!liveRepo) return;
    try {
      await invoke('delete_snapshot', {
        repo: liveRepo,
        creds: liveCreds,
        snapshotId,
        prune,
      });
      if (selectedSnapshot?.id === snapshotId) {
        setSelectedSnapshot(null);
        setNodes([]);
        setFolderBreakdown([]);
      }
      await refreshLiveDashboard();
    } catch (err: any) {
      alert(`Failed to delete snapshot: ${err?.toString()}`);
      throw err;
    }
  };

  const handleInstallScheduler = async (p: Profile): Promise<SchedulerStatus> => {
    if (!liveRepo) throw new Error('Repository configuration missing. Please configure repository first.');
    setInstallingScheduler(true);
    try {
      const st: SchedulerStatus = await invoke('install_scheduler', {
        profile: p,
        repo: liveRepo,
        creds: liveCreds,
      });
      setSchedulerStatus(st);
      refreshLiveDashboard();
      return st;
    } finally {
      setInstallingScheduler(false);
    }
  };

  const handleUninstallScheduler = async (profileId: string) => {
    setUninstallingScheduler(true);
    try {
      await invoke('uninstall_scheduler', { profileId });
      setSchedulerStatus(null);
      refreshLiveDashboard();
      alert('Local automatic scheduler uninstalled. Backup data remains intact.');
    } finally {
      setUninstallingScheduler(false);
    }
  };

  return (
    <div className="app-container">
      <Header
        theme={theme}
        onToggleTheme={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
        isDemoMode={isDemoMode}
        onToggleDemoMode={() => setIsDemoMode(!isDemoMode)}
        repo={activeRepo}
        onOpenRepoConfig={() => setIsRepoModalOpen(true)}
        summary={summary}
        systemInfo={systemInfo}
        usbVaultStatus={usbVaultStatus}
        vaultUnlocked={vaultUnlocked}
        onOpenUsbVaultModal={() => setIsUsbVaultModalOpen(true)}
        onDownloadRestic={handleDownloadRestic}
      />

      {/* Main Navigation Bar */}
      <nav className="main-nav">
        <div className="nav-tabs">
          <button
            className={`nav-tab ${activeTab === 'dashboard' ? 'active' : ''}`}
            onClick={() => setActiveTab('dashboard')}
          >
            <LayoutDashboard size={16} />
            <span>Overview & Storage</span>
          </button>

          <button
            className={`nav-tab ${activeTab === 'browse' ? 'active' : ''}`}
            onClick={() => setActiveTab('browse')}
          >
            <FolderOpen size={16} />
            <span>Browse Backups & Restore</span>
          </button>
        </div>

        <div className="nav-quick-actions">
          <button onClick={() => setIsManualBackupOpen(true)} className="btn-primary">
            <Play size={14} />
            <span>Run Manual Backup</span>
          </button>

          <button onClick={() => setIsSchedulerWizardOpen(true)} className="btn-secondary">
            <Cpu size={14} />
            <span>
              {schedulerStatus?.installed ? 'Scheduler (Installed)' : 'Install Auto-Backup'}
            </span>
          </button>
        </div>
      </nav>

      {/* Main Content View */}
      <main className="main-content">
        {activeTab === 'dashboard' ? (
          <>
            <StatusCards
              summary={summary}
              selectedProfile={activeProfile}
              onOpenSchedulerWizard={() => setIsSchedulerWizardOpen(true)}
              onRunIntegrityCheck={handleRunIntegrityCheck}
              onRecordRestoreTest={() => {
                if (selectedSnapshot) {
                  handleRecordRestoreTest(selectedSnapshot.id, defaultRestoreDir);
                } else {
                  alert('Select a snapshot to record a confirmed test.');
                }
              }}
              checkingIntegrity={checkingIntegrity}
            />

            <StorageSection
              storage={storage}
              snapshots={snapshots}
              selectedSnapshotId={selectedSnapshot?.id || null}
              onSelectSnapshot={(id) => {
                const found = snapshots.find((s) => s.id === id);
                if (found) handleSelectSnapshot(found);
              }}
              folderBreakdown={folderBreakdown}
              loadingBreakdown={loadingBreakdown}
              breakdownError={breakdownError}
              onRetryBreakdown={() => {
                if (selectedSnapshot) {
                  loadSnapshotDetails(selectedSnapshot.id, liveRepo, liveCreds);
                }
              }}
              onRefreshStorage={handleRefreshStorage}
              refreshing={refreshingStorage}
              localFreeSpace={localFreeSpace}
              restoreTargetDir={defaultRestoreDir}
              onOpenB2ManagementModal={() => setIsB2ModalOpen(true)}
            />

            <ActivityAlerts alerts={summary?.alerts || []} recentRuns={recentRuns} />
          </>
        ) : (
          <ErrorBoundary fallbackTitle="Snapshot Explorer Error">
            <SnapshotBrowser
              snapshots={snapshots}
              selectedSnapshot={selectedSnapshot}
              onSelectSnapshot={handleSelectSnapshot}
              nodes={nodes}
              loadingNodes={loadingNodes}
              nodesError={nodesError}
              onRetryNodes={() => {
                if (selectedSnapshot) {
                  loadSnapshotDetails(selectedSnapshot.id, liveRepo, liveCreds);
                }
              }}
              onRestore={handleRestore}
              restoring={restoring}
              restoreResult={restoreResult}
              restoreError={restoreError}
              onRecordRestoreTest={handleRecordRestoreTest}
              onTogglePin={handleTogglePin}
              onRunForget={handleRunForget}
              onDeleteSnapshot={handleDeleteSnapshot}
              defaultRestoreBaseDir={defaultRestoreDir}
            />
          </ErrorBoundary>
        )}
      </main>

      {/* Modals */}
      <ManualBackupModal
        isOpen={isManualBackupOpen}
        onClose={() => setIsManualBackupOpen(false)}
        onMinimize={() => setIsManualBackupOpen(false)}
        profiles={liveProfiles}
        selectedProfile={activeProfile}
        onSelectProfile={setLiveSelectedProfile}
        systemInfo={systemInfo}
        onStartBackup={handleStartManualBackup}
        onCancelBackup={handleCancelBackup}
        isRunning={backupRunning}
        progress={backupProgress}
        backupResult={backupResult}
        backupError={backupError}
      />

      <AutomaticBackupWizard
        isOpen={isSchedulerWizardOpen}
        onClose={() => setIsSchedulerWizardOpen(false)}
        profile={activeProfile}
        systemInfo={systemInfo}
        schedulerStatus={schedulerStatus}
        onInstallScheduler={handleInstallScheduler}
        onUninstallScheduler={handleUninstallScheduler}
        installing={installingScheduler}
        uninstalling={uninstallingScheduler}
      />

      <RepoConfigModal
        isOpen={isRepoModalOpen}
        onClose={() => setIsRepoModalOpen(false)}
        repo={liveRepo}
        creds={liveCreds}
        onSaveRepo={handleSaveRepo}
        onTestConnection={handleTestConnection}
        onForgetCredentials={handleForgetCredentials}
      />

      <B2ManagementModal
        isOpen={isB2ModalOpen}
        onClose={() => setIsB2ModalOpen(false)}
        repo={activeRepo}
        creds={liveCreds}
        onMeasure={handleMeasureB2Usage}
        onPurgeVersions={handlePurgeB2Versions}
        currentUsage={storage?.b2_provider_details}
      />

      <UsbVaultModal
        isOpen={isUsbVaultModalOpen}
        onClose={() => setIsUsbVaultModalOpen(false)}
        vaultStatus={usbVaultStatus}
        onUnlock={handleUnlockVault}
        onClearVault={handleClearVault}
      />

      {/* Floating Backup Progress & Estimated Time Monitor */}
      <FloatingBackupProgress
        isRunning={backupRunning}
        progress={backupProgress}
        backupResult={!isManualBackupOpen ? backupResult : null}
        backupError={!isManualBackupOpen ? backupError : null}
        activeProfileName={activeProfile?.distinction_label || activeProfile?.name || 'Minecraft Backup'}
        onExpand={() => setIsManualBackupOpen(true)}
        onCancel={handleCancelBackup}
        onDismissResult={() => {
          setBackupResult(null);
          setBackupError(null);
        }}
      />
    </div>
  );
}

export default App;
