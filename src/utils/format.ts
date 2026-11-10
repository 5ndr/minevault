export function formatBytes(bytes?: number | null, decimals: number = 2): string {
  if (bytes === undefined || bytes === null || isNaN(bytes) || bytes <= 0) return '0 B';
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'];

  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const safeI = Math.min(Math.max(0, i), sizes.length - 1);
  return `${parseFloat((bytes / Math.pow(k, safeI)).toFixed(dm))} ${sizes[safeI]}`;
}

export function formatRelativeTime(isoString?: string): string {
  if (!isoString) return 'Never';
  const date = new Date(isoString);
  if (isNaN(date.getTime())) return 'Never';
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);

  if (diffSec < 60) return `${diffSec}s ago`;
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHour < 24) return `${diffHour}h ago`;
  if (diffDay === 1) return 'Yesterday';
  if (diffDay < 7) return `${diffDay}d ago`;

  const m = String(date.getMonth() + 1).padStart(2, '0');
  const d = String(date.getDate()).padStart(2, '0');
  const y = String(date.getFullYear()).slice(-2);
  return `${m}-${d}-${y}`;
}

/** Formats dates as MM-DD-YY HH:MM:SS */
export function formatExactTimestamp(isoString?: string): string {
  if (!isoString) return 'N/A';
  const date = new Date(isoString);
  if (isNaN(date.getTime())) return 'N/A';
  const m = String(date.getMonth() + 1).padStart(2, '0');
  const d = String(date.getDate()).padStart(2, '0');
  const y = String(date.getFullYear()).slice(-2);
  const hours = String(date.getHours()).padStart(2, '0');
  const mins = String(date.getMinutes()).padStart(2, '0');
  const secs = String(date.getSeconds()).padStart(2, '0');
  return `${m}-${d}-${y} ${hours}:${mins}:${secs}`;
}

/** Formats dates as MM-DD-YY HH:MM */
export function formatDateShort(isoString?: string): string {
  if (!isoString) return 'N/A';
  const date = new Date(isoString);
  if (isNaN(date.getTime())) return 'N/A';
  const m = String(date.getMonth() + 1).padStart(2, '0');
  const d = String(date.getDate()).padStart(2, '0');
  const y = String(date.getFullYear()).slice(-2);
  const hours = String(date.getHours()).padStart(2, '0');
  const mins = String(date.getMinutes()).padStart(2, '0');
  return `${m}-${d}-${y} ${hours}:${mins}`;
}

export function formatDuration(seconds: number): string {
  if (seconds < 1) return `${Math.round(seconds * 1000)}ms`;
  if (seconds < 60) return `${seconds.toFixed(1)}s`;
  const mins = Math.floor(seconds / 60);
  const remSec = Math.round(seconds % 60);
  return `${mins}m ${remSec}s`;
}

export interface B2PricingEstimate {
  monthlyCost: number;
  yearlyCost: number;
  monthlyFormatted: string;
  yearlyFormatted: string;
  isFreeTier: boolean;
  rateDescription: string;
  totalGb: number;
  billableGb: number;
}

/**
 * Calculates rough Backblaze B2 storage pricing:
 * - 10 GB Free Storage tier
 * - $0.006 per GB / month ($6 / TB / month) beyond 10 GB
 * - 3x monthly storage free downloads
 */
export function calculateB2Pricing(bytes?: number | null): B2PricingEstimate {
  if (!bytes || bytes <= 0 || isNaN(bytes)) {
    return {
      monthlyCost: 0,
      yearlyCost: 0,
      monthlyFormatted: '$0.00/mo',
      yearlyFormatted: '$0.00/yr',
      isFreeTier: true,
      rateDescription: '10 GB Free Tier ($0.006/GB/mo thereafter)',
      totalGb: 0,
      billableGb: 0,
    };
  }

  const totalGb = bytes / (1024 * 1024 * 1024);
  const FREE_TIER_GB = 10;
  const RATE_PER_GB_MONTH = 0.006;

  if (totalGb <= FREE_TIER_GB) {
    return {
      monthlyCost: 0,
      yearlyCost: 0,
      monthlyFormatted: '$0.00/mo',
      yearlyFormatted: '$0.00/yr',
      isFreeTier: true,
      rateDescription: `Free Tier (First 10 GB free • ${totalGb.toFixed(1)} GB stored)`,
      totalGb,
      billableGb: 0,
    };
  }

  const billableGb = totalGb - FREE_TIER_GB;
  const monthlyCost = billableGb * RATE_PER_GB_MONTH;
  const yearlyCost = monthlyCost * 12;

  const monthlyFormatted = monthlyCost < 0.01 ? '<$0.01/mo' : `~$${monthlyCost.toFixed(2)}/mo`;
  const yearlyFormatted = yearlyCost < 0.05 ? '<$0.05/yr' : `~$${yearlyCost.toFixed(2)}/yr`;

  return {
    monthlyCost,
    yearlyCost,
    monthlyFormatted,
    yearlyFormatted,
    isFreeTier: false,
    rateDescription: `$0.006/GB/mo after 10 GB free (${totalGb.toFixed(1)} GB total, ${billableGb.toFixed(1)} GB billed)`,
    totalGb,
    billableGb,
  };
}
