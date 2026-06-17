import type { SecurityExtensionId } from './extensionCatalog';

export const SECURITY_TASK_IDS = [
  'vuln-triage',
  'alert-investigation',
  'ioc-analysis',
  'web-investigation',
  'report-writing',
  'wooyun-legacy',
] as const;

export type SecurityTaskId = (typeof SECURITY_TASK_IDS)[number];
export type SecurityTaskAvailability = 'ready' | 'preview';
export type SecurityTaskLaunchMode = 'recipe' | 'prompt';
export type SecurityTaskPrimaryPath = 'recipe' | 'skill';

export interface SecurityTaskDefinition {
  id: SecurityTaskId;
  recipeId?: string;
  skillId: string;
  recommendedExtensionIds: ReadonlyArray<SecurityExtensionId>;
  starterPrompt: {
    ready?: {
      en: string;
      zh: string;
    };
    preview: {
      en: string;
      zh: string;
    };
  };
}

export interface SecurityTaskLaunchConfig
  extends Omit<SecurityTaskDefinition, 'starterPrompt'> {
  availability: SecurityTaskAvailability;
  launchMode: SecurityTaskLaunchMode;
  primaryPath: SecurityTaskPrimaryPath;
  preferredRecipeId?: string;
  starterPrompt: string;
}

export interface AvailableRecipeManifest {
  id: string;
  file_path?: string;
}

