import { ResticSnapshot } from '../types';

export interface SnapshotLauncherInfo {
  id: string;
  name: string;
  originalPath: string;
  isSpecificInstance: boolean;
  instanceName?: string;
  scopeType: 'launcher' | 'instance';
  scopeLabel: string;
}

const KNOWN_LAUNCHERS: Record<string, string> = {
  prism: 'Prism Launcher',
  prismlauncher: 'Prism Launcher',
  curseforge: 'CurseForge',
  modrinth: 'Modrinth App',
  vanilla: 'Minecraft (Vanilla)',
  atlauncher: 'ATLauncher',
  technic: 'Technic Launcher',
  ftb: 'FTB App',
  custom: 'Custom Minecraft Directory',
};

export function getLauncherDisplayName(id: string): string {
  const cleanId = id.replace(/^launcher:/i, '').toLowerCase();
  return KNOWN_LAUNCHERS[cleanId] || (cleanId.charAt(0).toUpperCase() + cleanId.slice(1));
}

export function detectLauncherFromSnapshot(snapshot?: ResticSnapshot | null): SnapshotLauncherInfo {
  const originalPath = snapshot?.paths?.[0] || '';
  const lowerPath = originalPath.toLowerCase();
  const tags = snapshot?.tags || [];

  let isSpecificInstance = false;
  let instanceName: string | undefined;

  // 1. Check tags for explicit instance or scope tag
  let hasExplicitLauncherScope = false;
  for (const t of tags) {
    const low = t.toLowerCase();
    if (low === 'scope:launcher' || low === 'backup:launcher') {
      hasExplicitLauncherScope = true;
    } else if (low.startsWith('instance:')) {
      isSpecificInstance = true;
      instanceName = t.slice(9);
    } else if (low === 'scope:instance' || low === 'backup:instance' || low === 'instance_backup') {
      isSpecificInstance = true;
    }
  }

  // 2. Check if originalPath points to a specific instance folder
  if (!hasExplicitLauncherScope) {
    const parts = originalPath.split(/[/\\]/).filter(Boolean);
    const instIdx = parts.findIndex((p) => p.toLowerCase() === 'instances');
    if (instIdx !== -1 && instIdx + 1 < parts.length) {
      isSpecificInstance = true;
      instanceName = instanceName || parts[instIdx + 1];
    }

    const profIdx = parts.findIndex((p) => p.toLowerCase() === 'profiles');
    if (profIdx !== -1 && profIdx + 1 < parts.length) {
      isSpecificInstance = true;
      instanceName = instanceName || parts[profIdx + 1];
    }
  }

  let launcherId = 'minecraft';
  let launcherName = 'Minecraft Launcher';

  // 3. Check explicit 'launcher:<id>' tag
  for (const t of tags) {
    if (t.toLowerCase().startsWith('launcher:')) {
      launcherId = t.slice(9).toLowerCase();
      launcherName = getLauncherDisplayName(launcherId);
      break;
    }
  }

  // 4. Check standalone launcher tags (e.g. 'prism', 'curseforge')
  if (launcherId === 'minecraft') {
    for (const t of tags) {
      const low = t.toLowerCase();
      if (KNOWN_LAUNCHERS[low]) {
        launcherId = low;
        launcherName = KNOWN_LAUNCHERS[low];
        break;
      }
    }
  }

  // 5. Fallback: Detect launcher from path structure
  if (launcherId === 'minecraft') {
    if (lowerPath.includes('prismlauncher') || lowerPath.includes('/prism/')) {
      launcherId = 'prism';
      launcherName = 'Prism Launcher';
    } else if (lowerPath.includes('modrinth') || lowerPath.includes('theseus')) {
      launcherId = 'modrinth';
      launcherName = 'Modrinth App';
    } else if (lowerPath.includes('curseforge') || lowerPath.includes('curse client')) {
      launcherId = 'curseforge';
      launcherName = 'CurseForge';
    } else if (lowerPath.includes('.minecraft')) {
      launcherId = 'vanilla';
      launcherName = 'Minecraft (Vanilla)';
    } else if (lowerPath.includes('atlauncher')) {
      launcherId = 'atlauncher';
      launcherName = 'ATLauncher';
    }
  }

  const scopeType: 'launcher' | 'instance' = isSpecificInstance ? 'instance' : 'launcher';
  const scopeLabel = isSpecificInstance
    ? instanceName
      ? `Instance: ${instanceName}`
      : 'Instance Backup'
    : 'Full Launcher Backup';

  return {
    id: launcherId,
    name: launcherName,
    originalPath,
    isSpecificInstance,
    instanceName,
    scopeType,
    scopeLabel,
  };
}
