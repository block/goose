# macOS App Signing and Notarization

This guide explains how to sign and notarize the Goose Electron app for distribution.

## Prerequisites

1. **Apple Developer Account** (paid, $99/year)
2. **Developer ID Application Certificate** installed in Keychain
3. **App-specific password** for notarization

## Getting a Developer ID Certificate

**Note:** You currently have an "Apple Development" certificate, which is for App Store development. For ad-hoc distribution outside the App Store, you need a "Developer ID Application" certificate.

### Get Developer ID Certificate

1. Go to <https://developer.apple.com/account/resources/certificates>
2. Click the "+" button to create a new certificate
3. Select "Developer ID Application" (under "Software")
4. Follow the instructions to create a CSR (Certificate Signing Request):
   - Open Keychain Access
   - Menu: Keychain Access → Certificate Assistant → Request a Certificate from a Certificate Authority
   - Enter your email and name
   - Select "Saved to disk"
5. Upload the CSR file
6. Download the certificate and double-click to install it in Keychain

After installation, verify it appears:
```bash
security find-identity -v -p codesigning
```

You should see a line with "Developer ID Application".

## Setup

### 1. Find Your Certificate Identity

```bash
security find-identity -v -p codesigning
```

Look for a line like:
```
1) XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX "Developer ID Application: Your Name (TEAM_ID)"
```

### 2. Create App-Specific Password

1. Go to <https://appleid.apple.com>
2. Sign in with your Apple ID
3. Navigate to "Security" → "App-Specific Passwords"
4. Click "Generate password..."
5. Name it "Goose Notarization" (or similar)
6. Save the generated password (it looks like `xxxx-xxxx-xxxx-xxxx`)

### 3. Set Environment Variables

Create a `.env.signing` file in `ui/desktop/`:

```bash
# Your Developer ID Application certificate name (full string from step 1)
APPLE_IDENTITY="Developer ID Application: Your Name (TEAM_ID)"

# Your Apple ID email
APPLE_ID="your.email@example.com"

# App-specific password from step 2
APPLE_APP_SPECIFIC_PASSWORD="xxxx-xxxx-xxxx-xxxx"

# Your Team ID (the 10-character code in parentheses from step 1)
APPLE_TEAM_ID="TEAM_ID"
```

**Security Note:** Add `.env.signing` to `.gitignore` to keep credentials private!

## Building with Signing

### Option 1: Without Notarization (Faster, for testing)

Just set `APPLE_IDENTITY`:

```bash
cd ui/desktop
export APPLE_IDENTITY="Developer ID Application: Your Name (TEAM_ID)"
npm run bundle:default
```

The app will be signed but not notarized. Recipients need to right-click → Open first time.

### Option 2: Full Signing + Notarization (Recommended for distribution)

Set all environment variables:

```bash
cd ui/desktop
source .env.signing  # or manually export the variables
npm run bundle:default
```

This will:
1. Build the app
2. Sign it with your certificate
3. Submit it to Apple for notarization (takes 2-10 minutes)
4. Staple the notarization ticket to the app

### Option 3: Using `just make-ui`

From the project root:

```bash
# Export your signing credentials first
export APPLE_IDENTITY="Developer ID Application: Your Name (TEAM_ID)"
export APPLE_ID="your.email@example.com"
export APPLE_APP_SPECIFIC_PASSWORD="xxxx-xxxx-xxxx-xxxx"
export APPLE_TEAM_ID="TEAM_ID"

# Then build
just make-ui
```

## Verification

### Check if app is signed:

```bash
codesign -dv --verbose=4 out/Goose-darwin-arm64/Goose.app
```

Should show your identity and entitlements.

### Check if app is notarized:

```bash
spctl -a -vvv -t install out/Goose-darwin-arm64/Goose.app
```

Should say: `accepted` and `source=Notarized Developer ID`

## Troubleshooting

### "No identity found" error

- Make sure your Developer ID certificate is installed in Keychain
- Run `security find-identity -v -p codesigning` to verify

### Notarization fails

- Check your Apple ID and app-specific password are correct
- Ensure you have an active Apple Developer Program membership
- Check notarization logs: the build will output an ID you can use with:
  ```bash
  xcrun notarytool log <submission-id> --apple-id your.email@example.com --password xxxx-xxxx-xxxx-xxxx --team-id TEAM_ID
  ```

### "App is damaged" error on recipient's machine

- The app wasn't properly notarized
- Recipient can bypass with: `xattr -cr /path/to/Goose.app` (but this defeats the purpose)

## Distribution

After successful signing and notarization, the app is in:
```
out/Goose-darwin-arm64/Goose.app
```

The zip file:
```
out/Goose-darwin-arm64/Goose.zip
```

Recipients can:
1. Download and unzip
2. Drag to Applications folder
3. Double-click to run (no warnings!)

## Ad-Hoc Distribution (No Notarization)

If you just want to sign for yourself or trusted testers:

1. Only set `APPLE_IDENTITY`
2. Build the app
3. Recipients run: `xattr -cr /path/to/Goose.app` once
4. Or right-click → Open first time

This is faster but less polished for end users.