export const SECURITY_TASKS: readonly SecurityTaskDefinition[] = [
  {
    id: 'vuln-triage',
    recipeId: 'security-vuln-triage',
    skillId: 'vuln-triage',
    recommendedExtensionIds: ['aiseesec-mcp'],
    starterPrompt: {
      ready: {
        en: [
          'This task already runs on the security-vuln-triage recipe as the primary workflow.',
          'If the vuln-triage skill is available and directly observable in this session, use it as supporting methodology and keep the recipe-backed structure.',
          'If the session cannot directly confirm that the mapped skill is loaded, do not present it as a confirmed runtime signal. Keep this task workflow and output structure as the source of truth.',
          'Return these sections: Vulnerability conclusion, Exploitation conditions, Risk severity, Key evidence, Unknowns, Next steps.',
          '',
          'Material to analyze:',
        ].join('\n'),
        zh: [
          '当前任务以 security-vuln-triage recipe 作为主执行路径。',
          '如果已安装且当前会话可直接观测到 vuln-triage skill，可把它作为补充方法论，并保持 recipe-backed 的输出结构。',
          '如果当前会话不能直接观测到映射 skill 已加载，请不要把它表述为已确认的运行时信号，仍以当前任务要求的流程和输出结构为准。',
          '请按这些部分输出：漏洞结论、利用条件、风险等级、关键证据、不确定项、建议下一步。',
          '',
          '待分析内容：',
        ].join('\n'),
      },
      preview: {
        en: [
          'The security-vuln-triage recipe runtime is unavailable in this workspace.',
          'If the vuln-triage skill is available and directly observable in this session, use it as the primary methodology and output template.',
          'If the session cannot directly confirm that the mapped skill is loaded, do not present it as a confirmed runtime signal. Still answer with the required output structure below.',
          'Return these sections: Vulnerability conclusion, Exploitation conditions, Risk severity, Key evidence, Unknowns, Next steps.',
          '',
          'Material to analyze:',
        ].join('\n'),
        zh: [
          '当前工作区缺少 security-vuln-triage recipe 运行时。',
          '如果已安装且当前会话可直接观测到 vuln-triage skill，请把它作为主方法论和输出模板。',
          '如果当前会话不能直接观测到映射 skill 已加载，请不要把它表述为已确认的运行时信号，仍按下面要求的输出结构组织回答。',
          '请按这些部分输出：漏洞结论、利用条件、风险等级、关键证据、不确定项、建议下一步。',
          '',
          '待分析内容：',
        ].join('\n'),
      },
    },
  },
  {
    id: 'alert-investigation',
    recipeId: 'alert-investigation',
    skillId: 'alert-triage',
    recommendedExtensionIds: ['threat-intel-mcp', 'local-security-gateway-mcp'],
    starterPrompt: {
      ready: {
        en: [
          'This task already runs on the alert-investigation recipe as the primary workflow.',
          'If the alert-triage skill is available and directly observable in this session, use it as supporting methodology and preserve its evidence boundary.',
          'If the session cannot directly confirm that the mapped skill is loaded, do not present it as a confirmed runtime signal. Keep this task workflow and output structure as the source of truth.',
          'Return these sections: Authenticity judgement, Trigger cause, Affected assets, Risk severity, Response actions, Missing data.',
          '',
          'Alert evidence:',
        ].join('\n'),
        zh: [
          '当前任务以 alert-investigation recipe 作为主执行路径。',
          '如果已安装且当前会话可直接观测到 alert-triage skill，可把它作为补充方法论，并保持它的证据边界。',
          '如果当前会话不能直接观测到映射 skill 已加载，请不要把它表述为已确认的运行时信号，仍以当前任务要求的流程和输出结构为准。',
          '请按这些部分输出：真实性判断、触发原因、受影响资产、风险等级、建议处置动作、待补充数据。',
          '',
          '告警线索：',
        ].join('\n'),
      },
      preview: {
        en: [
          'The alert-investigation recipe runtime is unavailable in this workspace.',
          'If the alert-triage skill is available and directly observable in this session, use it as the primary methodology and output template.',
          'If the session cannot directly confirm that the mapped skill is loaded, do not present it as a confirmed runtime signal. Still answer with the required output structure below.',
          'Return these sections: Authenticity judgement, Trigger cause, Affected assets, Risk severity, Response actions, Missing data.',
          '',
          'Alert evidence:',
        ].join('\n'),
        zh: [
          '当前工作区缺少 alert-investigation recipe 运行时。',
          '如果已安装且当前会话可直接观测到 alert-triage skill，请把它作为主方法论和输出模板。',
          '如果当前会话不能直接观测到映射 skill 已加载，请不要把它表述为已确认的运行时信号，仍按下面要求的输出结构组织回答。',
          '请按这些部分输出：真实性判断、触发原因、受影响资产、风险等级、建议处置动作、待补充数据。',
          '',
          '告警线索：',
        ].join('\n'),
      },
    },
  },
  {
    id: 'ioc-analysis',
    recipeId: 'ioc-analysis',
    skillId: 'ioc-analysis',
    recommendedExtensionIds: ['threat-intel-mcp'],
    starterPrompt: {
      ready: {
        en: [
          'This task already runs on the ioc-analysis recipe as the primary workflow.',
          'If the ioc-analysis skill is available and directly observable in this session, use it as supporting methodology and keep the recipe-backed output structure.',
          'If the session cannot directly confirm that the mapped skill is loaded, do not present it as a confirmed runtime signal. Keep this task workflow and output structure as the source of truth.',
          'Return these sections: IOC summary, Key findings, Linked entities, Confidence judgement, Next steps, Open questions.',
          '',
          'IOC clues:',
        ].join('\n'),
        zh: [
          '当前任务以 ioc-analysis recipe 作为主执行路径。',
          '如果已安装且当前会话可直接观测到 ioc-analysis skill，可把它作为补充方法论，并保持 recipe-backed 的输出结构。',
          '如果当前会话不能直接观测到映射 skill 已加载，请不要把它表述为已确认的运行时信号，仍以当前任务要求的流程和输出结构为准。',
          '请按这些部分输出：IOC 摘要、关键发现、关联关系、可信度判断、建议下一步、待确认项。',
          '',
          '待分析 IOC：',
        ].join('\n'),
      },
      preview: {
        en: [
          'The ioc-analysis recipe runtime is unavailable in this workspace.',
          'If the ioc-analysis skill is available and directly observable in this session, use it as the primary methodology and output template.',
          'If the session cannot directly confirm that the mapped skill is loaded, do not present it as a confirmed runtime signal. Still answer with the required output structure below.',
          'Return these sections: IOC summary, Key findings, Linked entities, Confidence judgement, Next steps, Open questions.',
          '',
          'IOC clues:',
        ].join('\n'),
        zh: [
          '当前工作区缺少 ioc-analysis recipe 运行时。',
          '如果已安装且当前会话可直接观测到 ioc-analysis skill，请把它作为主方法论和输出模板。',
          '如果当前会话不能直接观测到映射 skill 已加载，请不要把它表述为已确认的运行时信号，仍按下面要求的输出结构组织回答。',
          '请按这些部分输出：IOC 摘要、关键发现、关联关系、可信度判断、建议下一步、待确认项。',
          '',
          '待分析 IOC：',
        ].join('\n'),
      },
    },
  },
  {
    id: 'web-investigation',
    recipeId: 'web-investigation',
    skillId: 'ioc-analysis',
    recommendedExtensionIds: ['browser-assist-mcp', 'threat-intel-mcp'],
    starterPrompt: {
      ready: {
        en: [
          'This task already runs on the web-investigation recipe as the primary workflow.',
          'If the ioc-analysis skill is available and directly observable in this session, use it as supporting methodology for IOC extraction, linked entities, and confidence judgement.',
          'If the session cannot directly confirm that the mapped skill is loaded, do not present it as a confirmed runtime signal. Keep this task workflow and output structure as the source of truth.',
          'Return these sections: Page theme, Suspicious indicators, Related IOCs, Confidence judgement, Next steps.',
          '',
          'Page or clue to inspect:',
        ].join('\n'),
        zh: [
          '当前任务以 web-investigation recipe 作为主执行路径。',
          '如果已安装且当前会话可直接观测到 ioc-analysis skill，可把它作为补充方法论，用于 IOC 提取、关联关系和可信度判断。',
          '如果当前会话不能直接观测到映射 skill 已加载，请不要把它表述为已确认的运行时信号，仍以当前任务要求的流程和输出结构为准。',
          '请按这些部分输出：页面主题、可疑指标、关联 IOC、可信度判断、建议下一步。',
          '',
          '待调查页面或线索：',
        ].join('\n'),
      },
      preview: {
        en: [
          'The web-investigation recipe runtime is unavailable in this workspace.',
          'If the ioc-analysis skill is available and directly observable in this session, use it as the primary methodology for IOC extraction and confidence judgement.',
          'If the session cannot directly confirm that the mapped skill is loaded, do not present it as a confirmed runtime signal. Still answer with the required output structure below.',
          'Return these sections: Page theme, Suspicious indicators, Related IOCs, Confidence judgement, Next steps.',
          '',
          'Page or clue to inspect:',
        ].join('\n'),
        zh: [
          '当前工作区缺少 web-investigation recipe 运行时。',
          '如果已安装且当前会话可直接观测到 ioc-analysis skill，请把它作为主方法论，用于 IOC 提取和可信度判断。',
          '如果当前会话不能直接观测到映射 skill 已加载，请不要把它表述为已确认的运行时信号，仍按下面要求的输出结构组织回答。',
          '请按这些部分输出：页面主题、可疑指标、关联 IOC、可信度判断、建议下一步。',
          '',
          '待调查页面或线索：',
        ].join('\n'),
      },
    },
  },
  {
    id: 'report-writing',
    recipeId: 'report-writing',
    skillId: 'report-writing',
    recommendedExtensionIds: [],
    starterPrompt: {
      ready: {
        en: [
          'This task already runs on the report-writing recipe as the primary workflow.',
          'If the report-writing skill is available and directly observable in this session, use it as supporting methodology and keep the recipe-backed output structure.',
          'If the session cannot directly confirm that the mapped skill is loaded, do not present it as a confirmed runtime signal. Keep this task workflow and output structure as the source of truth.',
          'Return these sections: Background, Core conclusions, Key evidence, Impact and risk, Action items, Open questions.',
          '',
          'Source material:',
        ].join('\n'),
        zh: [
          '当前任务以 report-writing recipe 作为主执行路径。',
          '如果已安装且当前会话可直接观测到 report-writing skill，可把它作为补充方法论，并保持 recipe-backed 的输出结构。',
          '如果当前会话不能直接观测到映射 skill 已加载，请不要把它表述为已确认的运行时信号，仍以当前任务要求的流程和输出结构为准。',
          '请把原始材料整理成结构化安全报告，并包含这些部分：背景、核心结论、关键证据、影响与风险、建议动作、待确认项。',
          '',
          '原始材料：',
        ].join('\n'),
      },
      preview: {
        en: [
          'The report-writing recipe runtime is unavailable in this workspace.',
          'If the report-writing skill is available and directly observable in this session, use it as the primary methodology and output template.',
          'If the session cannot directly confirm that the mapped skill is loaded, do not present it as a confirmed runtime signal. Still answer with the required output structure below.',
          'Turn the source material into a structured security report with these sections: Background, Core conclusions, Key evidence, Impact and risk, Action items, Open questions.',
          '',
          'Source material:',
        ].join('\n'),
        zh: [
          '当前工作区缺少 report-writing recipe 运行时。',
          '如果已安装且当前会话可直接观测到 report-writing skill，请把它作为主方法论和输出模板。',
          '如果当前会话不能直接观测到映射 skill 已加载，请不要把它表述为已确认的运行时信号，仍按下面要求的输出结构组织回答。',
          '请把原始材料整理成结构化安全报告，并包含这些部分：背景、核心结论、关键证据、影响与风险、建议动作、待确认项。',
          '',
          '原始材料：',
        ].join('\n'),
      },
    },
  },
  {
    id: 'wooyun-legacy',
    recipeId: 'wooyun-legacy',
    skillId: 'wooyun-legacy',
    recommendedExtensionIds: ['browser-assist-mcp'],
    starterPrompt: {
      ready: {
        en: [
          'This task already runs on the wooyun-legacy recipe as the primary workflow.',
          'If the wooyun-legacy skill is available and directly observable in this session, use it as supporting methodology and keep the analysis defensive.',
          'If the session cannot directly confirm that the mapped skill is loaded, do not present it as a confirmed runtime signal. Keep this task workflow and output structure as the source of truth.',
          'Return these sections: Current conclusion, Execution mode, Business flow and domain, Test hypotheses, Key evidence, Risk and impact, Remediation, Open questions.',
          'Inside the Key evidence section, distinguish direct evidence from historical analogy.',
          '',
          'Target workflow or API:',
        ].join('\n'),
        zh: [
          '当前任务以 wooyun-legacy recipe 作为主执行路径。',
          '如果已安装且当前会话可直接观测到 wooyun-legacy skill，可把它作为补充方法论，并保持防守视角。',
          '如果当前会话不能直接观测到映射 skill 已加载，请不要把它表述为已确认的运行时信号，仍以当前任务要求的流程和输出结构为准。',
          '请按这些部分输出：当前结论、执行模式、业务流程与领域、测试假设、关键证据、风险与影响、修复建议、待确认项。',
          '请在“关键证据”部分区分直接证据与历史模式类比。',
          '',
          '目标流程或接口：',
        ].join('\n'),
      },
      preview: {
        en: [
          'The wooyun-legacy recipe runtime is unavailable in this workspace.',
          'If the wooyun-legacy skill is available and directly observable in this session, use it as the primary methodology and keep the analysis defensive.',
          'If the session cannot directly confirm that the mapped skill is loaded, do not present it as a confirmed runtime signal. Still answer with the required output structure below.',
          'Return these sections: Current conclusion, Execution mode, Business flow and domain, Test hypotheses, Key evidence, Risk and impact, Remediation, Open questions.',
          'Inside the Key evidence section, distinguish direct evidence from historical analogy.',
          '',
          'Target workflow or API:',
        ].join('\n'),
        zh: [
          '当前工作区缺少 wooyun-legacy recipe 运行时。',
          '如果已安装且当前会话可直接观测到 wooyun-legacy skill，请把它作为主方法论，并保持防守视角。',
          '如果当前会话不能直接观测到映射 skill 已加载，请不要把它表述为已确认的运行时信号，仍按下面要求的输出结构组织回答。',
          '请按这些部分输出：当前结论、执行模式、业务流程与领域、测试假设、关键证据、风险与影响、修复建议、待确认项。',
          '请在“关键证据”部分区分直接证据与历史模式类比。',
          '',
          '目标流程或接口：',
        ].join('\n'),
      },
    },
  },
] as const;

