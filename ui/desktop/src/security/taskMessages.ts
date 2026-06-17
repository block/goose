import type { MessageDescriptor } from 'react-intl';
import { defineMessages } from '../i18n';
import type { SecurityTaskId } from './taskCatalog';

const i18n = defineMessages({
  launcherSectionTitle: {
    id: 'launcher.securityTasksTitle',
    defaultMessage: 'Security starters',
  },
  launcherSectionDescription: {
    id: 'launcher.securityTasksDescription',
    defaultMessage:
      'Launch a native security task template. If the current workspace is missing a bundled task template runtime, the entry falls back to the mapped skill prompt. Recommended extensions stay optional and are marked as local preview, disabled stub, or blocked.',
  },
  securitySectionTitle: {
    id: 'securityTasks.sectionTitle',
    defaultMessage: 'Security task starters',
  },
  securitySectionDescription: {
    id: 'securityTasks.sectionDescription',
    defaultMessage:
      'These entry points stay on top of the existing task template and skill runtime. All six bundled security tasks prefer the task template runtime first; if a task template runtime is missing in the current workspace, the app falls back to the mapped skill prompt instead of adding a parallel execution layer. Recommended extensions stay optional and keep their current real-preview, disabled-stub, or blocker state.',
  },
  methodologyBoundary: {
    id: 'securityTasks.methodologyBoundary',
    defaultMessage:
      'The current UI can show an attached task template, but it does not show explicit skill-load telemetry. Skill mappings remain guidance-first rather than confirmed runtime signals.',
  },
  extensionOverviewTitle: {
    id: 'securityTasks.extensionOverviewTitle',
    defaultMessage: 'Extension status',
  },
  extensionOverviewDescription: {
    id: 'securityTasks.extensionOverviewDescription',
    defaultMessage:
      'Enable real local preview chains from Settings → Extensions. Stub and blocked entries stay visible so task recommendations remain honest.',
  },
  savedRecipesTitle: {
    id: 'securityTasks.savedRecipesTitle',
    defaultMessage: 'Saved task templates',
  },
  openExtensions: {
    id: 'securityTasks.openExtensions',
    defaultMessage: 'Open Extensions',
  },
  startRecipe: {
    id: 'securityTasks.startRecipe',
    defaultMessage: 'Start task',
  },
  startPreview: {
    id: 'securityTasks.startPreview',
    defaultMessage: 'Start guided chat',
  },
  badgeReady: {
    id: 'securityTasks.badgeReady',
    defaultMessage: 'Template',
  },
  badgePreview: {
    id: 'securityTasks.badgePreview',
    defaultMessage: 'Preview',
  },
  mappingRecipe: {
    id: 'securityTasks.mappingRecipe',
    defaultMessage: 'Task template mapping',
  },
  mappingSkill: {
    id: 'securityTasks.mappingSkill',
    defaultMessage: 'Skill mapping',
  },
  primaryPathLabel: {
    id: 'securityTasks.primaryPathLabel',
    defaultMessage: 'Primary path',
  },
  primaryPathRecipe: {
    id: 'securityTasks.primaryPathRecipe',
    defaultMessage: 'Task template runtime',
  },
  primaryPathSkill: {
    id: 'securityTasks.primaryPathSkill',
    defaultMessage: 'Skill-guided prompt',
  },
  recommendedExtensions: {
    id: 'securityTasks.recommendedExtensions',
    defaultMessage: 'Recommended extensions',
  },
  extensionStatusLocalPreview: {
    id: 'securityTasks.extensionStatusLocalPreview',
    defaultMessage: 'Local preview',
  },
  extensionStatusDisabledStub: {
    id: 'securityTasks.extensionStatusDisabledStub',
    defaultMessage: 'Disabled stub',
  },
  extensionStatusBlocked: {
    id: 'securityTasks.extensionStatusBlocked',
    defaultMessage: 'Blocked',
  },
  extensionDetailLocalPreview: {
    id: 'securityTasks.extensionDetailLocalPreview',
    defaultMessage: 'Can be enabled now with the current local preview implementation.',
  },
  extensionDetailDisabledStub: {
    id: 'securityTasks.extensionDetailDisabledStub',
    defaultMessage: 'Catalog entry exists, but the real gateway/runtime is intentionally deferred.',
  },
  extensionDetailBlockedExternal: {
    id: 'securityTasks.extensionDetailBlockedExternal',
    defaultMessage: 'Requires external proprietary API, account, or service agreement before it can be enabled.',
  },
  vulnTriageTitle: {
    id: 'securityTasks.vulnTriage.title',
    defaultMessage: 'Vulnerability Triage',
  },
  vulnTriageDescription: {
    id: 'securityTasks.vulnTriage.description',
    defaultMessage: 'Assess CVE, advisory, PoC, patch, and asset context into an actionable conclusion.',
  },
  alertInvestigationTitle: {
    id: 'securityTasks.alertInvestigation.title',
    defaultMessage: 'Alert Investigation',
  },
  alertInvestigationDescription: {
    id: 'securityTasks.alertInvestigation.description',
    defaultMessage: 'Triage a security alert for authenticity, scope, cause, and response actions.',
  },
  iocAnalysisTitle: {
    id: 'securityTasks.iocAnalysis.title',
    defaultMessage: 'IOC Analysis',
  },
  iocAnalysisDescription: {
    id: 'securityTasks.iocAnalysis.description',
    defaultMessage: 'Investigate domain, IP, URL, hash, and other IOC clues in a structured way.',
  },
  webInvestigationTitle: {
    id: 'securityTasks.webInvestigation.title',
    defaultMessage: 'Web Investigation',
  },
  webInvestigationDescription: {
    id: 'securityTasks.webInvestigation.description',
    defaultMessage: 'Inspect a web page, extract suspicious indicators, and summarize follow-up leads.',
  },
  reportWritingTitle: {
    id: 'securityTasks.reportWriting.title',
    defaultMessage: 'Report Writing',
  },
  reportWritingDescription: {
    id: 'securityTasks.reportWriting.description',
    defaultMessage: 'Turn findings into a structured report for operators, tickets, or leadership updates.',
  },
  wooyunLegacyTitle: {
    id: 'securityTasks.wooyunLegacy.title',
    defaultMessage: 'WooYun-style Review',
  },
  wooyunLegacyDescription: {
    id: 'securityTasks.wooyunLegacy.description',
    defaultMessage: 'Review business workflows with a WooYun-style logic investigation approach.',
  },
});

