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

export interface SecurityTaskDefinition {
  id: SecurityTaskId;
  recipeId?: string;
  skillId: string;
  recommendedExtensionIds: ReadonlyArray<SecurityExtensionId>;
  starterPrompt: {
    en: string;
    zh: string;
  };
}

export interface SecurityTaskLaunchConfig
  extends Omit<SecurityTaskDefinition, 'starterPrompt'> {
  availability: SecurityTaskAvailability;
  launchMode: SecurityTaskLaunchMode;
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
      en: [
        'Use the security-vuln-triage recipe if it is available in this session.',
        'If the vuln-triage skill is installed, follow its method and output template.',
        'Help me triage a vulnerability and return: conclusion, exploitation conditions, severity, key evidence, unknowns, and next steps.',
        '',
        'Material to analyze:',
      ].join('\n'),
      zh: [
        '如果当前会话可用，请使用 security-vuln-triage recipe。',
        '如果已安装 vuln-triage skill，优先遵循它的方法论和输出模板。',
        '请帮我完成漏洞研判，并输出：漏洞结论、利用条件、风险等级、关键证据、不确定项、建议下一步。',
        '',
        '待分析内容：',
      ].join('\n'),
    },
  },
  {
    id: 'alert-investigation',
    recipeId: 'alert-investigation',
    skillId: 'alert-triage',
    recommendedExtensionIds: ['threat-intel-mcp', 'local-security-gateway-mcp'],
    starterPrompt: {
      en: [
        'Use the alert-investigation recipe if it is available in this session.',
        'If the alert-triage skill is installed, follow its method and output template.',
        'Help me investigate this alert and return: authenticity judgement, trigger cause, affected assets, severity, and response actions.',
        '',
        'Alert evidence:',
      ].join('\n'),
      zh: [
        '如果当前会话可用，请使用 alert-investigation recipe。',
        '如果已安装 alert-triage skill，优先遵循它的方法论和输出模板。',
        '请帮我完成告警分析，并输出：真实性判断、触发原因、受影响资产、风险等级、建议处置动作。',
        '',
        '告警线索：',
      ].join('\n'),
    },
  },
  {
    id: 'ioc-analysis',
    skillId: 'ioc-analysis',
    recommendedExtensionIds: ['threat-intel-mcp'],
    starterPrompt: {
      en: [
        'Use the ioc-analysis skill if it is installed.',
        'Analyze the IOC clues below and produce a structured result with indicator classification, confidence, linked entities, risk conclusion, and next steps.',
        '',
        'IOC clues:',
      ].join('\n'),
      zh: [
        '如果已安装 ioc-analysis skill，请优先使用它的方法论。',
        '请对下面的 IOC 线索做结构化研判，输出：指标分类、可信度判断、关联实体、风险结论、建议下一步。',
        '',
        '待分析 IOC：',
      ].join('\n'),
    },
  },
  {
    id: 'web-investigation',
    recipeId: 'web-investigation',
    skillId: 'ioc-analysis',
    recommendedExtensionIds: ['browser-assist-mcp', 'threat-intel-mcp'],
    starterPrompt: {
      en: [
        'Use the web-investigation recipe if it is available in this session.',
        'If the ioc-analysis skill is installed, use it for IOC extraction and confidence judgement.',
        'Investigate the target page and return: page theme, suspicious indicators, related IOCs, confidence, and next steps.',
        '',
        'Page or clue to inspect:',
      ].join('\n'),
      zh: [
        '如果当前会话可用，请使用 web-investigation recipe。',
        '如果已安装 ioc-analysis skill，请用它来完成 IOC 提取和可信度判断。',
        '请对目标页面做网页调查，并输出：页面主题、可疑指标、关联 IOC、可信度判断、建议下一步。',
        '',
        '待调查页面或线索：',
      ].join('\n'),
    },
  },
  {
    id: 'report-writing',
    skillId: 'report-writing',
    recommendedExtensionIds: [],
    starterPrompt: {
      en: [
        'Use the report-writing skill if it is installed.',
        'Turn the investigation result below into a structured security report that can be handed off to operators or ticketing systems.',
        '',
        'Source material:',
      ].join('\n'),
      zh: [
        '如果已安装 report-writing skill，请优先使用它的方法论。',
        '请把下面的调查结果整理成适合汇报、留痕或转工单的结构化安全报告。',
        '',
        '原始材料：',
      ].join('\n'),
    },
  },
  {
    id: 'wooyun-legacy',
    skillId: 'wooyun-legacy',
    recommendedExtensionIds: ['browser-assist-mcp'],
    starterPrompt: {
      en: [
        'Use the wooyun-legacy skill if it is installed.',
        'Review the target workflow using a WooYun-style business logic investigation approach and distinguish direct evidence from historical pattern analogy.',
        '',
        'Target workflow or API:',
      ].join('\n'),
      zh: [
        '如果已安装 wooyun-legacy skill，请优先使用它的方法论。',
        '请按 WooYun 风格的业务逻辑排查方法审视下面的流程或接口，并区分直接证据与历史模式类比。',
        '',
        '目标流程或接口：',
      ].join('\n'),
    },
  },
] as const;

const SECURITY_TASK_MAP = new Map(SECURITY_TASKS.map((task) => [task.id, task]));

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

export function resolveSecurityTaskLaunchConfig(
  taskId: SecurityTaskId,
  locale = 'en',
  availableRecipeIds?: ReadonlySet<string>
): SecurityTaskLaunchConfig {
  const task = getSecurityTaskById(taskId);
  const recipeAvailable =
    typeof task.recipeId === 'string' &&
    (availableRecipeIds === undefined || availableRecipeIds.has(task.recipeId));

  return {
    ...task,
    availability: recipeAvailable ? 'ready' : 'preview',
    launchMode: recipeAvailable ? 'recipe' : 'prompt',
    recipeId: recipeAvailable ? task.recipeId : undefined,
    starterPrompt: isChineseLocale(locale) ? task.starterPrompt.zh : task.starterPrompt.en,
  };
}
