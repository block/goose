import { useCallback, useEffect, useMemo, useState } from 'react';
import { AlertCircle, Plus, Zap } from 'lucide-react';
import { getSlashCommands } from '../../api';
import { defineMessages, useIntl } from '../../i18n';
import { useNavigation } from '../../hooks/useNavigation';
import { startNewSession } from '../../sessions';
import type {
  ManagedSkillInvalidCode,
  ImportManagedSkillResult,
  ManagedBundledSkillRecord,
  ManagedLocalSkillRecord,
  ManagedSkillsInventory,
} from '../../security/managedSkills';
import { getManagedLocalVisibleSkillNames } from '../../security/managedSkillsView';
import {
  filterVisibleSkillCommands,
  isSecurityGooseScopedSkillVisibility,
} from '../../security/skillVisibility';
import { errorMessage } from '../../utils/conversionUtils';
import { getSearchShortcutText } from '../../utils/keyboardShortcuts';
import { getInitialWorkingDir } from '../../utils/workingDir';
import { toastError, toastSuccess } from '../../toasts';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { SearchView } from '../conversation/SearchView';
import { Button } from '../ui/button';
import { Card } from '../ui/card';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';
import { Input } from '../ui/input';
import { ScrollArea } from '../ui/scroll-area';
import { Skeleton } from '../ui/skeleton';