const SECURITY_TASK_MAP = new Map(SECURITY_TASKS.map((task) => [task.id, task]));
const SECURITY_TASK_RECIPE_ID_MAP = new Map(
  SECURITY_TASKS.flatMap((task) => (task.recipeId ? [[task.recipeId, task.id] as const] : []))
);

function isChineseLocale(locale?: string): boolean {
  return typeof locale === 'string' && locale.toLowerCase().startsWith('zh');
}

function getRecipeLookupKey(filePath?: string): string | null {
  if (typeof filePath !== 'string' || filePath.length === 0) {
    return null;
  }

  const fileName = filePath.split(/[\\/]/).pop();
  if (!fileName) {
    return null;
  }

  return fileName.replace(/\.(json|ya?ml)$/i, '');
}

export function collectAvailableRecipeIds(
  recipeManifests: ReadonlyArray<AvailableRecipeManifest>
): ReadonlySet<string> {
  return new Set(collectAvailableRecipeRuntimeIds(recipeManifests).keys());
}

export function collectAvailableRecipeRuntimeIds(
  recipeManifests: ReadonlyArray<AvailableRecipeManifest>
): ReadonlyMap<string, string> {
  const availableRecipeIds = new Set<string>();
  const runtimeRecipeIds = new Map<string, string>();

  for (const recipeManifest of recipeManifests) {
    availableRecipeIds.add(recipeManifest.id);
    runtimeRecipeIds.set(recipeManifest.id, recipeManifest.id);

    const lookupKey = getRecipeLookupKey(recipeManifest.file_path);
    if (lookupKey) {
      availableRecipeIds.add(lookupKey);
      runtimeRecipeIds.set(lookupKey, recipeManifest.id);
    }
  }

  return runtimeRecipeIds;
}

