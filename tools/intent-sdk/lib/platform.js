/**
 * Platform detection and binary path resolution
 */

const os = require('os');
const path = require('path');
const fs = require('fs');

/**
 * Detect the current platform and architecture
 */
function detectPlatform() {
  const platform = os.platform();
  const arch = os.arch();
  
  // Map Node.js platform/arch to our naming convention
  const platformMap = {
    'darwin': 'darwin',
    'linux': 'linux',
    'win32': 'win32'
  };
  
  const archMap = {
    'x64': 'x64',
    'arm64': 'arm64',
    'aarch64': 'arm64'
  };
  
  const mappedPlatform = platformMap[platform];
  const mappedArch = archMap[arch];
  
  if (!mappedPlatform || !mappedArch) {
    throw new Error(`Unsupported platform: ${platform}-${arch}`);
  }
  
  return {
    os: mappedPlatform,
    arch: mappedArch,
    original: {
      platform,
      arch
    }
  };
}

/**
 * Get the path to the platform-specific binary
 */
async function getBinaryPath(platform) {
  const binaryName = platform.os === 'win32' ? 'synapsed-mcp-server.exe' : 'synapsed-mcp-server';
  
  // First, check if we have a local binary (for development)
  const localBinaryPath = path.join(__dirname, '..', 'binaries', `${platform.os}-${platform.arch}`, binaryName);
  if (fs.existsSync(localBinaryPath)) {
    return localBinaryPath;
  }
  
  // Check for installed platform package
  try {
    const packageName = `@synapsed/intent-sdk-${platform.os}-${platform.arch}`;
    const packagePath = require.resolve(packageName);
    const packageDir = path.dirname(packagePath);
    const packageBinaryPath = path.join(packageDir, 'bin', binaryName);
    
    if (fs.existsSync(packageBinaryPath)) {
      return packageBinaryPath;
    }
  } catch (e) {
    // Package not installed
  }
  
  // Fallback to adjacent binary (for testing)
  const adjacentPath = path.join(__dirname, '..', '..', 'target', 'release', binaryName);
  if (fs.existsSync(adjacentPath)) {
    return adjacentPath;
  }
  
  throw new Error(`Binary not found for platform ${platform.os}-${platform.arch}`);
}

/**
 * Check if a binary is executable
 */
function isBinaryExecutable(binaryPath) {
  try {
    fs.accessSync(binaryPath, fs.constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

/**
 * Make a binary executable (Unix-like systems)
 */
function makeBinaryExecutable(binaryPath) {
  if (os.platform() !== 'win32') {
    try {
      fs.chmodSync(binaryPath, 0o755);
      return true;
    } catch (error) {
      console.warn(`Failed to make binary executable: ${error.message}`);
      return false;
    }
  }
  return true;
}

module.exports = {
  detectPlatform,
  getBinaryPath,
  isBinaryExecutable,
  makeBinaryExecutable
};