const i18n = defineMessages({
  addSkill: {
    id: 'skillsView.addSkill',
    defaultMessage: 'Import Skill',
  },
  adjustSearchTerms: {
    id: 'skillsView.adjustSearchTerms',
    defaultMessage: 'Try adjusting your search terms',
  },
  bundledSkillsTitle: {
    id: 'skillsView.bundledSkillsTitle',
    defaultMessage: 'Bundled Skills',
  },
  cancel: {
    id: 'skillsView.cancel',
    defaultMessage: 'Cancel',
  },
  currentProjectStatus: {
    id: 'skillsView.currentProjectStatus',
    defaultMessage: 'Current project',
  },
  deleteAction: {
    id: 'skillsView.deleteAction',
    defaultMessage: 'Delete',
  },
  deleteSkillSuccessMessage: {
    id: 'skillsView.deleteSkillSuccessMessage',
    defaultMessage: 'Removed "{skillName}" from the current project runtime.',
  },
  deleteSkillSuccessTitle: {
    id: 'skillsView.deleteSkillSuccessTitle',
    defaultMessage: 'Local skill deleted',
  },
  deleteSkillErrorTitle: {
    id: 'skillsView.deleteSkillErrorTitle',
    defaultMessage: 'Could not delete local skill',
  },
  deleteSkillErrorMessage: {
    id: 'skillsView.deleteSkillErrorMessage',
    defaultMessage: 'Could not remove the selected local skill from the current project runtime.',
  },
  deleteSkill: {
    id: 'skillsView.deleteSkill',
    defaultMessage: 'Delete local skill',
  },
  deleteSkillConfirmMessage: {
    id: 'skillsView.deleteSkillConfirmMessage',
    defaultMessage:
      'Delete the current-project skill "{skillName}" from .agents/skills? This does not touch bundled sources or other directories.',
  },
  deleteSkillConfirmTitle: {
    id: 'skillsView.deleteSkillConfirmTitle',
    defaultMessage: 'Delete local skill?',
  },
  errorLoadingSkills: {
    id: 'skillsView.errorLoadingSkills',
    defaultMessage: 'Error Loading Skills',
  },
  detailsAction: {
    id: 'skillsView.detailsAction',
    defaultMessage: 'Details',
  },
  detailDescription: {
    id: 'skillsView.detailDescription',
    defaultMessage:
      'Review this skill description, its active skill folder, and the SKILL.md body used for guidance.',
  },
  detailTitle: {
    id: 'skillsView.detailTitle',
    defaultMessage: '{skillName} details',
  },
  descriptionLabel: {
    id: 'skillsView.descriptionLabel',
    defaultMessage: 'Description',
  },
  builtInDetailDescription: {
    id: 'skillsView.builtInDetailDescription',
    defaultMessage:
      'This is a built-in skill. The app can list it, but there is no current-project skill package to inspect or restore.',
  },
  importConflictMessage: {
    id: 'skillsView.importConflictMessage',
    defaultMessage:
      'A skill named "{skillName}" already exists in the current project. Overwrite it with the selected package?',
  },
  importConflictTitle: {
    id: 'skillsView.importConflictTitle',
    defaultMessage: 'Overwrite existing skill?',
  },
  importSkillErrorTitle: {
    id: 'skillsView.importSkillErrorTitle',
    defaultMessage: 'Skill import failed',
  },
  importSkillErrorMessage: {
    id: 'skillsView.importSkillErrorMessage',
    defaultMessage: 'Could not import the selected skill package.',
  },
  importSkillSuccessMessage: {
    id: 'skillsView.importSkillSuccessMessage',
    defaultMessage: 'Installed "{skillName}" into the current project runtime.',
  },
  importSkillSuccessTitle: {
    id: 'skillsView.importSkillSuccessTitle',
    defaultMessage: 'Skill imported',
  },
  invalidSkillDescription: {
    id: 'skillsView.invalidSkillDescription',
    defaultMessage:
      'This current-project skill package is missing a valid SKILL.md frontmatter name or directory layout.',
  },
  invalidRecoveryLabel: {
    id: 'skillsView.invalidRecoveryLabel',
    defaultMessage: 'Recovery suggestion',
  },
  invalidRecoveryInvalidFrontmatter: {
    id: 'skillsView.invalidRecoveryInvalidFrontmatter',
    defaultMessage: 'Fix the YAML frontmatter syntax in SKILL.md, then re-import or restore the skill.',
  },
  invalidRecoveryMissingFrontmatter: {
    id: 'skillsView.invalidRecoveryMissingFrontmatter',
    defaultMessage:
      'Add YAML frontmatter with at least a non-empty name field to SKILL.md, then re-import or restore the skill.',
  },
  invalidRecoveryMissingName: {
    id: 'skillsView.invalidRecoveryMissingName',
    defaultMessage:
      'Add a non-empty name field to SKILL.md frontmatter, then re-import or restore the skill.',
  },
  invalidRecoveryMissingSkillMd: {
    id: 'skillsView.invalidRecoveryMissingSkillMd',
    defaultMessage: 'Add SKILL.md to the skill folder root, then re-import or restore the skill.',
  },
  invalidRecoveryNameMismatch: {
    id: 'skillsView.invalidRecoveryNameMismatch',
    defaultMessage:
      'Rename the folder or update the frontmatter name so both match, then re-import or restore the skill.',
  },
  invalidRecoveryUnsupported: {
    id: 'skillsView.invalidRecoveryUnsupported',
    defaultMessage: 'Repair the local package layout, then re-import it as a skill folder or zip.',
  },
  invalidSkillPackageStatus: {
    id: 'skillsView.invalidSkillPackageStatus',
    defaultMessage: 'Invalid skill package',
  },
  localSkillsTitle: {
    id: 'skillsView.localSkillsTitle',
    defaultMessage: 'My Skills / Local Skills',
  },
  missingRuntimeStatus: {
    id: 'skillsView.missingRuntimeStatus',
    defaultMessage: 'Missing runtime',
  },
  noMatchingSkills: {
    id: 'skillsView.noMatchingSkills',
    defaultMessage: 'No matching skills found',
  },
  noSecuritySkillsDescription: {
    id: 'skillsView.noSecuritySkillsDescription',
    defaultMessage:
      'This build only surfaces built-in skills, bundled security skills, and local skills installed into the current project.',
  },
  noSkillsDescription: {
    id: 'skillsView.noSkillsDescription',
    defaultMessage:
      'Skills are loaded from SKILL.md files in ~/.config/agents/skills/, .goose/skills/, or other supported directories.',
  },
  noSkillsInstalled: {
    id: 'skillsView.noSkillsInstalled',
    defaultMessage: 'No skills installed',
  },
  noSkillsInstalledDetail: {
    id: 'skillsView.noSkillsInstalledDetail',
    defaultMessage:
      'The current project has no managed local skills yet. Import a skill folder or zip to install it into .agents/skills.',
  },
  noSkillFileAvailable: {
    id: 'skillsView.noSkillFileAvailable',
    defaultMessage: 'No SKILL.md file is available for this skill source.',
  },
  ok: {
    id: 'skillsView.ok',
    defaultMessage: 'OK',
  },
  overrideBundledWarning: {
    id: 'skillsView.overrideBundledWarning',
    defaultMessage:
      'This will replace the current-project runtime copy of a bundled security skill. You can restore the bundled version later.',
  },
  overwriteAction: {
    id: 'skillsView.overwriteAction',
    defaultMessage: 'Overwrite',
  },
  overwriteLocalWarning: {
    id: 'skillsView.overwriteLocalWarning',
    defaultMessage:
      'This will replace the existing current-project skill package in .agents/skills.',
  },
  restoreBundledSkill: {
    id: 'skillsView.restoreBundledSkill',
    defaultMessage: 'Restore bundled version',
  },
  restoreSkillSuccessMessage: {
    id: 'skillsView.restoreSkillSuccessMessage',
    defaultMessage: 'Restored bundled skill "{skillName}" into the current project runtime.',
  },
  restoreSkillSuccessTitle: {
    id: 'skillsView.restoreSkillSuccessTitle',
    defaultMessage: 'Bundled skill restored',
  },
  restoreSkillErrorTitle: {
    id: 'skillsView.restoreSkillErrorTitle',
    defaultMessage: 'Could not restore bundled skill',
  },
  restoreSkillErrorMessage: {
    id: 'skillsView.restoreSkillErrorMessage',
    defaultMessage: 'Could not restore the bundled skill into the current project runtime.',
  },
  restoreShortAction: {
    id: 'skillsView.restoreShortAction',
    defaultMessage: 'Restore',
  },
  restoreConfirmMessage: {
    id: 'skillsView.restoreConfirmMessage',
    defaultMessage:
      'Restore the bundled skill "{skillName}" into the current project runtime and discard the local override?',
  },
  restoreConfirmTitle: {
    id: 'skillsView.restoreConfirmTitle',
    defaultMessage: 'Restore bundled version?',
  },
  scopedBuiltInStatus: {
    id: 'skillsView.scopedBuiltInStatus',
    defaultMessage: 'Built-in',
  },
  revealCurrentProjectFolder: {
    id: 'skillsView.revealCurrentProjectFolder',
    defaultMessage: 'Reveal skill folder',
  },
  revealCurrentProjectFolderSuccessTitle: {
    id: 'skillsView.revealCurrentProjectFolderSuccessTitle',
    defaultMessage: 'Skill folder opened',
  },
  revealCurrentProjectFolderSuccessMessage: {
    id: 'skillsView.revealCurrentProjectFolderSuccessMessage',
    defaultMessage: 'Opened "{directoryPath}" in Finder.',
  },
  revealCurrentProjectFolderErrorTitle: {
    id: 'skillsView.revealCurrentProjectFolderErrorTitle',
    defaultMessage: 'Could not open skill folder',
  },
  revealCurrentProjectFolderErrorMessage: {
    id: 'skillsView.revealCurrentProjectFolderErrorMessage',
    defaultMessage: 'Could not open the selected skill folder.',
  },
  launchPromptConfirm: {
    id: 'skillsView.launchPromptConfirm',
    defaultMessage: 'Start chat',
  },
  launchPromptDescription: {
    id: 'skillsView.launchPromptDescription',
    defaultMessage:
      'This skill requires slash-command input before the app opens the chat session.',
  },
  launchPromptTitle: {
    id: 'skillsView.launchPromptTitle',
    defaultMessage: 'Start {skillName}',
  },
  requiredInputHint: {
    id: 'skillsView.requiredInputHint',
    defaultMessage:
      'This skill needs input before the chat starts. The app will send the slash command with the value you provide here.',
  },
  requiredInputLabel: {
    id: 'skillsView.requiredInputLabel',
    defaultMessage: 'Required input',
  },
  startAction: {
    id: 'skillsView.startAction',
    defaultMessage: 'Start',
  },
  startChatFromSkill: {
    id: 'skillsView.startChatFromSkill',
    defaultMessage: 'Start chat from this skill',
  },
  startChatFromSkillErrorTitle: {
    id: 'skillsView.startChatFromSkillErrorTitle',
    defaultMessage: 'Could not start chat from skill',
  },
  startChatFromSkillErrorMessage: {
    id: 'skillsView.startChatFromSkillErrorMessage',
    defaultMessage: 'Could not start a new chat from the selected skill.',
  },
  startChatFromSkillUnavailable: {
    id: 'skillsView.startChatFromSkillUnavailable',
    defaultMessage:
      'The current / menu has not discovered this skill yet. Reopen the session after importing, restoring, or fixing the skill package, then try again.',
  },
  runtimeDiagnosisInvalid: {
    id: 'skillsView.runtimeDiagnosisInvalid',
    defaultMessage: 'Current-project skill package is invalid and may not be discoverable by the current runtime.',
  },
  runtimeDiagnosisLabel: {
    id: 'skillsView.runtimeDiagnosisLabel',
    defaultMessage: 'Runtime diagnosis',
  },
  runtimeDiagnosisLocalOverride: {
    id: 'skillsView.runtimeDiagnosisLocalOverride',
    defaultMessage:
      'Current-project runtime copy differs from the bundled security source.',
  },
  runtimeDiagnosisMissingRuntime: {
    id: 'skillsView.runtimeDiagnosisMissingRuntime',
    defaultMessage:
      'Bundled security source exists, but the current project runtime copy is missing.',
  },
  skillFolderLabel: {
    id: 'skillsView.skillFolderLabel',
    defaultMessage: 'Skill folder',
  },
  skillFileLabel: {
    id: 'skillsView.skillFileLabel',
    defaultMessage: 'SKILL.md',
  },
  skillFileBodyLabel: {
    id: 'skillsView.skillFileBodyLabel',
    defaultMessage: 'SKILL.md body',
  },
  loadingSkillFile: {
    id: 'skillsView.loadingSkillFile',
    defaultMessage: 'Loading SKILL.md…',
  },
  searchSkillsPlaceholder: {
    id: 'skillsView.searchSkillsPlaceholder',
    defaultMessage: 'Search skills...',
  },
  securitySkillsDescription: {
    id: 'skillsView.securitySkillsDescription',
    defaultMessage:
      'This build shows three skill groups: built-in skills, bundled security skills, and local skills installed into the current project. Skills from other directories such as Claude or Codex are not shown here by default. {shortcut} to search.',
  },
  securitySkillsDiscoveryDescription: {
    id: 'skillsView.securitySkillsDiscoveryDescription',
    defaultMessage:
      'The / menu only shows skills the current runtime has discovered for this session. Newly imported local skills usually appear there after you reopen the session.',
  },
  skillsDescription: {
    id: 'skillsView.skillsDescription',
    defaultMessage: 'View installed skills that extend the app capabilities. {shortcut} to search.',
  },
  skillsTitle: {
    id: 'skillsView.skillsTitle',
    defaultMessage: 'Skills',
  },
  tryAgain: {
    id: 'skillsView.tryAgain',
    defaultMessage: 'Try Again',
  },
  overriddenLocallyStatus: {
    id: 'skillsView.overriddenLocallyStatus',
    defaultMessage: 'Overridden locally',
  },
  unavailableValue: {
    id: 'skillsView.unavailableValue',
    defaultMessage: 'Unavailable',
  },
});

