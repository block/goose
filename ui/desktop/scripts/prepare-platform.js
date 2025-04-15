const { execSync } = require('child_process');
const path = require('path');

try {
    if (process.platform === 'win32') {
        execSync(path.join(__dirname, 'prepare-windows-npm.bat'), { stdio: 'inherit' });
    } else {
        execSync(path.join(__dirname, 'prepare-windows-npm.sh'), { 
            stdio: 'inherit',
            shell: '/bin/bash'
        });
    }
} catch (error) {
    console.error('Error preparing platform:', error);
    process.exit(1);
}