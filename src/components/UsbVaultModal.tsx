import React, { useState } from 'react';
import { Lock, Unlock, X, AlertTriangle } from 'lucide-react';
import { UsbVaultStatus } from '../types';

interface UsbVaultModalProps {
  isOpen: boolean;
  onClose: () => void;
  vaultStatus: UsbVaultStatus | null;
  onUnlock: (passphrase: string) => Promise<void>;
  onClearVault?: () => Promise<void>;
}

export const UsbVaultModal: React.FC<UsbVaultModalProps> = ({
  isOpen,
  onClose,
  vaultStatus,
  onUnlock,
  onClearVault,
}) => {
  if (!isOpen) return null;

  const [passphrase, setPassphrase] = useState('');
  const [unlocking, setUnlocking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleUnlockClick = async () => {
    setUnlocking(true);
    setError(null);
    try {
      await onUnlock(passphrase);
      onClose();
    } catch (e: any) {
      setError(e?.toString() || 'Incorrect passphrase or corrupted vault file');
    } finally {
      setUnlocking(false);
    }
  };

  return (
    <div className="modal-backdrop">
      <div className="modal-box modal-sm">
        <div className="modal-header">
          <div className="modal-header-title">
            <Lock size={18} className="text-warning" />
            <h3>Unlock USB Key Vault</h3>
          </div>
          <button onClick={onClose} className="close-btn" disabled={unlocking}>
            <X size={18} />
          </button>
        </div>

        <div className="modal-body">
          <p className="text-muted text-xs">
            An encrypted key vault (<code>minevault_keys.enc</code>) was detected on this USB drive. Enter your master vault passphrase to decrypt your B2 Application Keys into session memory.
          </p>

          <div className="form-group mt-3">
            <label className="text-xs font-semibold">Vault Master Passphrase:</label>
            <input
              type="password"
              value={passphrase}
              onChange={(e) => setPassphrase(e.target.value)}
              placeholder="Enter vault passphrase..."
              disabled={unlocking}
              autoFocus
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleUnlockClick();
              }}
            />
          </div>

          {error && (
            <div className="alert-banner alert-error mt-3">
              <AlertTriangle size={15} className="flex-shrink-0" />
              <span className="text-xs">{error}</span>
            </div>
          )}

          {vaultStatus?.vault_path && (
            <div className="text-xs text-muted mt-2 truncate font-mono">
              Path: {vaultStatus.vault_path}
            </div>
          )}
        </div>

        <div className="modal-footer space-between">
          {onClearVault ? (
            <button
              type="button"
              onClick={async () => {
                if (confirm('Delete this encrypted vault file from the USB drive?')) {
                  await onClearVault();
                  onClose();
                }
              }}
              className="btn-danger btn-tiny"
            >
              Reset Vault
            </button>
          ) : <div />}

          <div className="footer-actions">
            <button onClick={onClose} className="btn-secondary" disabled={unlocking}>
              Cancel
            </button>
            <button
              onClick={handleUnlockClick}
              disabled={unlocking || !passphrase.trim()}
              className="btn-primary"
            >
              <Unlock size={14} />
              <span>{unlocking ? 'Decrypting...' : 'Unlock Vault'}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
