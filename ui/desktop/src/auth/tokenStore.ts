import fs from 'node:fs'
import path from 'node:path'

export type StoredTokens = {
  accessToken: string
  refreshToken?: string
  idToken?: string
  expiresAt: number
  tokenType?: string
  scope?: string
}

export type SafeStorageLike = {
  isEncryptionAvailable: () => boolean
  encryptString: (plain: string) => Buffer
  decryptString: (encrypted: Buffer) => string
}

export class TokenStore {
  private readonly filePath: string
  private readonly safeStorage: SafeStorageLike

  constructor(userDataPath: string, safeStorage: SafeStorageLike) {
    this.filePath = path.join(userDataPath, 'zitadel-tokens.bin')
    this.safeStorage = safeStorage
  }

  save(tokens: StoredTokens): void {
    if (!this.safeStorage.isEncryptionAvailable()) {
      throw new Error('safeStorage encryption is not available')
    }
    const payload = Buffer.from(JSON.stringify(tokens), 'utf8')
    const encrypted = this.safeStorage.encryptString(payload.toString('utf8'))
    fs.writeFileSync(this.filePath, encrypted)
  }

  load(): StoredTokens | null {
    if (!fs.existsSync(this.filePath)) return null
    if (!this.safeStorage.isEncryptionAvailable()) return null
    try {
      const encrypted = fs.readFileSync(this.filePath)
      const plain = this.safeStorage.decryptString(encrypted)
      return JSON.parse(plain) as StoredTokens
    } catch {
      return null
    }
  }

  clear(): void {
    if (fs.existsSync(this.filePath)) {
      fs.unlinkSync(this.filePath)
    }
  }
}
