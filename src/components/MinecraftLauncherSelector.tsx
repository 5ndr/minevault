import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Gamepad2,
  Folder,
  FolderOpen,
  CheckCircle,
  AlertTriangle,
  Layers,
} from 'lucide-react';
import { LauncherDescriptor } from '../types';

export interface LauncherSelectionState {
  launcherId: string;
  launcherName: string;
  basePath: string;
  scope: 'entire' | 'instance';
  selectedInstance: string;
  resolvedSourcePaths: string[];
  recommendedExclusions: string[];
}

interface MinecraftLauncherSelectorProps {
  initialBasePath?: string;
  onSelectionChange: (selection: LauncherSelectionState) => void;
  disabled?: boolean;
}

export const MinecraftLauncherSelector: React.FC<MinecraftLauncherSelectorProps> = ({
  initialBasePath,
  onSelectionChange,
  disabled = false,
}) => {
  const [launchers, setLaunchers] = useState<LauncherDescriptor[]>([]);
  const [selectedLauncherId, setSelectedLauncherId] = useState<string>('prism');
  const [customPath, setCustomPath] = useState<string>('');
  const [scope, setScope] = useState<'entire' | 'instance'>('entire');
  const [selectedInstance, setSelectedInstance] = useState<string>('');
  const [instances, setInstances] = useState<string[]>([]);
  const [scanningInstances, setScanningInstances] = useState<boolean>(false);

  // Fetch supported and detected launchers on mount
  useEffect(() => {
    async function loadLaunchers() {
      try {
        const detected: LauncherDescriptor[] = await invoke('get_supported_launchers');
        setLaunchers(detected);

        // Choose Prism Launcher if detected or present, otherwise first detected launcher
        const prism = detected.find((l) => l.id === 'prism');
        const firstDetected = detected.find((l) => l.detected);
        const defaultChoice = prism?.detected ? prism : (firstDetected || prism || detected[0]);

        if (defaultChoice) {
          setSelectedLauncherId(defaultChoice.id);
          const activePath = initialBasePath || defaultChoice.default_path;
          setCustomPath(activePath);
          setInstances(defaultChoice.instances);
          if (defaultChoice.instances.length > 0) {
            setSelectedInstance(defaultChoice.instances[0]);
          }
        }
      } catch (err) {
        console.error('Failed to load launchers:', err);
      }
    }
    loadLaunchers();
  }, []);

  // Update instances when customPath changes
  async function refreshInstancesForPath(pathStr: string) {
    if (!pathStr.trim()) {
      setInstances([]);
      return;
    }
    setScanningInstances(true);
    try {
      const found: string[] = await invoke('scan_launcher_instances_cmd', {
        launcherPath: pathStr.trim(),
      });
      setInstances(found);
      if (found.length > 0 && !found.includes(selectedInstance)) {
        setSelectedInstance(found[0]);
      }
    } catch (_) {
      setInstances([]);
    } finally {
      setScanningInstances(false);
    }
  }

  // Handle switching launcher tab/button
  const handleSelectLauncher = (l: LauncherDescriptor) => {
    setSelectedLauncherId(l.id);
    setCustomPath(l.default_path);
    setInstances(l.instances);
    if (l.instances.length > 0) {
      setSelectedInstance(l.instances[0]);
    } else {
      setSelectedInstance('');
    }
  };

  const handleCustomLauncherSelect = () => {
    setSelectedLauncherId('custom');
    if (!customPath) {
      setCustomPath('/Users/declan/Library/Application Support/PrismLauncher');
    }
    refreshInstancesForPath(customPath);
  };

  // Open native system folder picker
  const handlePickFolder = async () => {
    try {
      const picked: string | null = await invoke('pick_folder');
      if (picked) {
        setCustomPath(picked);
        setSelectedLauncherId('custom');
        await refreshInstancesForPath(picked);
      }
    } catch (err) {
      console.error('Folder picker error:', err);
    }
  };

  // Compute resolved source paths and emit change
  useEffect(() => {
    const currentLauncher = launchers.find((l) => l.id === selectedLauncherId);
    const launcherName = currentLauncher ? currentLauncher.name : 'Custom Minecraft Location';
    const basePath = customPath.trim();

    let resolvedSourcePaths: string[] = [];
    if (basePath) {
      if (scope === 'instance' && selectedInstance) {
        // Build instance specific path (instances/<name> or saves/<world>)
        const sep = basePath.includes('\\') ? '\\' : '/';
        resolvedSourcePaths = [`${basePath}${sep}instances${sep}${selectedInstance}`];
      } else {
        resolvedSourcePaths = [basePath];
      }
    }

    const recommendedExclusions = [
      '*.lock',
      'logs/',
      'crash-reports/',
      '.cache/',
      'cache/',
      'metacache/',
      'webcache/',
    ];

    onSelectionChange({
      launcherId: selectedLauncherId,
      launcherName,
      basePath,
      scope,
      selectedInstance,
      resolvedSourcePaths,
      recommendedExclusions,
    });
  }, [selectedLauncherId, customPath, scope, selectedInstance, launchers]);

  return (
    <div className="minecraft-launcher-selector">
      <div className="selector-section-title">
        <Gamepad2 size={18} className="text-primary" />
        <h4>1. Which Minecraft launcher are you using?</h4>
      </div>
      <p className="section-instruction">
        MineVault auto-detects common launcher installation paths across macOS, Windows, and Linux.
      </p>

      {/* Launcher Choice Grid */}
      <div className="launchers-grid">
        {launchers.map((l) => (
          <button
            key={l.id}
            type="button"
            disabled={disabled}
            className={`launcher-card-btn ${selectedLauncherId === l.id ? 'active' : ''}`}
            onClick={() => handleSelectLauncher(l)}
          >
            <div className="launcher-card-top">
              <span className="launcher-title">{l.name}</span>
              {l.detected && (
                <span className="pill-detected" title="Detected on this computer">
                  <CheckCircle size={12} /> Detected
                </span>
              )}
            </div>
            <div className="launcher-card-instances">
              {l.instances.length > 0
                ? `${l.instances.length} instance${l.instances.length > 1 ? 's' : ''} found`
                : l.detected
                ? 'Installed'
                : 'Not found'}
            </div>
          </button>
        ))}

        <button
          type="button"
          disabled={disabled}
          className={`launcher-card-btn ${selectedLauncherId === 'custom' ? 'active' : ''}`}
          onClick={handleCustomLauncherSelect}
        >
          <div className="launcher-card-top">
            <span className="launcher-title">Custom / Server</span>
            <Folder size={14} />
          </div>
          <div className="launcher-card-instances">Custom folder / server</div>
        </button>
      </div>

      {/* Path Inspector & System Picker Fallback */}
      <div className="launcher-path-box">
        <div className="path-box-header">
          <label>Launcher / Minecraft Directory Path:</label>
          <button
            type="button"
            onClick={handlePickFolder}
            disabled={disabled}
            className="btn-browse-folder"
            title="Open native OS system folder dialog to choose Minecraft folder"
          >
            <FolderOpen size={14} />
            <span>Browse Folder...</span>
          </button>
        </div>

        <input
          type="text"
          value={customPath}
          disabled={disabled}
          onChange={(e) => {
            setCustomPath(e.target.value);
            refreshInstancesForPath(e.target.value);
          }}
          placeholder="/Users/.../Library/Application Support/PrismLauncher"
          className="launcher-path-input"
        />
        <span className="form-hint">
          Path doesn't look right or using a custom location? Click <strong>Browse Folder...</strong> to select it with your system file chooser.
        </span>
      </div>

      {/* Backup Scope: Entire Launcher vs Specific Instance */}
      <div className="scope-selection-card">
        <div className="scope-header">
          <Layers size={16} className="text-primary" />
          <strong>2. What do you want to back up?</strong>
        </div>

        <div className="scope-options-row">
          <label className={`scope-radio-label ${scope === 'entire' ? 'selected' : ''}`}>
            <input
              type="radio"
              name="backup_scope"
              checked={scope === 'entire'}
              disabled={disabled}
              onChange={() => setScope('entire')}
            />
            <div>
              <strong>Entire Launcher</strong>
              <span className="scope-desc">All instances, mods, settings, resource packs, and worlds</span>
            </div>
          </label>

          <label className={`scope-radio-label ${scope === 'instance' ? 'selected' : ''}`}>
            <input
              type="radio"
              name="backup_scope"
              checked={scope === 'instance'}
              disabled={disabled}
              onChange={() => setScope('instance')}
            />
            <div>
              <strong>Specific Instance / World</strong>
              <span className="scope-desc">Only back up a single selected instance or modpack</span>
            </div>
          </label>
        </div>

        {/* If Specific Instance is selected, show instance picker */}
        {scope === 'instance' && (
          <div className="instance-selector-group">
            <label>Select Instance to Back Up:</label>
            {scanningInstances ? (
              <div className="text-muted text-sm">Scanning directory for instances...</div>
            ) : instances.length > 0 ? (
              <select
                value={selectedInstance}
                disabled={disabled}
                onChange={(e) => setSelectedInstance(e.target.value)}
                className="instance-dropdown"
              >
                {instances.map((inst) => (
                  <option key={inst} value={inst}>
                    {inst}
                  </option>
                ))}
              </select>
            ) : (
              <div className="instance-input-row">
                <input
                  type="text"
                  value={selectedInstance}
                  disabled={disabled}
                  onChange={(e) => setSelectedInstance(e.target.value)}
                  placeholder="Enter instance folder name (e.g. VaultHunters, 1.20.4-Fabric)"
                  className="instance-text-input"
                />
                <span className="form-hint">
                  No automated instances found in this folder. Enter the instance folder name or back up the entire directory.
                </span>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Minecraft World Consistency Advisory Warning */}
      <div className="minecraft-safety-alert">
        <AlertTriangle size={18} className="text-warning flex-shrink-0" />
        <div className="safety-alert-content">
          <strong>Minecraft World Consistency Advisory:</strong>
          <p>
            Backing up an active, running singleplayer world can result in torn region chunks or desynchronized inventory files.
            We strongly advise <strong>closing singleplayer worlds to the title screen</strong> before initiating a backup.
          </p>
        </div>
      </div>
    </div>
  );
};
