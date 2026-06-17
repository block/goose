import type { ManagedSkillsInventory } from './managedSkills';

export function getManagedLocalVisibleSkillNames(inventory: ManagedSkillsInventory): string[] {
  return inventory.localSkills
    .filter((skill) => skill.status !== 'invalid')
    .map((skill) => skill.id)
    .sort();
}