export const securityTaskUiMessages = {
  launcherSectionTitle: i18n.launcherSectionTitle,
  launcherSectionDescription: i18n.launcherSectionDescription,
  securitySectionTitle: i18n.securitySectionTitle,
  securitySectionDescription: i18n.securitySectionDescription,
  methodologyBoundary: i18n.methodologyBoundary,
  extensionOverviewTitle: i18n.extensionOverviewTitle,
  extensionOverviewDescription: i18n.extensionOverviewDescription,
  savedRecipesTitle: i18n.savedRecipesTitle,
  openExtensions: i18n.openExtensions,
  startRecipe: i18n.startRecipe,
  startPreview: i18n.startPreview,
  badgeReady: i18n.badgeReady,
  badgePreview: i18n.badgePreview,
  mappingRecipe: i18n.mappingRecipe,
  mappingSkill: i18n.mappingSkill,
  primaryPathLabel: i18n.primaryPathLabel,
  primaryPathRecipe: i18n.primaryPathRecipe,
  primaryPathSkill: i18n.primaryPathSkill,
  recommendedExtensions: i18n.recommendedExtensions,
  extensionStatusLocalPreview: i18n.extensionStatusLocalPreview,
  extensionStatusDisabledStub: i18n.extensionStatusDisabledStub,
  extensionStatusBlocked: i18n.extensionStatusBlocked,
  extensionDetailLocalPreview: i18n.extensionDetailLocalPreview,
  extensionDetailDisabledStub: i18n.extensionDetailDisabledStub,
  extensionDetailBlockedExternal: i18n.extensionDetailBlockedExternal,
} satisfies Record<string, MessageDescriptor>;

export const SECURITY_TASK_COPY: Record<
  SecurityTaskId,
  { title: MessageDescriptor; description: MessageDescriptor }
> = {
  'vuln-triage': {
    title: i18n.vulnTriageTitle,
    description: i18n.vulnTriageDescription,
  },
  'alert-investigation': {
    title: i18n.alertInvestigationTitle,
    description: i18n.alertInvestigationDescription,
  },
  'ioc-analysis': {
    title: i18n.iocAnalysisTitle,
    description: i18n.iocAnalysisDescription,
  },
  'web-investigation': {
    title: i18n.webInvestigationTitle,
    description: i18n.webInvestigationDescription,
  },
  'report-writing': {
    title: i18n.reportWritingTitle,
    description: i18n.reportWritingDescription,
  },
  'wooyun-legacy': {
    title: i18n.wooyunLegacyTitle,
    description: i18n.wooyunLegacyDescription,
  },
};
