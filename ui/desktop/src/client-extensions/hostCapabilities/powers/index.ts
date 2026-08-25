export { platformPower } from './platform';
export { goosePower } from './goose';
export { processPower } from './process';

import { goosePower } from './goose';
import { platformPower } from './platform';
import { processPower } from './process';

export const COMMON_HOST_POWERS = [platformPower, goosePower, processPower] as const;

export type CommonHostPowerId = (typeof COMMON_HOST_POWERS)[number]['id'];
