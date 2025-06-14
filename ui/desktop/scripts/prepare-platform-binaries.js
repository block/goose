const fs = require('fs');
const path = require('path');

// Paths
const srcBinDir = path.join(__dirname, '..', 'src', 'bin');
const platformWinDir = path.join(__dirname, '..', 'src', 'platform', 'windows', 'bin');

// Platform-specific file patterns
const windowsFiles = [
    '*.exe',
    '*.dll',
    '*.cmd',
    'goose-npm/**/*'
];

const macosFiles = [
    'goosed',
    'goose',
    'temporal',
    'temporal-service',
    'jbang',
    'npx',
    'uvx',
    '*.db',
    '*.log',
    '.gitkeep'
];

// Helper function to check if file matches patterns
function matchesPattern(filename, patterns) {
    return patterns.some(pattern => {
        if (pattern.includes('**')) {
            // Handle directory patterns
            const basePattern = pattern.split('/**')[0];
            return filename.startsWith(basePattern);
        } else if (pattern.includes('*')) {
            // Handle wildcard patterns - be more precise with file extensions
            if (pattern.startsWith('*.')) {
                // For file extension patterns like *.exe, *.dll
                const extension = pattern.substring(2); // Remove "*."
                return filename.endsWith('.' + extension);
            } else {
                // For other wildcard patterns
                const regex = new RegExp('^' + pattern.replace(/\*/g, '.*') + '$');
                return regex.test(filename);
            }
        } else {
            // Exact match
            return filename === pattern;
        }
    });
}

// Helper function to clean directory of cross-platform files
function cleanBinDirectory(targetPlatform) {
    console.log(`Cleaning bin directory for ${targetPlatform} build...`);
    
    if (!fs.existsSync(srcBinDir)) {
        console.log('src/bin directory does not exist, skipping cleanup');
        return;
    }

    const files = fs.readdirSync(srcBinDir, { withFileTypes: true });
    
    files.forEach(file => {
        const filePath = path.join(srcBinDir, file.name);
        
        if (targetPlatform === 'darwin' || targetPlatform === 'linux') {
            // For macOS/Linux, remove Windows-specific files
            if (matchesPattern(file.name, windowsFiles)) {
                console.log(`Removing Windows file: ${file.name}`);
                if (file.isDirectory()) {
                    fs.rmSync(filePath, { recursive: true, force: true });
                } else {
                    fs.unlinkSync(filePath);
                }
            }
        } else if (targetPlatform === 'win32') {
            // For Windows, remove macOS-specific files (keep only Windows files and common files)
            if (!matchesPattern(file.name, windowsFiles) && !matchesPattern(file.name, ['*.db', '*.log', '.gitkeep'])) {
                // Check if it's a macOS binary (executable without extension)
                if (file.isFile() && !path.extname(file.name) && file.name !== '.gitkeep') {
                    try {
                        // Check if file is executable (likely a macOS binary)
                        const stats = fs.statSync(filePath);
                        if (stats.mode & parseInt('111', 8)) { // Check if any execute bit is set
                            console.log(`Removing macOS binary: ${file.name}`);
                            fs.unlinkSync(filePath);
                        }
                    } catch (err) {
                        console.warn(`Could not check file ${file.name}:`, err.message);
                    }
                }
            }
        }
    });
}

// Helper function to copy platform-specific files
function copyPlatformFiles(targetPlatform) {
    if (targetPlatform === 'win32') {
        console.log('Copying Windows-specific files...');
        
        if (!fs.existsSync(platformWinDir)) {
            console.warn('Windows platform directory does not exist');
            return;
        }

        // Ensure src/bin exists
        if (!fs.existsSync(srcBinDir)) {
            fs.mkdirSync(srcBinDir, { recursive: true });
        }

        // Copy Windows-specific files
        const files = fs.readdirSync(platformWinDir, { withFileTypes: true });
        files.forEach(file => {
            if (file.name === 'README.md' || file.name === '.gitignore') {
                return;
            }

            const srcPath = path.join(platformWinDir, file.name);
            const destPath = path.join(srcBinDir, file.name);
            
            if (file.isDirectory()) {
                fs.cpSync(srcPath, destPath, { recursive: true, force: true });
                console.log(`Copied directory: ${file.name}`);
            } else {
                fs.copyFileSync(srcPath, destPath);
                console.log(`Copied: ${file.name}`);
            }
        });
    }
}

// Main function
function preparePlatformBinaries() {
    const targetPlatform = process.env.ELECTRON_PLATFORM || process.platform;
    
    console.log(`Preparing binaries for platform: ${targetPlatform}`);
    
    // First copy platform-specific files if needed
    copyPlatformFiles(targetPlatform);
    
    // Then clean up cross-platform files
    cleanBinDirectory(targetPlatform);
    
    console.log('Platform binary preparation complete');
}

// Run if called directly
if (require.main === module) {
    preparePlatformBinaries();
}

module.exports = { preparePlatformBinaries };