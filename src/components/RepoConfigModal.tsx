import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Database,
  Lock,
  Unlock,
  ShieldAlert,
  Folder,
  Cloud,
  CheckCircle,
  AlertTriangle,
  X,
  Trash2,
  Key,
} from 'lucide-react';
import { RepoConfig, SessionCredentials, RepoType } from '../types';

interface RepoConfigModalProps {
  isOpen: boolean;
  onClose: () => void;
  repo: RepoConfig | null;
  creds: SessionCredentials | null;
  onSaveRepo: (repo: RepoConfig, creds: SessionCredentials) => Promise<void>;
  onTestConnection: (repo: RepoConfig, creds: SessionCredentials) => Promise<string>;
  onForgetCredentials: () => void;
}

export const RepoConfigModal: React.FC<RepoConfigModalProps> = ({
  isOpen,
  onClose,
  repo,
  creds,
  onSaveRepo,
  onTestConnection,
  onForgetCredentials,
}) => {
  if (!isOpen) return null;

  const [repoType, setRepoType] = useState<RepoType>(repo?.repo_type || 's3_b2');
  const [name, setName] = useState<string>(repo?.name || 'Backblaze B2 Repository');
  const [target, setTarget] = useState<string>(
    repo?.target || 's3:https://s3.us-east-005.backblazeb2.com/PrismBackups/minecraft'
  );
  const [insecureNoPassword, setInsecureNoPassword] = useState<boolean>(
    repo ? repo.insecure_no_password : true // Default matches user's existing repo initialized with --insecure-no-password
  );
  const [b2KeyId, setB2KeyId] = useState<string>(repo?.b2_key_id || '');
  const [b2AppKey, setB2AppKey] = useState<string>(creds?.b2_application_key || '');
  const [b2ManagementKeyId, setB2ManagementKeyId] = useState<string>(repo?.b2_management_key_id || '');
  const [b2ManagementAppKey, setB2ManagementAppKey] = useState<string>(creds?.b2_management_app_key || '');
  const [showAdvancedB2, setShowAdvancedB2] = useState<boolean>(Boolean(repo?.b2_management_key_id));
  const [saveToUsbVault, setSaveToUsbVault] = useState<boolean>(true);
  const [usbVaultPassphrase, setUsbVaultPassphrase] = useState<string>('');
  const [password, setPassword] = useState<string>(creds?.password || '');
  const [testing, setTesting] = useState<boolean>(false);
  const [testResult, setTestResult] = useState<string | null>(null);
  const [testError, setTestError] = useState<string | null>(null);

  const formatTarget = (val: string): string => {
    let t = val.trim();
    if (repoType === 's3_b2') {
      if (t.startsWith('https://')) {
        t = `s3:${t}`;
      } else if (t.startsWith('s3://https://')) {
        t = `s3:https://${t.slice('s3://https://'.length)}`;
      } else if (!t.startsWith('s3:') && t.includes('backblazeb2.com')) {
        t = `s3:https://${t}`;
      }
    }
    return t;
  };

  const handleTest = async () => {
    setTesting(true);
    setTestResult(null);
    setTestError(null);

    const cleanTarget = formatTarget(target);
    setTarget(cleanTarget);

    const testRepoConfig: RepoConfig = {
      id: repo?.id || 'primary-repo',
      name,
      repo_type: repoType,
      target: cleanTarget,
      insecure_no_password: insecureNoPassword,
      has_password: !insecureNoPassword && password.trim().length > 0,
      b2_key_id: repoType === 's3_b2' ? b2KeyId : undefined,
      b2_management_key_id: repoType === 's3_b2' && b2ManagementKeyId.trim() ? b2ManagementKeyId.trim() : undefined,
    };

    const testSessionCreds: SessionCredentials = {
      password: insecureNoPassword ? undefined : password,
      b2_application_key: repoType === 's3_b2' ? b2AppKey : undefined,
      b2_management_app_key: repoType === 's3_b2' && b2ManagementAppKey.trim() ? b2ManagementAppKey.trim() : undefined,
      save_to_local_os: false,
    };

    try {
      const msg = await onTestConnection(testRepoConfig, testSessionCreds);
      setTestResult(msg);
    } catch (e: any) {
      setTestError(e?.toString() || 'Connection failed');
    } finally {
      setTesting(false);
    }
  };

  const handleSave = async () => {
    const cleanTarget = formatTarget(target);
    const updatedRepo: RepoConfig = {
      id: repo?.id || 'primary-repo',
      name,
      repo_type: repoType,
      target: cleanTarget,
      insecure_no_password: insecureNoPassword,
      has_password: !insecureNoPassword && password.trim().length > 0,
      b2_key_id: repoType === 's3_b2' ? b2KeyId : undefined,
      b2_management_key_id: repoType === 's3_b2' && b2ManagementKeyId.trim() ? b2ManagementKeyId.trim() : undefined,
    };

    const updatedCreds: SessionCredentials = {
      password: insecureNoPassword ? undefined : password,
      b2_application_key: repoType === 's3_b2' ? b2AppKey : undefined,
      b2_management_app_key: repoType === 's3_b2' && b2ManagementAppKey.trim() ? b2ManagementAppKey.trim() : undefined,
      save_to_local_os: false,
    };

    await onSaveRepo(updatedRepo, updatedCreds);

    if (saveToUsbVault && (b2AppKey.trim() || b2ManagementAppKey.trim())) {
      try {
        await invoke('save_usb_vault_cmd', {
          repo: updatedRepo,
          creds: updatedCreds,
          passphrase: usbVaultPassphrase.trim() || null,
        });
      } catch (err) {
        console.error('Failed to save encrypted USB vault:', err);
      }
    }

    onClose();
  };

  return (
    <div className="modal-backdrop">
      <div className="modal-box modal-lg">
        <div className="modal-header">
          <div className="modal-header-title">
            <Database size={18} className="text-primary" />
            <h3>Repository Configuration</h3>
          </div>
          <button onClick={onClose} className="close-btn">
            <X size={18} />
          </button>
        </div>

        <div className="modal-body">
          {/* Repo Type Selection */}
          <div className="form-group">
            <label>Repository Storage Engine:</label>
            <div className="radio-group-tabs">
              <button
                type="button"
                className={`tab-btn ${repoType === 's3_b2' ? 'active' : ''}`}
                onClick={() => setRepoType('s3_b2')}
              >
                <Cloud size={16} />
                <span>Backblaze B2 (S3 Endpoint)</span>
              </button>
              <button
                type="button"
                className={`tab-btn ${repoType === 'local' ? 'active' : ''}`}
                onClick={() => setRepoType('local')}
              >
                <Folder size={16} />
                <span>Local / USB Drive Folder</span>
              </button>
            </div>
          </div>

          <div className="form-group">
            <label>Profile / Display Name:</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Backblaze B2 Minecraft Repository"
            />
          </div>

          <div className="form-group">
            <label>
              {repoType === 's3_b2'
                ? 'S3 URL Endpoint (e.g. s3:https://s3.<region>.backblazeb2.com/<bucket>/<optional-prefix>):'
                : 'Local Repository Path:'}
            </label>
            <input
              type="text"
              value={target}
              onChange={(e) => setTarget(e.target.value)}
              placeholder={
                repoType === 's3_b2'
                  ? 's3:https://s3.us-east-005.backblazeb2.com/PrismBackups/minecraft'
                  : '/Volumes/USB_Drive/backups/repo'
              }
            />
            {repoType === 's3_b2' && (
              <span className="form-hint">
                Format required by restic: <code>s3:https://s3.&lt;region&gt;.backblazeb2.com/&lt;bucket&gt;/&lt;prefix&gt;</code>
                <br />
                Example: <code>s3:https://s3.us-east-005.backblazeb2.com/PrismBackups/minecraft</code>
              </span>
            )}
          </div>

          {/* B2 Credentials */}
          {repoType === 's3_b2' && (
            <div className="credentials-group">
              <div className="form-group">
                <label>B2 Application Key ID (AWS_ACCESS_KEY_ID):</label>
                <input
                  type="text"
                  value={b2KeyId}
                  onChange={(e) => setB2KeyId(e.target.value)}
                  placeholder="004..."
                />
              </div>

              <div className="form-group">
                <label>B2 Secret Application Key (AWS_SECRET_ACCESS_KEY):</label>
                <input
                  type="password"
                  value={b2AppKey}
                  onChange={(e) => setB2AppKey(e.target.value)}
                  placeholder="K004..."
                />
                <span className="form-hint">
                  Session memory only. Never written to disk or the USB drive without explicit local scheduling setup.
                </span>
              </div>

              {/* Dedicated B2 Management API Key (Optional) */}
              <div className="advanced-b2-accordion mt-2">
                <button
                  type="button"
                  onClick={() => setShowAdvancedB2(!showAdvancedB2)}
                  className="accordion-toggle-btn text-xs text-muted hover:text-main"
                >
                  <span>{showAdvancedB2 ? '▼' : '▶'} Optional: Dedicated B2 Management API Token (for direct provider bucket size)</span>
                </button>

                {showAdvancedB2 && (
                  <div className="mt-2 p-2.5 rounded bg-surface border border-subtle text-xs space-y-2">
                    <p className="text-muted">
                      By default, the app uses your repository Application Key above. If your repository key has restricted capabilities, you can enter a dedicated B2 Management Key ID and Application Key with <code>listBuckets</code> / <code>listFileNames</code> permissions here.
                    </p>
                    <div className="form-group">
                      <label>Management Key ID:</label>
                      <input
                        type="text"
                        value={b2ManagementKeyId}
                        onChange={(e) => setB2ManagementKeyId(e.target.value)}
                        placeholder="004..."
                      />
                    </div>
                    <div className="form-group">
                      <label>Management Secret Application Key:</label>
                      <input
                        type="password"
                        value={b2ManagementAppKey}
                        onChange={(e) => setB2ManagementAppKey(e.target.value)}
                        placeholder="K004..."
                      />
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Encryption / Passwordless Toggle */}
          <div className="encryption-options-card">
            <div className="encryption-header">
              {insecureNoPassword ? (
                <Unlock size={18} className="text-warning" />
              ) : (
                <Lock size={18} className="text-success" />
              )}
              <strong>Repository Password Mode</strong>
            </div>

            <label className="checkbox-row">
              <input
                type="checkbox"
                checked={insecureNoPassword}
                onChange={(e) => setInsecureNoPassword(e.target.checked)}
              />
              <span>Use Empty Passwordless Mode (--insecure-no-password)</span>
            </label>

            {insecureNoPassword ? (
              <div className="security-notice-box">
                <ShieldAlert size={16} className="text-warning flex-shrink-0" />
                <div>
                  <strong>Passwordless Repository Support:</strong>
                  <p>
                    Your repository was initialized with <code>--insecure-no-password</code>. The app supports this mode
                    without altering or converting it. Anyone with read access to the storage bucket can read snapshots
                    without a passphrase.
                  </p>
                </div>
              </div>
            ) : (
              <div className="form-group mt-3">
                <label>Repository Encryption Password:</label>
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder="Enter repository password..."
                />
              </div>
            )}
          </div>

          {/* Encrypted USB Storage */}
          {repoType === 's3_b2' && (
            <div className="usb-vault-config-card mt-3">
              <div className="flex items-center gap-2">
                <Key size={16} className="text-primary" />
                <strong>Encrypted USB Key Vault</strong>
              </div>

              <label className="checkbox-row mt-2">
                <input
                  type="checkbox"
                  checked={saveToUsbVault}
                  onChange={(e) => setSaveToUsbVault(e.target.checked)}
                />
                <span>Store Application Keys encrypted on USB drive (AES-256-GCM)</span>
              </label>

              {saveToUsbVault && (
                <div className="mt-2 pl-6 space-y-2">
                  <div className="form-group">
                    <label className="text-xs">USB Vault Master Passphrase (Optional):</label>
                    <input
                      type="password"
                      value={usbVaultPassphrase}
                      onChange={(e) => setUsbVaultPassphrase(e.target.value)}
                      placeholder="Leave blank for automatic portable USB unlock"
                    />
                    <span className="form-hint">
                      If set, you'll enter this passphrase when launching MineVault on any computer. If left blank, keys remain encrypted via an internal USB device seed.
                    </span>
                  </div>
                </div>
              )}
            </div>
          )}

          {/* Test Connection Results */}
          {testResult && (
            <div className="alert-banner alert-success">
              <CheckCircle size={16} />
              <span>{testResult}</span>
            </div>
          )}

          {testError && (
            <div className="alert-banner alert-error">
              <AlertTriangle size={16} />
              <span>{testError}</span>
            </div>
          )}
        </div>

        <div className="modal-footer space-between">
          <div>
            <button
              type="button"
              onClick={onForgetCredentials}
              className="btn-danger btn-tiny"
              title="Clears in-memory keys and saved credentials"
            >
              <Trash2 size={13} />
              <span>Forget Credentials</span>
            </button>
          </div>

          <div className="footer-actions">
            <button
              type="button"
              onClick={handleTest}
              disabled={testing || !target.trim()}
              className="btn-secondary"
            >
              {testing ? 'Testing connection...' : 'Test Connection (Read-Only)'}
            </button>
            <button type="button" onClick={handleSave} className="btn-primary">
              Save Configuration
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
