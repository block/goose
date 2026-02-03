#!/usr/bin/env bun

import yargs from 'yargs';
import { hideBin } from 'yargs/helpers';
import * as addCommand from './commands/add';
import * as listCommand from './commands/list';
import * as completeCommand from './commands/complete';
import * as deleteCommand from './commands/delete';
import * as statsCommand from './commands/stats';
import { formatError } from './utils/formatter';

// Setup yargs
const cli = yargs(hideBin(process.argv))
  .scriptName('task-manager')
  .usage('$0 <command> [options]')
  .command(addCommand)
  .command(listCommand)
  .command(completeCommand)
  .command(deleteCommand)
  .command(statsCommand)
  .demandCommand(1, 'You need to specify a command')
  .help('h')
  .alias('h', 'help')
  .version('1.0.0')
  .alias('v', 'version')
  .strict()
  .fail((msg, err) => {
    // Custom error handler
    if (err) {
      console.log(formatError(err.message));
    } else if (msg) {
      console.log(formatError(msg));
    }
    console.log('\nUse --help for usage information');
    process.exit(1);
  });

// Parse and execute
cli.parse();
