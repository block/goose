import { ipcMain, app, BrowserWindow } from 'electron';
import fs from 'node:fs/promises';
import fsSync from 'node:fs';
import path from 'node:path';
import crypto from 'crypto';

function calculateRecipeHash(recipe: unknown): string {
  const hash = crypto.createHash('sha256');
  hash.update(JSON.stringify(recipe));
  return hash.digest('hex');
}

async function getRecipeHashesDir(): Promise<string> {
  const userDataPath = app.getPath('userData');
  const hashesDir = path.join(userDataPath, 'recipe_hashes');
  await fs.mkdir(hashesDir, { recursive: true });
  return hashesDir;
}

function isBundledRecipeByTitleAndDescription(recipe: unknown): boolean {
  if (typeof recipe !== 'object' || recipe === null) return false;
  const title = (recipe as { title?: unknown }).title;
  const description = (recipe as { description?: unknown }).description;
  if (typeof title !== 'string' || typeof description !== 'string') return false;
  try {
    const listFile = path.join(app.getPath('userData'), 'bundled-recipe-titles.json');
    if (!fsSync.existsSync(listFile)) return false;
    const raw = fsSync.readFileSync(listFile, 'utf-8');
    const items: unknown = JSON.parse(raw);
    if (!Array.isArray(items)) return false;
    return items.some(
      (it) =>
        it != null &&
        typeof it === 'object' &&
        (it as { title?: unknown }).title === title &&
        (it as { description?: unknown }).description === description
    );
  } catch {
    return false;
  }
}

ipcMain.handle('has-accepted-recipe-before', async (_event, recipe) => {
  const hash = calculateRecipeHash(recipe);
  const hashFile = path.join(await getRecipeHashesDir(), `${hash}.hash`);
  try {
    await fs.access(hashFile);
    return true;
  } catch (err) {
    if (typeof err === 'object' && err !== null && 'code' in err && err.code === 'ENOENT') {
      return isBundledRecipeByTitleAndDescription(recipe);
    }
    throw err;
  }
});

ipcMain.handle('record-recipe-hash', async (_event, recipe) => {
  const hash = calculateRecipeHash(recipe);
  const filePath = path.join(await getRecipeHashesDir(), `${hash}.hash`);
  const timestamp = new Date().toISOString();
  await fs.writeFile(filePath, timestamp);
  return true;
});

ipcMain.on('close-window', () => {
  const currentWindow = BrowserWindow.getFocusedWindow();
  if (currentWindow) {
    currentWindow.close();
  }
});
