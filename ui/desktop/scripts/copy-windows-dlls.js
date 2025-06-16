const fs = require('fs');
const path = require('path');

// Paths
const platformWinDir = path.join(__dirname, '..', 'src', 'platform', 'windows', 'bin');
const outDir = path.join(__dirname, '..', 'out', 'Goose-win32-x64', 'resources', 'bin');
const srcBinDir = path.join(__dirname, '..', 'src', 'bin');

// Helper function to copy files
function copyFiles(sourceDir, targetDir) {
    // Ensure target directory exists
    if (!fs.existsSync(targetDir)) {
        fs.mkdirSync(targetDir, { recursive: true });
    }

    // Copy all files from source to target directory
    console.log(`Copying files to ${targetDir}...`);
    fs.readdirSync(sourceDir).forEach(file => {
        const srcPath = path.join(sourceDir, file);
        const destPath = path.join(targetDir, file);
        
        // Skip .gitignore and README.md
        if (file === '.gitignore' || file === 'README.md') {
            return;
        }

        // Handle directories (like goose-npm)
        if (fs.statSync(srcPath).isDirectory()) {
            fs.cpSync(srcPath, destPath, { recursive: true, force: true });
            console.log(`Copied directory: ${file}`);
            return;
        }
        
        // Copy individual file
        fs.copyFileSync(srcPath, destPath);
        console.log(`Copied: ${file}`);
    });
}

// Copy to both directories
console.log('Copying Windows-specific files...');

// Copy to src/bin for development
copyFiles(platformWinDir, srcBinDir);

// Copy to output directory for distribution
copyFiles(platformWinDir, outDir);

console.log('Windows-specific files copied successfully');
