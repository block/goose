import { listSkills, SkillInfo } from '../api';
import { getInitialWorkingDir } from '../utils/workingDir';

export const listAvailableSkills = async (): Promise<SkillInfo[]> => {
  try {
    const response = await listSkills({
      query: {
        working_dir: getInitialWorkingDir(),
      },
    });
    return response?.data?.skills ?? [];
  } catch (error) {
    console.warn('Failed to list skills:', error);
    return [];
  }
};
