import process from 'node:process';
import onetime from 'onetime';
import signalExit from 'signal-exit';

const restoreCursor = onetime(() => {
	signalExit(() => {
		process.stderr.write('\u001B[?25h');
	}, {alwaysLast: true});
});

export default restoreCursor;
