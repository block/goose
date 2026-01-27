import { spawn, execSync } from 'child_process';
import * as path from 'path';
import * as fs from 'fs';

export interface WorktreeInfo {
  path: string;
  branch: string;
  commit: string;
}

export class GitWorktreeManager {
  private repoPath: string;
  private worktreesDir: string;

  constructor(repoPath: string) {
    this.repoPath = repoPath;
    this.worktreesDir = path.join(repoPath, '.goose-worktrees');
  }

  isGitRepo(): boolean {
    try {
      execSync('git rev-parse --git-dir', { 
        cwd: this.repoPath, 
        stdio: 'pipe' 
      });
      return true;
    } catch {
      return false;
    }
  }

  ensureWorktreesDir(): void {
    if (!fs.existsSync(this.worktreesDir)) {
      fs.mkdirSync(this.worktreesDir, { recursive: true });
    }
    
    // Add to .gitignore if not already there
    const gitignorePath = path.join(this.repoPath, '.gitignore');
    const ignoreEntry = '.goose-worktrees/';
    
    if (fs.existsSync(gitignorePath)) {
      const content = fs.readFileSync(gitignorePath, 'utf-8');
      if (!content.includes(ignoreEntry)) {
        fs.appendFileSync(gitignorePath, `\n${ignoreEntry}\n`);
      }
    }
  }

  async createWorktree(name: string, baseBranch?: string): Promise<WorktreeInfo> {
    this.ensureWorktreesDir();
    
    const branchName = `goose/${name}`;
    const worktreePath = path.join(this.worktreesDir, name);
    
    // Remove existing worktree if it exists
    if (fs.existsSync(worktreePath)) {
      await this.removeWorktree(name);
    }

    // Get the base branch (default to current branch)
    const base = baseBranch || this.getCurrentBranch();
    
    // Create a new branch and worktree
    try {
      // First, try to create the branch
      execSync(`git branch ${branchName} ${base}`, {
        cwd: this.repoPath,
        stdio: 'pipe'
      });
    } catch {
      // Branch might already exist, that's ok
    }

    // Create the worktree
    execSync(`git worktree add "${worktreePath}" ${branchName}`, {
      cwd: this.repoPath,
      stdio: 'pipe'
    });

    const commit = this.getCommitHash(worktreePath);

    return {
      path: worktreePath,
      branch: branchName,
      commit
    };
  }

  async removeWorktree(name: string): Promise<void> {
    const worktreePath = path.join(this.worktreesDir, name);
    
    if (fs.existsSync(worktreePath)) {
      execSync(`git worktree remove "${worktreePath}" --force`, {
        cwd: this.repoPath,
        stdio: 'pipe'
      });
    }

    // Optionally clean up the branch
    const branchName = `goose/${name}`;
    try {
      execSync(`git branch -D ${branchName}`, {
        cwd: this.repoPath,
        stdio: 'pipe'
      });
    } catch {
      // Branch might not exist or might be checked out elsewhere
    }
  }

  listWorktrees(): WorktreeInfo[] {
    try {
      const output = execSync('git worktree list --porcelain', {
        cwd: this.repoPath,
        encoding: 'utf-8'
      });

      const worktrees: WorktreeInfo[] = [];
      const entries = output.split('\n\n').filter(Boolean);

      for (const entry of entries) {
        const lines = entry.split('\n');
        let wtPath = '';
        let branch = '';
        let commit = '';

        for (const line of lines) {
          if (line.startsWith('worktree ')) {
            wtPath = line.substring(9);
          } else if (line.startsWith('branch ')) {
            branch = line.substring(7).replace('refs/heads/', '');
          } else if (line.startsWith('HEAD ')) {
            commit = line.substring(5);
          }
        }

        // Only include worktrees in our managed directory
        if (wtPath.startsWith(this.worktreesDir)) {
          worktrees.push({ path: wtPath, branch, commit });
        }
      }

      return worktrees;
    } catch {
      return [];
    }
  }

  getCurrentBranch(): string {
    try {
      return execSync('git rev-parse --abbrev-ref HEAD', {
        cwd: this.repoPath,
        encoding: 'utf-8'
      }).trim();
    } catch {
      return 'main';
    }
  }

  getCommitHash(worktreePath?: string): string {
    try {
      return execSync('git rev-parse --short HEAD', {
        cwd: worktreePath || this.repoPath,
        encoding: 'utf-8'
      }).trim();
    } catch {
      return 'unknown';
    }
  }

  getDiff(worktreePath: string): string {
    try {
      return execSync('git diff HEAD', {
        cwd: worktreePath,
        encoding: 'utf-8',
        maxBuffer: 10 * 1024 * 1024 // 10MB
      });
    } catch {
      return '';
    }
  }

  getStatus(worktreePath: string): string {
    try {
      return execSync('git status --short', {
        cwd: worktreePath,
        encoding: 'utf-8'
      });
    } catch {
      return '';
    }
  }

  commitChanges(worktreePath: string, message: string): boolean {
    try {
      execSync('git add -A', { cwd: worktreePath, stdio: 'pipe' });
      execSync(`git commit -m "${message.replace(/"/g, '\\"')}"`, {
        cwd: worktreePath,
        stdio: 'pipe'
      });
      return true;
    } catch {
      return false;
    }
  }
}