interface CommandSkillEntry {
  description: string;
  inputHint?: string;
  name: string;
}

interface SkillCardEntry {
  action: 'delete' | 'restore' | null;
  bundledSourceDir?: string;
  declaredName?: string;
  description: string;
  detailStatus: 'builtin' | 'bundled-security' | 'invalid' | 'local-custom' | 'local-override' | 'missing-runtime';
  invalidDetail?: string;
  invalidCode?: ManagedSkillInvalidCode;
  key: string;
  name: string;
  runtimeDir?: string;
  slashInputHint?: string;
  slashVisible: boolean;
  skillMdPath?: string;
  statusLabel?: string;
}

interface SkillSections {
  bundled: SkillCardEntry[];
  local: SkillCardEntry[];
}

interface SelectedSkillDetails {
  error?: string;
  loading: boolean;
  skill: SkillCardEntry;
  skillMdContents?: string;
}

interface PendingSkillLaunch {
  inputHint: string;
  skill: SkillCardEntry;
}

function shouldShowRuntimeDiagnosis(skill: SkillCardEntry): boolean {
  return (
    skill.detailStatus === 'missing-runtime' ||
    skill.detailStatus === 'local-override' ||
    skill.detailStatus === 'invalid'
  );
}

function getSkillBody(contents: string): string {
  const normalized = contents.replace(/\r\n/g, '\n');
  const frontmatterMatch = normalized.match(/^---\n[\s\S]*?\n---(?:\n|$)/);
  const body = frontmatterMatch ? normalized.slice(frontmatterMatch[0].length) : normalized;
  return body.trim();
}

function SkillSkeleton() {
  return (
    <Card className="p-2 mb-2 bg-background-primary">
      <div className="flex justify-between items-start gap-4">
        <div className="min-w-0 flex-1">
          <Skeleton className="h-5 w-3/4 mb-2" />
          <Skeleton className="h-4 w-full" />
        </div>
      </div>
    </Card>
  );
}

function StatusPill({ children }: { children: string }) {
  return (
    <span className="inline-flex items-center rounded-full border border-border-subtle px-2 py-0.5 text-xs text-text-secondary">
      {children}
    </span>
  );
}

