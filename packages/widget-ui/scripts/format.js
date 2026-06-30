const MIB = 1024 * 1024;
const GIB = MIB * 1024;

function formatScaled(value, unit) {
  if (value >= 100 || Math.abs(value - Math.round(value)) < 0.05) {
    return `${Math.round(value)} ${unit}`;
  }
  return `${value.toFixed(1)} ${unit}`;
}

function formatByteSize(bytes) {
  if (!bytes || bytes <= 0) return '';
  if (bytes >= GIB) {
    return formatScaled(bytes / GIB, 'GB');
  }
  return formatScaled(bytes / MIB, 'MB');
}

function formatByteSizeApprox(bytes) {
  const label = formatByteSize(bytes);
  return label ? `~${label}` : '';
}

function formatSpeedMbps(mbps) {
  if (!mbps || mbps <= 0) return '';
  if (mbps >= 100) return `${Math.round(mbps)} MB/s`;
  return `${mbps.toFixed(1)} MB/s`;
}

window.formatByteSize = formatByteSize;
window.formatByteSizeApprox = formatByteSizeApprox;
window.formatSpeedMbps = formatSpeedMbps;
