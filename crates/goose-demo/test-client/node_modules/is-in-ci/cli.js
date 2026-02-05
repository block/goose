#!/usr/bin/env node
import process from 'node:process';
import isInCi from './index.js';

process.exit(isInCi ? 0 : 1);
