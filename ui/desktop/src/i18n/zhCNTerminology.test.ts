import { describe, expect, it } from 'vitest';
import zhCN from './messages/zh-CN.json';

function message(id: string): string {
  return zhCN[id as keyof typeof zhCN]?.defaultMessage ?? '';
}

describe('zh-CN terminology', () => {
  it('uses productized labels for recipes and automation surfaces', () => {
    expect(message('navigation.itemRecipes')).toBe('任务模板');
    expect(message('navigation.itemScheduler')).toBe('自动化');
    expect(message('navigation.itemSessions')).toBe('历史会话');
    expect(message('recipesView.recipesTitle')).toBe('任务模板');
    expect(message('schedulesView.scheduler')).toBe('自动化');
    expect(message('chatInput.createRecipeFromSession')).toBe('保存为任务模板');
    expect(message('baseChat.recipeCreatedTitle')).toBe('任务模板创建成功！');
  });

  it('localizes security task terminology without raw English fallback copy', () => {
    expect(message('securityTasks.badgeReady')).toBe('模板');
    expect(message('securityTasks.mappingRecipe')).toBe('任务模板');
    expect(message('securityTasks.mappingSkill')).toBe('技能');
    expect(message('securityTasks.extensionStatusDisabledStub')).toBe('禁用占位');
    expect(message('securityTasks.savedRecipesTitle')).toBe('已保存任务模板');
    expect(message('securityTasks.sectionDescription')).toContain('任务模板');
    expect(message('launcher.securityTasksDescription')).not.toContain('recipe');
    expect(message('launcher.securityTasksDescription')).not.toContain('skill');
    expect(message('launcher.securityTasksDescription')).not.toContain('disabled stub');
    expect(message('launcher.securityTasksDescription')).not.toContain('blocker');
  });
});