export function getSecurityTaskById(taskId: SecurityTaskId): SecurityTaskDefinition {
  const task = SECURITY_TASK_MAP.get(taskId);
  if (!task) {
    throw new Error(`Unknown security task: ${taskId}`);
  }
  return task;
}

export function getSecurityTaskIdForRecipeManifest(
  recipeManifest: AvailableRecipeManifest
): SecurityTaskId | undefined {
  if (SECURITY_TASK_RECIPE_ID_MAP.has(recipeManifest.id)) {
    return SECURITY_TASK_RECIPE_ID_MAP.get(recipeManifest.id);
  }

  const lookupKey = getRecipeLookupKey(recipeManifest.file_path);
  if (lookupKey && SECURITY_TASK_RECIPE_ID_MAP.has(lookupKey)) {
    return SECURITY_TASK_RECIPE_ID_MAP.get(lookupKey);
  }

  return undefined;
}

export function resolveSecurityTaskLaunchConfig(
  taskId: SecurityTaskId,
  locale = 'en',
  availableRecipeIds?: ReadonlySet<string>
): SecurityTaskLaunchConfig {
  const task = getSecurityTaskById(taskId);
  const recipeAvailable =
    typeof task.recipeId === 'string' &&
    (availableRecipeIds === undefined || availableRecipeIds.has(task.recipeId));
  const primaryPath: SecurityTaskPrimaryPath = task.recipeId ? 'recipe' : 'skill';
  const localizedPromptSet =
    recipeAvailable && task.starterPrompt.ready ? task.starterPrompt.ready : task.starterPrompt.preview;

  return {
    ...task,
    availability: recipeAvailable ? 'ready' : 'preview',
    launchMode: recipeAvailable ? 'recipe' : 'prompt',
    primaryPath,
    preferredRecipeId: task.recipeId,
    recipeId: recipeAvailable ? task.recipeId : undefined,
    starterPrompt: isChineseLocale(locale) ? localizedPromptSet.zh : localizedPromptSet.en,
  };
}

export function resolveSecurityTaskLaunchConfigForRecipeManifest(
  recipeManifest: AvailableRecipeManifest,
  locale = 'en',
  availableRecipeIds?: ReadonlySet<string>
): SecurityTaskLaunchConfig | null {
  const taskId = getSecurityTaskIdForRecipeManifest(recipeManifest);
  return taskId ? resolveSecurityTaskLaunchConfig(taskId, locale, availableRecipeIds) : null;
}
