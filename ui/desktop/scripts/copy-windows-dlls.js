const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// Required DLLs that must be present
const REQUIRED_DLLS = [
    'libstdc++-6.dll',
    'libgcc_s_seh-1.dll',
    'libwinpthread-1.dll'
];

// Source and target directories
const sourceDir = path.join(__dirname, '../src/bin');
const targetDir = path.join(__dirname, '../out/Goose-win32-x64/resources/bin');

function ensureDirectoryExists(dir) {
    if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
        console.log(`Created directory: ${dir}`);
    }
}

function copyDLLs() {
    // Ensure target directory exists
    ensureDirectoryExists(targetDir);

    // Get list of DLLs in source directory
    const sourceDLLs = fs.readdirSync(sourceDir)
        .filter(file => file.toLowerCase().endsWith('.dll'));

    console.log('Found DLLs in source directory:', sourceDLLs);

    // Check for missing required DLLs
    const missingDLLs = REQUIRED_DLLS.filter(dll => 
        !sourceDLLs.includes(dll)
    );

    if (missingDLLs.length > 0) {
        console.error('Missing required DLLs:', missingDLLs);
        process.exit(1);
    }

    // Copy all DLLs and the executable to target directory
    sourceDLLs.forEach(dll => {
        const sourcePath = path.join(sourceDir, dll);
        const targetPath = path.join(targetDir, dll);
        
        try {
            fs.copyFileSync(sourcePath, targetPath);
            console.log(`Copied ${dll} to ${targetDir}`);
        } catch (err) {
            console.error(`Error copying ${dll}:`, err);
            process.exit(1);
        }
    });

    // Copy the executable
    const exeName = 'goosed.exe';
    const sourceExe = path.join(sourceDir, exeName);
    const targetExe = path.join(targetDir, exeName);
    
    try {
        if (fs.existsSync(sourceExe)) {
            fs.copyFileSync(sourceExe, targetExe);
            console.log(`Copied ${exeName} to ${targetDir}`);
        } else {
            console.error(`${exeName} not found in source directory`);
            process.exit(1);
        }
    } catch (err) {
        console.error(`Error copying ${exeName}:`, err);
        process.exit(1);
    }

    console.log('All files copied successfully');
}

// Main execution
try {
    copyDLLs();
} catch (err) {
    console.error('Error during copy process:', err);
    process.exit(1);
}