function SkillItem({
  deleteLabel,
  detailsLabel,
  onDelete,
  onDetails,
  onRestore,
  restoreLabel,
  startLabel,
  onStart,
  skill,
}: {
  deleteLabel: string;
  detailsLabel: string;
  onDelete: (skillName: string) => Promise<void>;
  onDetails: (skill: SkillCardEntry) => void;
  onRestore: (skillName: string) => Promise<void>;
  restoreLabel: string;
  startLabel: string;
  onStart: (skill: SkillCardEntry) => void;
  skill: SkillCardEntry;
}) {
  return (
    <Card className="py-2 px-4 mb-2 bg-background-primary border-none hover:bg-background-secondary transition-all duration-150">
      <div className="flex justify-between items-start gap-4">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2 mb-1">
            <h3 className="text-base truncate">{skill.name}</h3>
            {skill.statusLabel ? <StatusPill>{skill.statusLabel}</StatusPill> : null}
          </div>
          <p className="text-text-secondary text-sm line-clamp-2">{skill.description}</p>
          {skill.invalidDetail ? (
            <p className="text-text-secondary/80 text-xs mt-2 line-clamp-2">{skill.invalidDetail}</p>
          ) : null}
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {skill.slashVisible ? (
            <Button variant="default" size="sm" onClick={() => onStart(skill)}>
              {startLabel}
            </Button>
          ) : null}
          <Button variant="outline" size="sm" onClick={() => onDetails(skill)}>
            {detailsLabel}
          </Button>
          {skill.action === 'delete' ? (
            <Button variant="outline" size="sm" onClick={() => void onDelete(skill.name)}>
              {deleteLabel}
            </Button>
          ) : null}
          {skill.action === 'restore' ? (
            <Button variant="outline" size="sm" onClick={() => void onRestore(skill.name)}>
              {restoreLabel}
            </Button>
          ) : null}
        </div>
      </div>
    </Card>
  );
}

function isSkillMatch(skill: SkillCardEntry, searchTerm: string): boolean {
  if (!searchTerm) {
    return true;
  }

  const searchLower = searchTerm.toLowerCase();
  return (
    skill.name.toLowerCase().includes(searchLower) ||
    skill.description.toLowerCase().includes(searchLower) ||
    skill.invalidDetail?.toLowerCase().includes(searchLower) === true
  );
}

function getBundledStatusLabel(
  intl: ReturnType<typeof useIntl>,
  bundledSkill: ManagedBundledSkillRecord
): string | undefined {
  switch (bundledSkill.status) {
    case 'invalid':
      return intl.formatMessage(i18n.invalidSkillPackageStatus);
    case 'local-override':
      return intl.formatMessage(i18n.overriddenLocallyStatus);
    case 'missing-runtime':
      return intl.formatMessage(i18n.missingRuntimeStatus);
    default:
      return undefined;
  }
}

function getLocalStatusLabel(
  intl: ReturnType<typeof useIntl>,
  localSkill: ManagedLocalSkillRecord
): string | undefined {
  switch (localSkill.status) {
    case 'invalid':
      return intl.formatMessage(i18n.invalidSkillPackageStatus);
    case 'local-custom':
      return intl.formatMessage(i18n.currentProjectStatus);
    case 'local-override':
      return intl.formatMessage(i18n.overriddenLocallyStatus);
    default:
      return undefined;
  }
}

function getRuntimeDiagnosis(
  intl: ReturnType<typeof useIntl>,
  skill: SkillCardEntry
): string {
  switch (skill.detailStatus) {
    case 'missing-runtime':
      return intl.formatMessage(i18n.runtimeDiagnosisMissingRuntime);
    case 'local-override':
      return intl.formatMessage(i18n.runtimeDiagnosisLocalOverride);
    case 'invalid':
      return intl.formatMessage(i18n.runtimeDiagnosisInvalid);
    default:
      return '';
  }
}

function getInvalidRecoveryHint(
  intl: ReturnType<typeof useIntl>,
  invalidCode?: ManagedSkillInvalidCode
): string | undefined {
  switch (invalidCode) {
    case 'invalid_frontmatter':
      return intl.formatMessage(i18n.invalidRecoveryInvalidFrontmatter);
    case 'missing_frontmatter':
      return intl.formatMessage(i18n.invalidRecoveryMissingFrontmatter);
    case 'missing_name':
      return intl.formatMessage(i18n.invalidRecoveryMissingName);
    case 'missing_skill_md':
      return intl.formatMessage(i18n.invalidRecoveryMissingSkillMd);
    case 'name_mismatch':
      return intl.formatMessage(i18n.invalidRecoveryNameMismatch);
    case 'invalid_archive':
    case 'unsupported_source':
      return intl.formatMessage(i18n.invalidRecoveryUnsupported);
    default:
      return undefined;
  }
}

