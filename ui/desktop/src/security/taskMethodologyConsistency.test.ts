import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

import {
  SECURITY_TASK_IDS,
  getSecurityTaskById,
  resolveSecurityTaskLaunchConfig,
} from './taskCatalog';

const repoRoot = path.resolve(process.cwd(), '..', '..');
const availableRecipeIds = new Set(
  SECURITY_TASK_IDS.map((taskId) => getSecurityTaskById(taskId).recipeId).filter(
    (value): value is string => typeof value === 'string'
  )
);

const taskContracts = {
  'vuln-triage': ['漏洞结论', '利用条件', '风险等级', '关键证据', '不确定项', '建议下一步'],
  'alert-investigation': ['真实性判断', '触发原因', '受影响资产', '风险等级', '建议处置动作', '待补充数据'],
  'ioc-analysis': ['IOC 摘要', '关键发现', '关联关系', '可信度判断', '建议下一步', '待确认项'],
  'web-investigation': ['页面主题', '可疑指标', '关联 IOC', '可信度判断', '建议下一步'],
  'report-writing': ['背景', '核心结论', '关键证据', '影响与风险', '建议动作', '待确认项'],
  'wooyun-legacy': ['当前结论', '执行模式', '业务流程与领域', '测试假设', '关键证据', '风险与影响', '修复建议', '待确认项'],
} as const;

function readRepoFile(relativePath: string): string {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8');
}

describe('security task methodology consistency', () => {
  it('keeps recipe-ready starter prompts aligned with output sections and telemetry boundary', () => {
    for (const taskId of SECURITY_TASK_IDS) {
      const prompt = resolveSecurityTaskLaunchConfig(taskId, 'zh-CN', availableRecipeIds).starterPrompt;

      expect(prompt).toContain('当前任务以');
      expect(prompt).toContain('如果当前会话不能直接观测到映射 skill 已加载');

      for (const section of taskContracts[taskId]) {
        expect(prompt).toContain(section);
      }
    }
  });

  it('keeps bundled recipes aligned with skill methodology notice and output sections', () => {
    for (const taskId of SECURITY_TASK_IDS) {
      const task = getSecurityTaskById(taskId);
      const recipeSource = readRepoFile(`distro/security-cn/recipes/${task.recipeId}.yaml.example`);

      expect(recipeSource).toContain('message:');
      expect(recipeSource).toContain(task.skillId);
      expect(recipeSource).toContain('如果当前 Goose 会话不能直接观测到技能加载');

      for (const section of taskContracts[taskId]) {
        expect(recipeSource).toContain(section);
      }
    }
  });

  it('keeps the WooYun recipe aligned with local wrapper and upstream-enhanced execution modes', () => {
    const wooyunRecipeSource = readRepoFile('distro/security-cn/recipes/wooyun-legacy.yaml.example');

    expect(wooyunRecipeSource).toContain('本地包装模式');
    expect(wooyunRecipeSource).toContain('上游参考增强模式');
    expect(wooyunRecipeSource).toContain(
      '.agents/skills/wooyun-legacy/external/upstream/references/'
    );
  });
});
