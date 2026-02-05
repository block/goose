import {env} from 'node:process';

const isInCi = env.CI !== '0'
	&& env.CI !== 'false'
	&& (
		'CI' in env
			|| 'CONTINUOUS_INTEGRATION' in env
			|| Object.keys(env).some(key => key.startsWith('CI_'))
	);

export default isInCi;