export default function SkillsView() {
  const intl = useIntl();
  const setView = useNavigation();
  const currentWorkingDir = getInitialWorkingDir();
  const [commandSkills, setCommandSkills] = useState<CommandSkillEntry[]>([]);
  const [managedInventory, setManagedInventory] = useState<ManagedSkillsInventory | null>(null);
  const [loading, setLoading] = useState(true);
  const [showSkeleton, setShowSkeleton] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showContent, setShowContent] = useState(false);
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedSkillDetails, setSelectedSkillDetails] = useState<SelectedSkillDetails | null>(null);
  const [launchingSkillName, setLaunchingSkillName] = useState<string | null>(null);
  const [pendingSkillLaunch, setPendingSkillLaunch] = useState<PendingSkillLaunch | null>(null);
  const [pendingSkillInput, setPendingSkillInput] = useState('');
  const scopedSkillVisibility = isSecurityGooseScopedSkillVisibility();

  const loadSkills = useCallback(async () => {
    try {
      setLoading(true);
      setShowSkeleton(true);
      setShowContent(false);
      setError(null);

      const [response, inventory] = await Promise.all([
        getSlashCommands({
          query: { working_dir: currentWorkingDir },
          throwOnError: true,
        }),
        window.electron.listManagedSkills(currentWorkingDir),
      ]);

      const visibleSkills = filterVisibleSkillCommands(
        response.data?.commands ?? [],
        undefined,
        getManagedLocalVisibleSkillNames(inventory)
      )
        .filter((command) => command.command_type === 'Skill')
        .map((command) => ({
          description: command.help,
          inputHint: command.input_hint ?? undefined,
          name: command.command,
        }));

      setCommandSkills(visibleSkills);
      setManagedInventory(inventory);
    } catch (err) {
      setError(errorMessage(err, 'Failed to load skills'));
    } finally {
      setLoading(false);
    }
  }, [currentWorkingDir]);

  const visibleSkillNames = useMemo(
    () => new Set(commandSkills.map((skill) => skill.name)),
    [commandSkills]
  );
  const commandSkillMap = useMemo(
    () => new Map(commandSkills.map((skill) => [skill.name, skill])),
    [commandSkills]
  );

  useEffect(() => {
    void loadSkills();
  }, [loadSkills]);

  useEffect(() => {
    if (!loading && showSkeleton) {
      const timer = setTimeout(() => {
        setShowSkeleton(false);
        setTimeout(() => setShowContent(true), 50);
      }, 300);
      return () => clearTimeout(timer);
    }
    return undefined;
  }, [loading, showSkeleton]);

  const skillSections = useMemo<SkillSections>(() => {
    if (!managedInventory) {
      return {
        bundled: [],
        local: [],
      };
    }

    const bundledIds = new Set(managedInventory.bundledSkills.map((skill) => skill.id));
    const localIds = new Set(managedInventory.localSkills.map((skill) => skill.id));
    const builtinSkills: SkillCardEntry[] = commandSkills
      .filter((skill) => !bundledIds.has(skill.name) && !localIds.has(skill.name))
      .map((skill) => ({
        action: null,
        declaredName: skill.name,
        description: skill.description,
        detailStatus: 'builtin',
        key: `builtin:${skill.name}`,
        name: skill.name,
        slashInputHint: skill.inputHint,
        slashVisible: true,
        statusLabel: intl.formatMessage(i18n.scopedBuiltInStatus),
      }));
    const bundledSkills = managedInventory.bundledSkills.map<SkillCardEntry>((skill) => ({
      action:
        skill.status === 'bundled-security'
          ? null
          : skill.status === 'missing-runtime' || skill.status === 'local-override' || skill.status === 'invalid'
            ? 'restore'
            : null,
      bundledSourceDir: skill.sourceDir,
      declaredName: skill.declaredName,
      description: skill.description,
      detailStatus: skill.status,
      invalidDetail: skill.invalidDetail,
      invalidCode: skill.invalidCode,
      key: `bundled:${skill.id}`,
      name: skill.id,
      runtimeDir: skill.runtimeDir,
      slashInputHint: commandSkillMap.get(skill.id)?.inputHint,
      slashVisible: visibleSkillNames.has(skill.id),
      skillMdPath:
        skill.status === 'missing-runtime'
          ? `${skill.sourceDir}/SKILL.md`
          : `${skill.runtimeDir}/SKILL.md`,
      statusLabel: getBundledStatusLabel(intl, skill),
    }));
    const localSkills = managedInventory.localSkills.map<SkillCardEntry>((skill) => ({
      action: skill.status === 'local-custom' || skill.status === 'invalid' ? 'delete' : 'restore',
      bundledSourceDir: skill.bundledSkillId
        ? managedInventory.bundledSkills.find((bundledSkill) => bundledSkill.id === skill.bundledSkillId)?.sourceDir
        : managedInventory.bundledSkills.find((bundledSkill) => bundledSkill.id === skill.id)?.sourceDir,
      declaredName: skill.declaredName,
      description: skill.description || intl.formatMessage(i18n.invalidSkillDescription),
      detailStatus: skill.status,
      invalidDetail: skill.invalidDetail,
      invalidCode: skill.invalidCode,
      key: `local:${skill.id}`,
      name: skill.id,
      runtimeDir: skill.runtimeDir,
      slashInputHint: commandSkillMap.get(skill.id)?.inputHint,
      slashVisible: visibleSkillNames.has(skill.id),
      skillMdPath: `${skill.runtimeDir}/SKILL.md`,
      statusLabel: getLocalStatusLabel(intl, skill),
    }));

    return {
      bundled: [...builtinSkills, ...bundledSkills].filter((skill) => isSkillMatch(skill, searchTerm)),
      local: localSkills.filter((skill) => isSkillMatch(skill, searchTerm)),
    };
  }, [commandSkillMap, commandSkills, intl, managedInventory, searchTerm, visibleSkillNames]);

  const selectedSkill = selectedSkillDetails?.skill;

  useEffect(() => {
    if (!selectedSkill) {
      return;
    }

    const skillFilePath = selectedSkill.skillMdPath;
    if (!skillFilePath) {
      setSelectedSkillDetails((current) =>
        current
          ? {
              ...current,
              error:
                selectedSkill.detailStatus === 'builtin'
                  ? intl.formatMessage(i18n.builtInDetailDescription)
                  : intl.formatMessage(i18n.noSkillFileAvailable),
              loading: false,
            }
          : current
      );
      return;
    }

    let cancelled = false;
    setSelectedSkillDetails((current) =>
      current
        ? {
            ...current,
            error: undefined,
            loading: true,
            skillMdContents: undefined,
          }
        : current
    );

    void window.electron.readFile(skillFilePath).then((response) => {
      if (cancelled) {
        return;
      }

      setSelectedSkillDetails((current) =>
        current
          ? {
              ...current,
              error:
                response.found && !response.error
                  ? undefined
                  : response.error || intl.formatMessage(i18n.noSkillFileAvailable),
              loading: false,
              skillMdContents: response.found && !response.error ? response.file : undefined,
            }
          : current
      );
    });

    return () => {
      cancelled = true;
    };
  }, [intl, selectedSkill]);

  const showImportError = useCallback(
    async (result: Extract<ImportManagedSkillResult, { status: 'invalid' }>) => {
      await window.electron.showMessageBox({
        buttons: [intl.formatMessage(i18n.ok)],
        detail: result.reason,
        message: intl.formatMessage(i18n.importSkillErrorTitle),
        title: intl.formatMessage(i18n.importSkillErrorTitle),
        type: 'error',
      });
    },
    [intl]
  );

  const showActionError = useCallback(
    (title: string, fallbackMessage: string, err: unknown) => {
      const msg = errorMessage(err, fallbackMessage);
      toastError({
        msg,
        title,
        traceback: msg,
      });
    },
    []
  );

  const runImportSkill = useCallback(
    async (sourcePath: string, overwrite = false): Promise<void> => {
      const result = await window.electron.importManagedSkill({
        overwrite,
        sourcePath,
        workingDir: currentWorkingDir,
      });

      if (result.status === 'installed') {
        await loadSkills();
        toastSuccess({
          msg: intl.formatMessage(i18n.importSkillSuccessMessage, { skillName: result.skillId }),
          title: intl.formatMessage(i18n.importSkillSuccessTitle),
        });
        return;
      }

      if (result.status === 'invalid') {
        await showImportError(result);
        return;
      }

      const bundledConflict = result.existingStatus === 'bundled-security';
      const confirm = await window.electron.showMessageBox({
        buttons: [intl.formatMessage(i18n.cancel), intl.formatMessage(i18n.overwriteAction)],
        defaultId: 1,
        detail: bundledConflict
          ? intl.formatMessage(i18n.overrideBundledWarning)
          : intl.formatMessage(i18n.overwriteLocalWarning),
        message: intl.formatMessage(i18n.importConflictMessage, { skillName: result.skillId }),
        title: intl.formatMessage(i18n.importConflictTitle),
        type: 'warning',
      });

      if (confirm.response === 1) {
        await runImportSkill(sourcePath, true);
      }
    },
    [currentWorkingDir, intl, loadSkills, showImportError]
  );

  const handleImportSkill = useCallback(async () => {
    try {
      const selectedPath = await window.electron.selectFileOrDirectory(currentWorkingDir);
      if (!selectedPath) {
        return;
      }

      await runImportSkill(selectedPath);
    } catch (err) {
      showActionError(
        intl.formatMessage(i18n.importSkillErrorTitle),
        intl.formatMessage(i18n.importSkillErrorMessage),
        err
      );
    }
  }, [currentWorkingDir, intl, runImportSkill, showActionError]);

  const handleDeleteSkill = useCallback(
    async (skillName: string) => {
      try {
        const confirm = await window.electron.showMessageBox({
          buttons: [intl.formatMessage(i18n.cancel), intl.formatMessage(i18n.deleteAction)],
          defaultId: 1,
          message: intl.formatMessage(i18n.deleteSkillConfirmMessage, { skillName }),
          title: intl.formatMessage(i18n.deleteSkillConfirmTitle),
          type: 'warning',
        });

        if (confirm.response !== 1) {
          return;
        }

        await window.electron.deleteManagedLocalSkill(currentWorkingDir, skillName);
        await loadSkills();
        toastSuccess({
          msg: intl.formatMessage(i18n.deleteSkillSuccessMessage, { skillName }),
          title: intl.formatMessage(i18n.deleteSkillSuccessTitle),
        });
      } catch (err) {
        showActionError(
          intl.formatMessage(i18n.deleteSkillErrorTitle),
          intl.formatMessage(i18n.deleteSkillErrorMessage),
          err
        );
      }
    },
    [currentWorkingDir, intl, loadSkills, showActionError]
  );

  const handleRestoreSkill = useCallback(
    async (skillName: string) => {
      try {
        const confirm = await window.electron.showMessageBox({
          buttons: [intl.formatMessage(i18n.cancel), intl.formatMessage(i18n.restoreBundledSkill)],
          defaultId: 1,
          message: intl.formatMessage(i18n.restoreConfirmMessage, { skillName }),
          title: intl.formatMessage(i18n.restoreConfirmTitle),
          type: 'warning',
        });

        if (confirm.response !== 1) {
          return;
        }

        await window.electron.restoreBundledSkill(currentWorkingDir, skillName);
        await loadSkills();
        toastSuccess({
          msg: intl.formatMessage(i18n.restoreSkillSuccessMessage, { skillName }),
          title: intl.formatMessage(i18n.restoreSkillSuccessTitle),
        });
      } catch (err) {
        showActionError(
          intl.formatMessage(i18n.restoreSkillErrorTitle),
          intl.formatMessage(i18n.restoreSkillErrorMessage),
          err
        );
      }
    },
    [currentWorkingDir, intl, loadSkills, showActionError]
  );

  const handleOpenDetails = useCallback((skill: SkillCardEntry) => {
    setSelectedSkillDetails({
      loading: true,
      skill,
    });
  }, []);

  const handleRevealSkillFolder = useCallback(
    async (runtimeDir: string) => {
      const result = await window.electron.openDirectoryInExplorer(runtimeDir);
      if (result.opened) {
        toastSuccess({
          msg: intl.formatMessage(i18n.revealCurrentProjectFolderSuccessMessage, {
            directoryPath: runtimeDir,
          }),
          title: intl.formatMessage(i18n.revealCurrentProjectFolderSuccessTitle),
        });
        return;
      }

      const msg =
        result.error || intl.formatMessage(i18n.revealCurrentProjectFolderErrorMessage);
      toastError({
        msg,
        title: intl.formatMessage(i18n.revealCurrentProjectFolderErrorTitle),
        traceback: msg,
      });
    },
    [intl]
  );

  const handleStartSkillChat = useCallback(
    async (skillName: string, args?: string) => {
      setLaunchingSkillName(skillName);
      try {
        const trimmedArgs = args?.trim();
        const initialCommand = trimmedArgs ? `/${skillName} ${trimmedArgs}` : `/${skillName}`;

        await startNewSession(initialCommand, setView, currentWorkingDir);
        setPendingSkillLaunch(null);
        setPendingSkillInput('');
        setSelectedSkillDetails(null);
      } catch (err) {
        showActionError(
          intl.formatMessage(i18n.startChatFromSkillErrorTitle),
          intl.formatMessage(i18n.startChatFromSkillErrorMessage),
          err
        );
      } finally {
        setLaunchingSkillName((current) => (current === skillName ? null : current));
      }
    },
    [currentWorkingDir, intl, setView, showActionError]
  );

  const handleRequestStartSkillChat = useCallback(
    (skill: SkillCardEntry) => {
      if (!skill.slashVisible) {
        return;
      }

      if (skill.slashInputHint) {
        setPendingSkillLaunch({
          inputHint: skill.slashInputHint,
          skill,
        });
        setPendingSkillInput('');
        setSelectedSkillDetails((current) =>
          current?.skill.key === skill.key ? null : current
        );
        return;
      }

      void handleStartSkillChat(skill.name);
    },
    [handleStartSkillChat]
  );

  const renderSkillSection = useCallback(
    (title: string, skills: SkillCardEntry[], emptyMessage?: string) => (
      <section className="mb-6" key={title}>
        <div className="flex items-center justify-between mb-2">
          <h2 className="text-lg font-medium">{title}</h2>
        </div>
        {skills.length > 0 ? (
          skills.map((skill) => (
            <SkillItem
              key={skill.key}
              deleteLabel={intl.formatMessage(i18n.deleteAction)}
              detailsLabel={intl.formatMessage(i18n.detailsAction)}
              onDelete={handleDeleteSkill}
              onDetails={handleOpenDetails}
              onRestore={handleRestoreSkill}
              restoreLabel={intl.formatMessage(i18n.restoreShortAction)}
              startLabel={intl.formatMessage(i18n.startAction)}
              onStart={handleRequestStartSkillChat}
              skill={skill}
            />
          ))
        ) : emptyMessage ? (
          <Card className="py-3 px-4 mb-2 bg-background-primary border-none">
            <p className="text-sm text-text-secondary">{emptyMessage}</p>
          </Card>
        ) : null}
      </section>
    ),
    [handleDeleteSkill, handleOpenDetails, handleRequestStartSkillChat, handleRestoreSkill, intl]
  );

  const renderContent = () => {
    if (loading || showSkeleton) {
      return (
        <div className="space-y-2">
          <SkillSkeleton />
          <SkillSkeleton />
          <SkillSkeleton />
        </div>
      );
    }

    if (error) {
      return (
        <div className="flex flex-col items-center justify-center h-full text-text-secondary">
          <AlertCircle className="h-12 w-12 text-red-500 mb-4" />
          <p className="text-lg mb-2">{intl.formatMessage(i18n.errorLoadingSkills)}</p>
          <p className="text-sm text-center mb-4">{error}</p>
          <Button onClick={() => void loadSkills()} variant="default">
            {intl.formatMessage(i18n.tryAgain)}
          </Button>
        </div>
      );
    }

    if (
      commandSkills.length === 0 &&
      (!managedInventory ||
        (managedInventory.localSkills.length === 0 && managedInventory.bundledSkills.length === 0))
    ) {
      return (
        <div className="flex flex-col justify-center pt-2 h-full">
          <p className="text-lg">{intl.formatMessage(i18n.noSkillsInstalled)}</p>
          <p className="text-sm text-text-secondary">
            {intl.formatMessage(
              scopedSkillVisibility ? i18n.noSecuritySkillsDescription : i18n.noSkillsDescription
            )}
          </p>
          <p className="text-sm text-text-secondary mt-2">
            {intl.formatMessage(i18n.noSkillsInstalledDetail)}
          </p>
        </div>
      );
    }

    if (skillSections.bundled.length === 0 && skillSections.local.length === 0 && searchTerm) {
      return (
        <div className="flex flex-col items-center justify-center h-full text-text-secondary mt-4">
          <Zap className="h-12 w-12 mb-4" />
          <p className="text-lg mb-2">{intl.formatMessage(i18n.noMatchingSkills)}</p>
          <p className="text-sm">{intl.formatMessage(i18n.adjustSearchTerms)}</p>
        </div>
      );
    }

    return (
      <div className="space-y-2">
        {renderSkillSection(intl.formatMessage(i18n.bundledSkillsTitle), skillSections.bundled)}
        {renderSkillSection(
          intl.formatMessage(i18n.localSkillsTitle),
          skillSections.local,
          intl.formatMessage(i18n.noSkillsInstalledDetail)
        )}
      </div>
    );
  };

  const activeSkill = selectedSkill;
  const invalidRecoveryHint = activeSkill
    ? getInvalidRecoveryHint(intl, activeSkill.invalidCode)
    : undefined;
  const showRuntimeDiagnosisCard = activeSkill ? shouldShowRuntimeDiagnosis(activeSkill) : false;
  const canStartChatFromSkill = activeSkill ? visibleSkillNames.has(activeSkill.name) : false;
  const isStartingActiveSkill = activeSkill ? launchingSkillName === activeSkill.name : false;
  const launchPromptInput = pendingSkillInput.trim();

  return (
    <MainPanelLayout>
      <div className="flex-1 flex flex-col min-h-0">
        <div className="bg-background-primary px-8 pb-8 pt-16">
          <div className="flex flex-col page-transition">
            <div className="flex justify-between items-center mb-1">
              <h1 className="text-4xl font-light">{intl.formatMessage(i18n.skillsTitle)}</h1>
              <Button
                variant="outline"
                size="sm"
                className="flex items-center gap-2"
                onClick={() => void handleImportSkill()}
              >
                <Plus className="w-4 h-4" />
                {intl.formatMessage(i18n.addSkill)}
              </Button>
            </div>
            <p className="text-sm text-text-secondary mb-1">
              {intl.formatMessage(
                scopedSkillVisibility ? i18n.securitySkillsDescription : i18n.skillsDescription,
                {
                  shortcut: getSearchShortcutText(),
                }
              )}
            </p>
            {scopedSkillVisibility ? (
              <p className="text-sm text-text-secondary mb-1">
                {intl.formatMessage(i18n.securitySkillsDiscoveryDescription)}
              </p>
            ) : null}
          </div>
        </div>

        <div className="flex-1 min-h-0 relative px-8">
          <ScrollArea className="h-full">
            <SearchView
              onSearch={(term) => setSearchTerm(term)}
              placeholder={intl.formatMessage(i18n.searchSkillsPlaceholder)}
            >
              <div
                className={`h-full relative transition-all duration-300 ${showContent ? 'opacity-100' : 'opacity-0'}`}
              >
                {renderContent()}
              </div>
            </SearchView>
          </ScrollArea>
        </div>
      </div>

      <Dialog
        open={selectedSkillDetails !== null}
        onOpenChange={(open) => {
          if (!open) {
            setSelectedSkillDetails(null);
          }
        }}
      >
        <DialogContent className="sm:max-w-3xl max-h-[85vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>
              {activeSkill
                ? intl.formatMessage(i18n.detailTitle, { skillName: activeSkill.name })
                : intl.formatMessage(i18n.skillsTitle)}
            </DialogTitle>
            <DialogDescription>{intl.formatMessage(i18n.detailDescription)}</DialogDescription>
          </DialogHeader>

          {activeSkill ? (
            <div className="space-y-4">
              <div
                className={`grid gap-4 ${showRuntimeDiagnosisCard ? 'md:grid-cols-2' : 'grid-cols-1'}`}
                data-testid="skill-detail-layout"
              >
                <Card className="px-4 py-3 border-none bg-background-secondary/50">
                  <div className="space-y-3 text-sm">
                    <div>
                      <p className="text-text-secondary">{intl.formatMessage(i18n.descriptionLabel)}</p>
                      <p className="mt-1 whitespace-pre-wrap leading-6">
                        {activeSkill.description || intl.formatMessage(i18n.unavailableValue)}
                      </p>
                    </div>
                  </div>
                </Card>

                {showRuntimeDiagnosisCard ? (
                  <Card className="px-4 py-3 border-none bg-background-secondary/50">
                    <div className="space-y-3 text-sm">
                      <div>
                        <p className="text-text-secondary">
                          {intl.formatMessage(i18n.runtimeDiagnosisLabel)}
                        </p>
                        <p className="mt-1 whitespace-pre-wrap">
                          {getRuntimeDiagnosis(intl, activeSkill)}
                        </p>
                      </div>
                      {activeSkill.invalidDetail ? (
                        <div>
                          <p className="text-text-secondary">
                            {intl.formatMessage(i18n.invalidSkillPackageStatus)}
                          </p>
                          <p className="mt-1 whitespace-pre-wrap">{activeSkill.invalidDetail}</p>
                        </div>
                      ) : null}
                      {invalidRecoveryHint ? (
                        <div>
                          <p className="text-text-secondary">
                            {intl.formatMessage(i18n.invalidRecoveryLabel)}
                          </p>
                          <p className="mt-1 whitespace-pre-wrap">{invalidRecoveryHint}</p>
                        </div>
                      ) : null}
                    </div>
                  </Card>
                ) : null}
              </div>

              <Card className="px-4 py-3 border-none bg-background-secondary/50">
                <div className="space-y-3 text-sm">
                  <div>
                    <p className="text-text-secondary">{intl.formatMessage(i18n.skillFolderLabel)}</p>
                    <p className="mt-1 break-all">
                      {activeSkill.runtimeDir || intl.formatMessage(i18n.unavailableValue)}
                    </p>
                  </div>

                  <div className="flex flex-wrap gap-2">
                    {activeSkill ? (
                      <Button
                        variant="default"
                        size="sm"
                        disabled={!canStartChatFromSkill || isStartingActiveSkill}
                        onClick={() => handleRequestStartSkillChat(activeSkill)}
                      >
                        {intl.formatMessage(i18n.startChatFromSkill)}
                      </Button>
                    ) : null}
                    {activeSkill.runtimeDir ? (
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => void handleRevealSkillFolder(activeSkill.runtimeDir!)}
                      >
                        {intl.formatMessage(i18n.revealCurrentProjectFolder)}
                      </Button>
                    ) : null}
                  </div>
                  {!canStartChatFromSkill ? (
                    <p className="text-xs text-text-secondary whitespace-pre-wrap">
                      {intl.formatMessage(i18n.startChatFromSkillUnavailable)}
                    </p>
                  ) : activeSkill?.slashInputHint ? (
                    <div className="space-y-1">
                      <p className="text-xs text-text-secondary">
                        {intl.formatMessage(i18n.requiredInputLabel)}
                      </p>
                      <p className="text-xs text-text-secondary whitespace-pre-wrap">
                        {activeSkill.slashInputHint}
                      </p>
                      <p className="text-xs text-text-secondary whitespace-pre-wrap">
                        {intl.formatMessage(i18n.requiredInputHint)}
                      </p>
                    </div>
                  ) : null}
                </div>
              </Card>

              <Card className="px-4 py-3 border-none bg-background-secondary/50">
                <div className="space-y-3">
                  <div>
                    <p className="text-sm text-text-secondary">
                      {intl.formatMessage(i18n.skillFileLabel)}
                    </p>
                    <p className="mt-1 text-sm break-all">
                      {activeSkill.skillMdPath || intl.formatMessage(i18n.unavailableValue)}
                    </p>
                  </div>
                  {selectedSkillDetails.loading ? (
                    <p className="text-sm text-text-secondary">
                      {intl.formatMessage(i18n.loadingSkillFile)}
                    </p>
                  ) : selectedSkillDetails.error ? (
                    <p className="text-sm text-text-secondary whitespace-pre-wrap">
                      {selectedSkillDetails.error}
                    </p>
                  ) : (
                    <div className="space-y-2">
                      <p className="text-sm text-text-secondary">
                        {intl.formatMessage(i18n.skillFileBodyLabel)}
                      </p>
                      <pre className="max-h-80 overflow-auto rounded-lg bg-background-primary p-3 text-xs leading-6 whitespace-pre-wrap break-words">
                        {getSkillBody(selectedSkillDetails.skillMdContents ?? '')}
                      </pre>
                    </div>
                  )}
                </div>
              </Card>
            </div>
          ) : null}
        </DialogContent>
      </Dialog>

      <Dialog
        open={pendingSkillLaunch !== null}
        onOpenChange={(open) => {
          if (!open) {
            setPendingSkillLaunch(null);
            setPendingSkillInput('');
          }
        }}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>
              {pendingSkillLaunch
                ? intl.formatMessage(i18n.launchPromptTitle, {
                    skillName: pendingSkillLaunch.skill.name,
                  })
                : intl.formatMessage(i18n.skillsTitle)}
            </DialogTitle>
            <DialogDescription>
              {intl.formatMessage(i18n.launchPromptDescription)}
            </DialogDescription>
          </DialogHeader>

          {pendingSkillLaunch ? (
            <div className="space-y-3">
              <div className="space-y-2">
                <label className="text-sm text-text-secondary" htmlFor="skill-launch-input">
                  {intl.formatMessage(i18n.requiredInputLabel)}
                </label>
                <Input
                  id="skill-launch-input"
                  value={pendingSkillInput}
                  onChange={(event) => setPendingSkillInput(event.target.value)}
                  placeholder={pendingSkillLaunch.inputHint}
                  autoFocus
                />
              </div>
              <p className="text-xs text-text-secondary whitespace-pre-wrap">
                {pendingSkillLaunch.inputHint}
              </p>
            </div>
          ) : null}

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setPendingSkillLaunch(null);
                setPendingSkillInput('');
              }}
            >
              {intl.formatMessage(i18n.cancel)}
            </Button>
            <Button
              onClick={() =>
                pendingSkillLaunch
                  ? void handleStartSkillChat(
                      pendingSkillLaunch.skill.name,
                      pendingSkillInput
                    )
                  : undefined
              }
              disabled={!launchPromptInput || !pendingSkillLaunch}
            >
              {intl.formatMessage(i18n.launchPromptConfirm)}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </MainPanelLayout>
  );
}
