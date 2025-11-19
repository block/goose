/**
 * Platform-specific utilities for handling cross-platform differences
 */

/**
 * Detects if running on Windows
 */
export function isWindows(): boolean {
  if (typeof window !== 'undefined' && window.electron) {
    const platform = window.electron.platform || navigator.platform;
    return platform === 'win32' || platform.includes('Win');
  }
  return navigator.platform.includes('Win');
}

/**
 * Fixes the command for Windows by adding .cmd extension to npm commands
 */
export function fixCommandForPlatform(cmd: string): string {
  if (!isWindows()) {
    return cmd;
  }

  const npmCommands = ['npx', 'npm', 'yarn', 'pnpm'];
  
  if (npmCommands.includes(cmd)) {
    return `${cmd}.cmd`;
  }
  
  return cmd;
}

/**
 * Validates and fixes extension arguments for the current platform
 * Specifically handles path placeholders on Windows
 */
export function fixExtensionArgsForPlatform(args: string[]): {
  args: string[];
  warnings: string[];
} {
  if (!isWindows()) {
    return { args, warnings: [] };
  }

  const warnings: string[] = [];
  const suggestedPaths = suggestWindowsDirectories();
  let pathIndex = 0;

  const fixedArgs = args.map((arg) => {
    // Detect Unix-style placeholder paths
    if (arg.startsWith('/path/to/') || arg === '/path/to/dir1' || arg === '/path/to/dir2') {
      const replacement = pathIndex < suggestedPaths.length 
        ? suggestedPaths[pathIndex++]
        : suggestedPaths[0];
      
      warnings.push(`Replaced placeholder ${arg} with ${replacement}`);
      return replacement;
    }
    
    return arg;
  });

  return { args: fixedArgs, warnings };
}

/**
 * Suggests common Windows directories based on user profile
 */
export function suggestWindowsDirectories(): string[] {
  const userHome = (typeof window !== 'undefined' && window.electron?.getEnv?.('USERPROFILE')) || 'C:\\Users\\Public';
  
  const suggestions = [
    userHome,
    `${userHome}\\Documents`,
    `${userHome}\\Desktop`,
  ];
  
  const possibleProjectDirs = [
    `${userHome}\\Projects`,
    `${userHome}\\source`,
    `${userHome}\\repos`,
    'D:\\Projects',
    'C:\\Projects',
  ];
  
  suggestions.push(...possibleProjectDirs.slice(0, 2));
  
  return suggestions;
}

/**
 * Comprehensive fix for extension config on Windows
 */
export function fixExtensionConfigForPlatform(config: {
  cmd: string;
  args: string[];
}): {
  cmd: string;
  args: string[];
  needsUserInput: boolean;
  suggestions?: string[];
  warnings?: string[];
} {
  if (!isWindows()) {
    return {
      cmd: config.cmd,
      args: config.args,
      needsUserInput: false,
    };
  }

  const fixedCmd = fixCommandForPlatform(config.cmd);
  const { args: fixedArgs, warnings } = fixExtensionArgsForPlatform(config.args);
  const needsUserInput = warnings.length > 0;
  const suggestions = needsUserInput ? suggestWindowsDirectories() : undefined;

  return {
    cmd: fixedCmd,
    args: fixedArgs,
    needsUserInput,
    suggestions,
    warnings,
  };
}
