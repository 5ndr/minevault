import React, { useState } from 'react';
import { Gamepad2, Usb, Sun, Moon, Database, Laptop, CheckCircle2, Key, Download, RefreshCw } from 'lucide-react';
import { RepoConfig, DashboardSummary, SystemInfo, UsbVaultStatus } from '../types';

interface HeaderProps {
  theme: 'dark' | 'light';
  onToggleTheme: () => void;
  isDemoMode: boolean;
  onToggleDemoMode: () => void;
  repo: RepoConfig | null;
  onOpenRepoConfig: () => void;
  summary: DashboardSummary | null;
  systemInfo: SystemInfo | null;
  usbVaultStatus?: UsbVaultStatus | null;
  vaultUnlocked?: boolean;
  onOpenUsbVaultModal?: () => void;
  onDownloadRestic?: () => Promise<void>;
}

export const Header: React.FC<HeaderProps> = ({
  theme,
  onToggleTheme,
  isDemoMode,
  onToggleDemoMode,
  repo,
  onOpenRepoConfig,
  summary,
  systemInfo,
  usbVaultStatus,
  vaultUnlocked,
  onOpenUsbVaultModal,
  onDownloadRestic,
}) => {
  const [downloadingEngine, setDownloadingEngine] = useState<boolean>(false);

  const handleDownloadClick = async () => {
    if (!onDownloadRestic || downloadingEngine) return;
    setDownloadingEngine(true);
    try {
      await onDownloadRestic();
    } finally {
      setDownloadingEngine(false);
    }
  };
  return (
    <header className="header">
      <div className="header-left">
        <div className="logo-badge">
          <div className="vault-icon-container">
            <Gamepad2 className="logo-icon" size={24} />
          </div>
          <div className="logo-text">
            <h1>MineVault</h1>
            <span className="subtitle">Portable Minecraft Backup Manager & Safe Vault</span>
          </div>
        </div>

        <div className="badge-pill usb-badge" title="Running portably from USB. No background services run unless explicitly installed.">
          <Usb size={14} className="icon-pulse" />
          <span>USB Portable Vault</span>
        </div>

        {systemInfo && (
          systemInfo.restic_found ? (
            <div
              className={`badge-pill ${systemInfo.is_usb_bundled ? 'usb-badge' : 'verified-pill'}`}
              title={`Restic engine binary path: ${systemInfo.restic_path}`}
            >
              <CheckCircle2 size={13} />
              <span>{systemInfo.is_usb_bundled ? 'USB Bundled Engine' : 'Restic Engine Ready'}</span>
            </div>
          ) : (
            <button
              type="button"
              onClick={handleDownloadClick}
              disabled={downloadingEngine}
              className="badge-pill bg-amber-500/20 text-amber-300 border border-amber-500/30 hover:bg-amber-500/30 cursor-pointer flex items-center gap-1.5"
              title="No Homebrew or package manager required. Click to download and install native restic engine directly."
            >
              {downloadingEngine ? <RefreshCw size={13} className="animate-spin" /> : <Download size={13} />}
              <span>{downloadingEngine ? 'Installing Engine...' : 'Install Native Restic Engine'}</span>
            </button>
          )
        )}

        {usbVaultStatus?.exists && (
          <button
            type="button"
            onClick={onOpenUsbVaultModal}
            className={`badge-pill ${vaultUnlocked ? 'verified-pill' : 'usb-badge'} cursor-pointer`}
            title={
              vaultUnlocked
                ? 'USB Encrypted Key Vault is unlocked'
                : 'Encrypted USB Key Vault is locked. Click to enter passphrase.'
            }
          >
            <Key size={13} />
            <span>{vaultUnlocked ? 'USB Keys Decrypted' : 'Unlock USB Vault'}</span>
          </button>
        )}

        {isDemoMode && (
          <div className="badge-pill demo-badge" title="Viewing simulated demonstration data. No real repository accessed.">
            <span>DEMO FIXTURES ACTIVE</span>
          </div>
        )}
      </div>

      <div className="header-right">
        {summary && (
          <div className="machine-tag" title={`Machine ID: ${summary.current_machine_id}`}>
            <Laptop size={14} />
            <span>{summary.current_machine_name}</span>
          </div>
        )}

        <button
          onClick={onOpenRepoConfig}
          className={`repo-button ${repo ? 'connected' : 'unconfigured'}`}
        >
          <Database size={15} />
          <span>{repo ? repo.name : 'Configure Repository'}</span>
          {repo?.insecure_no_password && (
            <span className="pill-warn" title="Repository uses --insecure-no-password">
              Passwordless
            </span>
          )}
        </button>

        <button
          onClick={onToggleDemoMode}
          className={`demo-toggle-btn ${isDemoMode ? 'active' : ''}`}
          title="Toggle between live repository mode and labeled demonstration fixtures"
        >
          {isDemoMode ? 'Switch to Live Repo' : 'Load Demo Data'}
        </button>

        <button
          onClick={onToggleTheme}
          className="theme-toggle-btn"
          aria-label="Toggle dark/light theme"
          title={`Switch to ${theme === 'dark' ? 'Light' : 'Dark'} theme`}
        >
          {theme === 'dark' ? <Sun size={18} /> : <Moon size={18} />}
        </button>
      </div>
    </header>
  );
};
