/**
 * @vitest-environment jsdom
 */
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SkillsView from './SkillsView';
import { IntlTestWrapper } from '../../i18n/test-utils';

const mocks = vi.hoisted(() => ({
  getSlashCommands: vi.fn(),
  setView: vi.fn(),
  startNewSession: vi.fn(),
  toastSuccess: vi.fn(),
  toastError: vi.fn(),
}));

vi.mock('../../api', async () => {
  const actual = await vi.importActual<typeof import('../../api')>('../../api');

  return {
    ...actual,
    getSlashCommands: mocks.getSlashCommands,
  };
});

vi.mock('../../toasts', () => ({
  toastSuccess: mocks.toastSuccess,
  toastError: mocks.toastError,
}));

vi.mock('../../sessions', async () => {
  const actual = await vi.importActual<typeof import('../../sessions')>('../../sessions');

  return {
    ...actual,
    startNewSession: mocks.startNewSession,
  };
});

vi.mock('../../hooks/useNavigation', () => ({
  useNavigation: () => mocks.setView,
}));

vi.mock('../../utils/workingDir', () => ({
  getInitialWorkingDir: () => '/tmp/security-goose',
}));

describe('SkillsView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.startNewSession.mockResolvedValue({ id: 'session-1' });
    const listManagedSkills = vi
      .fn()
      .mockResolvedValueOnce({
        workingDir: '/tmp/security-goose',
        bundledSkillRoot: '/repo/distro/security-cn/skills',
        runtimeSkillRoot: '/tmp/security-goose/.agents/skills',
        bundledSkills: [
          {
            declaredName: 'vuln-triage',
            id: 'vuln-triage',
            description: 'Security Goose skill',
            runtimeDir: '/tmp/security-goose/.agents/skills/vuln-triage',
            sourceDir: '/repo/distro/security-cn/skills/vuln-triage',
            status: 'local-override',
          },
          {
            declaredName: 'report-writing',
            id: 'report-writing',
            description: 'Bundled runtime missing',
            runtimeDir: '/tmp/security-goose/.agents/skills/report-writing',
            sourceDir: '/repo/distro/security-cn/skills/report-writing',
            status: 'missing-runtime',
          },
        ],
        localSkills: [
          {
            declaredName: 'local-investigation',
            id: 'local-investigation',
            description: 'Current project local skill',
            runtimeDir: '/tmp/security-goose/.agents/skills/local-investigation',
            status: 'local-custom',
          },
          {
            declaredName: 'broken-skill',
            id: 'broken-package',
            description: '',
            invalidCode: 'name_mismatch',
            invalidDetail:
              'Directory name "broken-package" does not match SKILL.md name "broken-skill".',
            runtimeDir: '/tmp/security-goose/.agents/skills/broken-package',
            status: 'invalid',
          },
        ],
      })
      .mockResolvedValue({
        workingDir: '/tmp/security-goose',
        bundledSkillRoot: '/repo/distro/security-cn/skills',
        runtimeSkillRoot: '/tmp/security-goose/.agents/skills',
        bundledSkills: [
          {
            declaredName: 'vuln-triage',
            id: 'vuln-triage',
            description: 'Security Goose skill',
            runtimeDir: '/tmp/security-goose/.agents/skills/vuln-triage',
            sourceDir: '/repo/distro/security-cn/skills/vuln-triage',
            status: 'bundled-security',
          },
          {
            declaredName: 'report-writing',
            id: 'report-writing',
            description: 'Bundled runtime missing',
            runtimeDir: '/tmp/security-goose/.agents/skills/report-writing',
            sourceDir: '/repo/distro/security-cn/skills/report-writing',
            status: 'missing-runtime',
          },
        ],
        localSkills: [],
      });
    Object.assign(window.electron, {
      listManagedSkills,
      selectFileOrDirectory: vi.fn(async () => '/tmp/local-investigation'),
      importManagedSkill: vi.fn(async () => ({
        status: 'installed',
        skillId: 'local-investigation',
        localStatus: 'local-custom',
        targetDir: '/tmp/security-goose/.agents/skills/local-investigation',
      })),
      deleteManagedLocalSkill: vi.fn(async () => ({
        removed: true,
        removedPath: '/tmp/security-goose/.agents/skills/local-investigation',
      })),
      restoreBundledSkill: vi.fn(async () => ({
        restored: true,
        targetDir: '/tmp/security-goose/.agents/skills/vuln-triage',
      })),
      openDirectoryInExplorer: vi.fn(async () => ({ opened: true })),
      readFile: vi.fn(async (filePath: string) => ({
        error: null,
        file: `---\nname: ${filePath.includes('broken-package') ? 'broken-skill' : 'vuln-triage'}\ndescription: Sample skill detail\n---\nUse this skill for structured triage.`,
        filePath,
        found: true,
      })),
      showMessageBox: vi.fn(async () => ({ response: 1 })),
    });
    (window as unknown as Record<string, unknown>).appConfig = {
      get: (key: string) =>
        key === 'GOOSE_VISIBLE_SKILLS_SCOPE' ? 'builtin-and-security' : undefined,
      getAll: () => ({
        GOOSE_VISIBLE_SKILLS_SCOPE: 'builtin-and-security',
      }),
    };
  });

  it('shows only Goose built-ins and bundled Security Goose skills in scoped mode', async () => {
    mocks.getSlashCommands.mockResolvedValue({
      data: {
        commands: [
          { command: 'goose-doc-guide', command_type: 'Skill', help: 'Builtin Goose docs skill' },
          { command: 'vuln-triage', command_type: 'Skill', help: 'Security Goose skill' },
          { command: 'claude-review', command_type: 'Skill', help: 'External Claude skill' },
          { command: 'local-investigation', command_type: 'Skill', help: 'Local project skill' },
          { command: 'security-vuln-triage', command_type: 'Recipe', help: 'Recipe path' },
        ],
      },
    });

    render(
      <IntlTestWrapper>
        <SkillsView />
      </IntlTestWrapper>
    );

    await waitFor(() => {
      expect(mocks.getSlashCommands).toHaveBeenCalledTimes(1);
    });

    await screen.findByText('goose-doc-guide');

    expect(screen.getByText('Bundled Skills')).toBeInTheDocument();
    expect(screen.getByText('My Skills / Local Skills')).toBeInTheDocument();
    expect(screen.getByText('goose-doc-guide')).toBeInTheDocument();
    expect(screen.getByText('vuln-triage')).toBeInTheDocument();
    expect(screen.queryByText('claude-review')).not.toBeInTheDocument();
    expect(screen.getByText('local-investigation')).toBeInTheDocument();
    expect(screen.getByText('Overridden locally')).toBeInTheDocument();
    expect(screen.getByText('Missing runtime')).toBeInTheDocument();
    expect(screen.getByText('Invalid skill package')).toBeInTheDocument();
    expect(
      screen.getByText(
        /shows three skill groups: built-in skills, bundled security skills, and local skills installed into the current project/i
      )
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /The \/ menu only shows skills the current runtime has discovered for this session/i
      )
    ).toBeInTheDocument();
  });

  it('shows runtime diagnostics and read-only skill details with Finder actions', async () => {
    mocks.getSlashCommands.mockResolvedValue({
      data: {
        commands: [
          { command: 'goose-doc-guide', command_type: 'Skill', help: 'Builtin Goose docs skill' },
          { command: 'vuln-triage', command_type: 'Skill', help: 'Security Goose skill' },
          { command: 'local-investigation', command_type: 'Skill', help: 'Local project skill' },
        ],
      },
    });

    render(
      <IntlTestWrapper>
        <SkillsView />
      </IntlTestWrapper>
    );

    await screen.findByText('Bundled Skills');

    expect(screen.queryByText('Current project skills')).not.toBeInTheDocument();
    expect(
      screen.queryByText(/bundled security skills tracked, 2 current-project local skills managed/i)
    ).not.toBeInTheDocument();
    expect(
      screen.queryByText(/1 missing runtime, 1 overridden locally, 1 invalid local package/i)
    ).not.toBeInTheDocument();
    expect(screen.queryByText('Working directory')).not.toBeInTheDocument();
    expect(screen.queryByText('Current project runtime')).not.toBeInTheDocument();
    expect(screen.queryByText('/tmp/security-goose')).not.toBeInTheDocument();
    expect(screen.queryByText('/tmp/security-goose/.agents/skills')).not.toBeInTheDocument();

    await userEvent.click(screen.getAllByRole('button', { name: 'Details' })[1]);

    const dialog = await screen.findByRole('dialog');
    const detailLayout = within(dialog).getByTestId('skill-detail-layout');

    expect(window.electron.readFile).toHaveBeenCalledWith(
      '/tmp/security-goose/.agents/skills/vuln-triage/SKILL.md'
    );
    expect(detailLayout.className).toContain('md:grid-cols-2');
    expect(within(dialog).queryByText('Source type')).not.toBeInTheDocument();
    expect(within(dialog).queryByText('Frontmatter name')).not.toBeInTheDocument();
    expect(within(dialog).queryByText('Working directory')).not.toBeInTheDocument();
    expect(within(dialog).queryByText('Current project runtime')).not.toBeInTheDocument();
    expect(within(dialog).getByText('Skill folder')).toBeInTheDocument();
    expect(within(dialog).getByText('/tmp/security-goose/.agents/skills/vuln-triage')).toBeInTheDocument();
    expect(within(dialog).getByText('Runtime diagnosis')).toBeInTheDocument();
    expect(
      within(dialog).getByText((content) => content.includes('Use this skill for structured triage.'))
    ).toBeInTheDocument();
    expect(within(dialog).queryByText(/name: vuln-triage/)).not.toBeInTheDocument();
    expect(within(dialog).queryByText(/description: Sample skill detail/)).not.toBeInTheDocument();

    await userEvent.click(within(dialog).getByRole('button', { name: 'Reveal skill folder' }));

    expect(window.electron.openDirectoryInExplorer).toHaveBeenCalledWith(
      '/tmp/security-goose/.agents/skills/vuln-triage'
    );
    expect(mocks.toastSuccess).toHaveBeenCalledWith({
      msg: 'Opened "/tmp/security-goose/.agents/skills/vuln-triage" in Finder.',
      title: 'Skill folder opened',
    });
  });

  it('can start a new chat directly from a slash-visible skill', async () => {
    mocks.getSlashCommands.mockResolvedValue({
      data: {
        commands: [
          { command: 'goose-doc-guide', command_type: 'Skill', help: 'Builtin Goose docs skill' },
          { command: 'vuln-triage', command_type: 'Skill', help: 'Security Goose skill' },
        ],
      },
    });

    render(
      <IntlTestWrapper>
        <SkillsView />
      </IntlTestWrapper>
    );

    await screen.findByText('vuln-triage');
    await userEvent.click(screen.getAllByRole('button', { name: 'Details' })[1]);

    const dialog = await screen.findByRole('dialog');
    const startButton = within(dialog).getByRole('button', { name: 'Start chat from this skill' });

    expect(startButton).toBeEnabled();

    await userEvent.click(startButton);

    await waitFor(() => {
      expect(mocks.startNewSession).toHaveBeenCalledWith(
        '/vuln-triage',
        mocks.setView,
        '/tmp/security-goose'
      );
    });
  });

  it('shows a direct start action on skill cards for slash-visible skills without required input', async () => {
    mocks.getSlashCommands.mockResolvedValue({
      data: {
        commands: [
          { command: 'goose-doc-guide', command_type: 'Skill', help: 'Builtin Goose docs skill' },
          { command: 'local-investigation', command_type: 'Skill', help: 'Local project skill' },
        ],
      },
    });

    render(
      <IntlTestWrapper>
        <SkillsView />
      </IntlTestWrapper>
    );

    const localSkillCard = (await screen.findByText('local-investigation')).closest('[data-slot="card"]');
    expect(localSkillCard).not.toBeNull();

    await userEvent.click(
      within(localSkillCard as HTMLElement).getByRole('button', { name: 'Start' })
    );

    await waitFor(() => {
      expect(mocks.startNewSession).toHaveBeenCalledWith(
        '/local-investigation',
        mocks.setView,
        '/tmp/security-goose'
      );
    });
  });

  it('collects required input before starting a parameterized skill chat', async () => {
    mocks.getSlashCommands.mockResolvedValue({
      data: {
        commands: [
          {
            command: 'local-investigation',
            command_type: 'Skill',
            help: 'Local project skill',
            input_hint: '<ioc>',
          },
        ],
      },
    });

    render(
      <IntlTestWrapper>
        <SkillsView />
      </IntlTestWrapper>
    );

    const localSkillCard = (await screen.findByText('local-investigation')).closest('[data-slot="card"]');
    expect(localSkillCard).not.toBeNull();

    await userEvent.click(
      within(localSkillCard as HTMLElement).getByRole('button', { name: 'Start' })
    );

    const dialog = await screen.findByRole('dialog', { name: 'Start local-investigation' });
    const input = within(dialog).getByRole('textbox', { name: 'Required input' });
    const confirmButton = within(dialog).getByRole('button', { name: 'Start chat' });

    expect(input).toHaveAttribute('placeholder', '<ioc>');
    expect(confirmButton).toBeDisabled();

    await userEvent.type(input, '8.8.8.8');
    expect(confirmButton).toBeEnabled();

    await userEvent.click(confirmButton);

    await waitFor(() => {
      expect(mocks.startNewSession).toHaveBeenCalledWith(
        '/local-investigation 8.8.8.8',
        mocks.setView,
        '/tmp/security-goose'
      );
    });
  });

  it('shows why a skill cannot start a chat when the current / menu has not discovered it', async () => {
    mocks.getSlashCommands.mockResolvedValue({
      data: {
        commands: [
          { command: 'goose-doc-guide', command_type: 'Skill', help: 'Builtin Goose docs skill' },
          { command: 'vuln-triage', command_type: 'Skill', help: 'Security Goose skill' },
        ],
      },
    });

    render(
      <IntlTestWrapper>
        <SkillsView />
      </IntlTestWrapper>
    );

    await screen.findByText('report-writing');
    await userEvent.click(screen.getAllByRole('button', { name: 'Details' })[2]);

    const dialog = await screen.findByRole('dialog');
    expect(
      within(dialog).getByRole('button', { name: 'Start chat from this skill' })
    ).toBeDisabled();
    expect(
      within(dialog).getByText(
        'The current / menu has not discovered this skill yet. Reopen the session after importing, restoring, or fixing the skill package, then try again.'
      )
    ).toBeInTheDocument();
  });

  it('shows an error toast when starting a chat from the selected skill fails', async () => {
    mocks.startNewSession.mockRejectedValueOnce(new Error('Session start failed'));
    mocks.getSlashCommands.mockResolvedValue({
      data: {
        commands: [{ command: 'vuln-triage', command_type: 'Skill', help: 'Security Goose skill' }],
      },
    });

    render(
      <IntlTestWrapper>
        <SkillsView />
      </IntlTestWrapper>
    );

    await screen.findByText('vuln-triage');
    await userEvent.click(screen.getAllByRole('button', { name: 'Details' })[0]);

    const dialog = await screen.findByRole('dialog');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Start chat from this skill' }));

    await waitFor(() => {
      expect(mocks.toastError).toHaveBeenCalledWith({
        msg: 'Session start failed',
        title: 'Could not start chat from skill',
        traceback: 'Session start failed',
      });
    });
  });

  it('shows an error toast when opening the skill folder fails', async () => {
    const openDirectoryInExplorer = window.electron.openDirectoryInExplorer as ReturnType<typeof vi.fn>;
    openDirectoryInExplorer.mockResolvedValue({
      error: 'Permission denied',
      opened: false,
    });

    mocks.getSlashCommands.mockResolvedValue({
      data: {
        commands: [{ command: 'vuln-triage', command_type: 'Skill', help: 'Security Goose skill' }],
      },
    });

    render(
      <IntlTestWrapper>
        <SkillsView />
      </IntlTestWrapper>
    );

    await screen.findByText('vuln-triage');
    await userEvent.click(screen.getAllByRole('button', { name: 'Details' })[0]);

    const dialog = await screen.findByRole('dialog');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Reveal skill folder' }));

    expect(mocks.toastError).toHaveBeenCalledWith({
      msg: 'Permission denied',
      title: 'Could not open skill folder',
      traceback: 'Permission denied',
    });
  });

  it('shows invalid skill recovery guidance in the detail view', async () => {
    mocks.getSlashCommands.mockResolvedValue({
      data: {
        commands: [
          { command: 'goose-doc-guide', command_type: 'Skill', help: 'Builtin Goose docs skill' },
          { command: 'broken-package', command_type: 'Skill', help: 'Invalid local skill' },
        ],
      },
    });

    render(
      <IntlTestWrapper>
        <SkillsView />
      </IntlTestWrapper>
    );

    await screen.findByText('broken-package');

    const detailButtons = screen.getAllByRole('button', { name: 'Details' });
    await userEvent.click(detailButtons[detailButtons.length - 1]);

    const dialog = await screen.findByRole('dialog');

    expect(within(dialog).getAllByText('Invalid skill package').length).toBeGreaterThan(0);
    expect(within(dialog).getByText('Skill folder')).toBeInTheDocument();
    expect(
      within(dialog).getByText(
        'Rename the folder or update the frontmatter name so both match, then re-import or restore the skill.'
      )
    ).toBeInTheDocument();
  });

  it('hides bundled source entry points and benign diagnostics for healthy bundled skills', async () => {
    const listManagedSkills = window.electron.listManagedSkills as ReturnType<typeof vi.fn>;
    listManagedSkills.mockReset();
    listManagedSkills.mockResolvedValue({
      workingDir: '/tmp/security-goose',
      bundledSkillRoot: '/repo/distro/security-cn/skills',
      runtimeSkillRoot: '/tmp/security-goose/.agents/skills',
      bundledSkills: [
        {
          declaredName: 'vuln-triage',
          id: 'vuln-triage',
          description: 'Security Goose skill',
          runtimeDir: '/tmp/security-goose/.agents/skills/vuln-triage',
          sourceDir: '/repo/distro/security-cn/skills/vuln-triage',
          status: 'bundled-security',
        },
      ],
      localSkills: [],
    });

    mocks.getSlashCommands.mockResolvedValue({
      data: {
        commands: [
          { command: 'vuln-triage', command_type: 'Skill', help: 'Security Goose skill' },
        ],
      },
    });

    render(
      <IntlTestWrapper>
        <SkillsView />
      </IntlTestWrapper>
    );

    await screen.findByText('vuln-triage');

    await userEvent.click(screen.getByRole('button', { name: 'Details' }));

    const dialog = await screen.findByRole('dialog');
    const detailLayout = within(dialog).getByTestId('skill-detail-layout');

    expect(detailLayout.className).toContain('grid-cols-1');
    expect(detailLayout.className).not.toContain('md:grid-cols-2');
    expect(within(dialog).queryByText('Bundled source')).not.toBeInTheDocument();
    expect(within(dialog).queryByText('Source type')).not.toBeInTheDocument();
    expect(within(dialog).queryByText('Frontmatter name')).not.toBeInTheDocument();
    expect(within(dialog).queryByText('Working directory')).not.toBeInTheDocument();
    expect(within(dialog).queryByText('Current project runtime')).not.toBeInTheDocument();
    expect(
      within(dialog).queryByText(
        'Bundled Security Goose skill is present in the current project runtime.'
      )
    ).not.toBeInTheDocument();
    expect(
      within(dialog).queryByRole('button', { name: 'Reveal bundled source' })
    ).not.toBeInTheDocument();
    expect(
      within(dialog).getByRole('button', { name: 'Reveal skill folder' })
    ).toBeInTheDocument();
  });

  it('reloads inventory after importing a local skill', async () => {
    mocks.getSlashCommands.mockResolvedValue({
      data: {
        commands: [
          { command: 'goose-doc-guide', command_type: 'Skill', help: 'Builtin Goose docs skill' },
          { command: 'local-investigation', command_type: 'Skill', help: 'Local project skill' },
        ],
      },
    });

    render(
      <IntlTestWrapper>
        <SkillsView />
      </IntlTestWrapper>
    );

    await screen.findByText('Bundled Skills');

    await userEvent.click(screen.getByRole('button', { name: 'Import Skill' }));

    await waitFor(() => {
      expect(window.electron.selectFileOrDirectory).toHaveBeenCalledWith('/tmp/security-goose');
      expect(window.electron.importManagedSkill).toHaveBeenCalledWith({
        overwrite: false,
        sourcePath: '/tmp/local-investigation',
        workingDir: '/tmp/security-goose',
      });
      expect(mocks.toastSuccess).toHaveBeenCalledWith({
        msg: 'Installed "local-investigation" into the current project runtime.',
        title: 'Skill imported',
      });
      expect(window.electron.listManagedSkills).toHaveBeenCalledTimes(2);
      expect(mocks.getSlashCommands).toHaveBeenCalledTimes(2);
    });
  });

  it('shows an error toast when importing a local skill fails', async () => {
    const importManagedSkill = window.electron.importManagedSkill as ReturnType<typeof vi.fn>;
    importManagedSkill.mockRejectedValue(new Error('Import blocked by filesystem'));

    mocks.getSlashCommands.mockResolvedValue({
      data: {
        commands: [
          { command: 'goose-doc-guide', command_type: 'Skill', help: 'Builtin Goose docs skill' },
        ],
      },
    });

    render(
      <IntlTestWrapper>
        <SkillsView />
      </IntlTestWrapper>
    );

    await screen.findByText('Bundled Skills');
    await userEvent.click(screen.getByRole('button', { name: 'Import Skill' }));

    await waitFor(() => {
      expect(mocks.toastError).toHaveBeenCalledWith({
        msg: 'Import blocked by filesystem',
        title: 'Skill import failed',
        traceback: 'Import blocked by filesystem',
      });
    });
  });

  it('reloads inventory after deleting or restoring a managed skill', async () => {
    const listManagedSkills = window.electron.listManagedSkills as ReturnType<typeof vi.fn>;
    listManagedSkills.mockReset();
    listManagedSkills
      .mockResolvedValueOnce({
        workingDir: '/tmp/security-goose',
        runtimeSkillRoot: '/tmp/security-goose/.agents/skills',
        bundledSkills: [
          {
            id: 'vuln-triage',
            description: 'Security Goose skill',
            status: 'local-override',
          },
        ],
        localSkills: [
          {
            id: 'local-investigation',
            description: 'Current project local skill',
            status: 'local-custom',
          },
        ],
      })
      .mockResolvedValueOnce({
        workingDir: '/tmp/security-goose',
        runtimeSkillRoot: '/tmp/security-goose/.agents/skills',
        bundledSkills: [
          {
            id: 'vuln-triage',
            description: 'Security Goose skill',
            status: 'local-override',
          },
        ],
        localSkills: [],
      })
      .mockResolvedValue({
        workingDir: '/tmp/security-goose',
        runtimeSkillRoot: '/tmp/security-goose/.agents/skills',
        bundledSkills: [
          {
            id: 'vuln-triage',
            description: 'Security Goose skill',
            status: 'bundled-security',
          },
        ],
        localSkills: [],
      });

    mocks.getSlashCommands.mockResolvedValue({
      data: {
        commands: [
          { command: 'goose-doc-guide', command_type: 'Skill', help: 'Builtin Goose docs skill' },
          { command: 'vuln-triage', command_type: 'Skill', help: 'Security Goose skill' },
          { command: 'local-investigation', command_type: 'Skill', help: 'Local project skill' },
        ],
      },
    });

    render(
      <IntlTestWrapper>
        <SkillsView />
      </IntlTestWrapper>
    );

    await screen.findByText('local-investigation');

    await userEvent.click(screen.getByRole('button', { name: /^Delete$/ }));

    await waitFor(() => {
      expect(window.electron.deleteManagedLocalSkill).toHaveBeenCalledWith(
        '/tmp/security-goose',
        'local-investigation'
      );
      expect(mocks.toastSuccess).toHaveBeenCalledWith({
        msg: 'Removed "local-investigation" from the current project runtime.',
        title: 'Local skill deleted',
      });
    });

    await userEvent.click(await screen.findByRole('button', { name: 'Restore' }));

    await waitFor(() => {
      expect(window.electron.restoreBundledSkill).toHaveBeenCalledWith(
        '/tmp/security-goose',
        'vuln-triage'
      );
      expect(mocks.toastSuccess).toHaveBeenCalledWith({
        msg: 'Restored bundled skill "vuln-triage" into the current project runtime.',
        title: 'Bundled skill restored',
      });
      expect(window.electron.listManagedSkills).toHaveBeenCalledTimes(3);
      expect(mocks.getSlashCommands).toHaveBeenCalledTimes(3);
    });
  });

  it('shows error toasts when deleting or restoring a managed skill fails', async () => {
    const deleteManagedLocalSkill = window.electron.deleteManagedLocalSkill as ReturnType<typeof vi.fn>;
    const restoreBundledSkill = window.electron.restoreBundledSkill as ReturnType<typeof vi.fn>;
    deleteManagedLocalSkill.mockRejectedValueOnce(new Error('Delete failed'));
    restoreBundledSkill.mockRejectedValueOnce(new Error('Restore failed'));

    const listManagedSkills = window.electron.listManagedSkills as ReturnType<typeof vi.fn>;
    listManagedSkills.mockReset();
    listManagedSkills.mockResolvedValue({
      workingDir: '/tmp/security-goose',
      runtimeSkillRoot: '/tmp/security-goose/.agents/skills',
      bundledSkills: [
        {
          id: 'vuln-triage',
          description: 'Security Goose skill',
          status: 'local-override',
        },
      ],
      localSkills: [
        {
          id: 'local-investigation',
          description: 'Current project local skill',
          status: 'local-custom',
        },
      ],
    });

    mocks.getSlashCommands.mockResolvedValue({
      data: {
        commands: [
          { command: 'goose-doc-guide', command_type: 'Skill', help: 'Builtin Goose docs skill' },
          { command: 'vuln-triage', command_type: 'Skill', help: 'Security Goose skill' },
          { command: 'local-investigation', command_type: 'Skill', help: 'Local project skill' },
        ],
      },
    });

    render(
      <IntlTestWrapper>
        <SkillsView />
      </IntlTestWrapper>
    );

    await screen.findByText('local-investigation');

    await userEvent.click(screen.getByRole('button', { name: /^Delete$/ }));

    await waitFor(() => {
      expect(mocks.toastError).toHaveBeenCalledWith({
        msg: 'Delete failed',
        title: 'Could not delete local skill',
        traceback: 'Delete failed',
      });
    });

    await userEvent.click(screen.getByRole('button', { name: 'Restore' }));

    await waitFor(() => {
      expect(mocks.toastError).toHaveBeenCalledWith({
        msg: 'Restore failed',
        title: 'Could not restore bundled skill',
        traceback: 'Restore failed',
      });
    });
  });
});
