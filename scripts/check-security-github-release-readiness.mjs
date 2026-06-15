#!/usr/bin/env node
import { execFileSync } from 'node:child_process';

function run(command, args, { allowFailure = false } = {}) {
  try {
    return {
      ok: true,
      stdout: execFileSync(command, args, {
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
      }).trim(),
    };
  } catch (error) {
    if (!allowFailure) {
      throw error;
    }
    return {
      ok: false,
      stdout: String(error.stdout || '').trim(),
      stderr: String(error.stderr || '').trim(),
      message: error.message,
      status: error.status ?? 1,
    };
  }
}

function parseRepoFromRemote(remoteUrl) {
  const sshMatch = remoteUrl.match(/github\.com[:/](.+?)(?:\.git)?$/);
  if (!sshMatch) {
    throw new Error(`Unable to parse GitHub repo from remote URL: ${remoteUrl}`);
  }
  return sshMatch[1];
}

function parseArgs(argv) {
  const parsed = {};

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--repo') {
      parsed.repo = argv[index + 1] || '';
      index += 1;
      continue;
    }
    if (arg === '--remote') {
      parsed.remote = argv[index + 1] || '';
      index += 1;
      continue;
    }
    throw new Error(`Unknown argument: ${arg}`);
  }

  return {
    remote: parsed.remote || 'origin',
    repo: parsed.repo || '',
  };
}

const requiredWorkflowFiles = [
  'bundle-desktop-manual.yml',
  'bundle-desktop.yml',
  'bundle-desktop-intel.yml',
];

const { remote, repo: repoOverride } = parseArgs(process.argv.slice(2));
const blockers = [];

const remoteUrlResult = run('git', ['remote', 'get-url', remote]);
const targetRepo = repoOverride || parseRepoFromRemote(remoteUrlResult.stdout);
const currentBranch = run('git', ['branch', '--show-current']).stdout;
const repoMetadataResult = run('gh', ['repo', 'view', targetRepo, '--json', 'defaultBranchRef']);
const repoMetadata = JSON.parse(repoMetadataResult.stdout || '{}');
const defaultBranch = repoMetadata.defaultBranchRef?.name || 'main';
const branchRemoteResult = run('git', ['ls-remote', '--heads', remote, currentBranch], {
  allowFailure: true,
});
const branchOnRemote = branchRemoteResult.ok && branchRemoteResult.stdout.length > 0;
if (!branchOnRemote) {
  blockers.push('current branch is not pushed to the target GitHub repo');
}
const workflowRef = branchOnRemote ? currentBranch : defaultBranch;

const authStatus = run('gh', ['auth', 'status'], { allowFailure: true });
const ghAuthenticated = authStatus.ok;
const hasWorkflowScope =
  ghAuthenticated && /\bworkflow\b/.test(`${authStatus.stdout}\n${authStatus.stderr}`);
if (!ghAuthenticated) {
  blockers.push('gh is not authenticated');
} else if (!hasWorkflowScope) {
  blockers.push('gh token does not show workflow scope');
}

const workflowsResult = run('gh', ['api', `repos/${targetRepo}/actions/workflows`], {
  allowFailure: true,
});
let registeredWorkflowPaths = [];
if (workflowsResult.ok) {
  const workflowsPayload = JSON.parse(workflowsResult.stdout || '{"workflows":[]}');
  registeredWorkflowPaths = Array.isArray(workflowsPayload.workflows)
    ? workflowsPayload.workflows.map((workflow) => workflow.path).filter(Boolean)
    : [];
} else {
  blockers.push('cannot read GitHub Actions workflows from target repo');
}

const contentsResult = run(
  'gh',
  ['api', `repos/${targetRepo}/contents/.github/workflows?ref=${workflowRef}`],
  { allowFailure: true }
);
let workflowFileNames = [];
if (contentsResult.ok) {
  const contentsPayload = JSON.parse(contentsResult.stdout || '[]');
  workflowFileNames = Array.isArray(contentsPayload)
    ? contentsPayload.map((entry) => entry.name).filter(Boolean)
    : [];
} else {
  blockers.push(`cannot read workflow files from target repo ref ${workflowRef}`);
}

const missingWorkflowFiles = requiredWorkflowFiles.filter(
  (workflowFile) => !workflowFileNames.includes(workflowFile)
);
if (missingWorkflowFiles.length > 0) {
  blockers.push(`target repo ref ${workflowRef} is missing workflow files: ${missingWorkflowFiles.join(', ')}`);
}

const registeredRequiredWorkflowPaths = requiredWorkflowFiles.map(
  (workflowFile) => `.github/workflows/${workflowFile}`
);
const missingRegisteredWorkflowPaths = registeredRequiredWorkflowPaths.filter(
  (workflowPath) => !registeredWorkflowPaths.includes(workflowPath)
);
if (missingRegisteredWorkflowPaths.length > 0) {
  blockers.push(
    `GitHub Actions API is not exposing required workflows: ${missingRegisteredWorkflowPaths.join(', ')}`
  );
}

const environmentsResult = run('gh', ['api', `repos/${targetRepo}/environments`], {
  allowFailure: true,
});
let environmentNames = [];
if (environmentsResult.ok) {
  const environmentsPayload = JSON.parse(environmentsResult.stdout || '{"environments":[]}');
  environmentNames = Array.isArray(environmentsPayload.environments)
    ? environmentsPayload.environments.map((environment) => environment.name).filter(Boolean)
    : [];
} else {
  blockers.push('cannot read GitHub environments from target repo');
}

const hasSigningEnvironment = environmentNames.includes('signing');
if (!hasSigningEnvironment) {
  blockers.push('target repo does not expose a signing environment');
}

console.log(`target_repo=${targetRepo}`);
console.log(`target_remote=${remote}`);
console.log(`current_branch=${currentBranch || 'detached'}`);
console.log(`workflow_ref=${workflowRef}`);
console.log(`branch_on_target_repo=${branchOnRemote ? 'yes' : 'no'}`);
console.log(`gh_authenticated=${ghAuthenticated ? 'yes' : 'no'}`);
console.log(`gh_workflow_scope=${hasWorkflowScope ? 'yes' : 'no'}`);
console.log(`workflow_file_count=${workflowFileNames.length}`);
console.log(`workflow_registration_count=${registeredWorkflowPaths.length}`);
console.log(`required_workflow_files_present=${missingWorkflowFiles.length === 0 ? 'yes' : 'no'}`);
console.log(`required_workflows_registered=${missingRegisteredWorkflowPaths.length === 0 ? 'yes' : 'no'}`);
console.log(`signing_environment_present=${hasSigningEnvironment ? 'yes' : 'no'}`);
console.log(`ready_for_remote_signed_rehearsal=${blockers.length === 0 ? 'yes' : 'no'}`);

if (missingWorkflowFiles.length > 0) {
  console.log(`missing_workflow_files=${missingWorkflowFiles.join(',')}`);
} else {
  console.log('missing_workflow_files=none');
}

if (missingRegisteredWorkflowPaths.length > 0) {
  console.log(`missing_registered_workflows=${missingRegisteredWorkflowPaths.join(',')}`);
} else {
  console.log('missing_registered_workflows=none');
}

if (environmentNames.length > 0) {
  console.log(`target_environments=${environmentNames.join(',')}`);
} else {
  console.log('target_environments=none');
}

if (blockers.length > 0) {
  console.error(`release_readiness_blockers=${blockers.join('; ')}`);
  process.exit(1);
}
