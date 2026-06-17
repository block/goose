# Security Goose signed release handoff panel

## Current snapshot

As of 2026-06-15, the Security Goose signed release candidate is in this state:

- target repo: `Ramos-dev/security-goose`
- candidate branch: `codex/goose-v1a-bootstrap`
- candidate commits:
  - `a748cd762` `feat: bootstrap security goose v1a candidate`
  - `811e5c7b2` `ci: fail fast on missing apple signing env`
- GitHub readiness: ready
  - `branch_on_target_repo=yes`
  - `required_workflow_files_present=yes`
  - `required_workflows_registered=yes`
  - `signing_environment_present=yes`
  - `ready_for_remote_signed_rehearsal=yes`
- latest real signed rehearsal:
  - run: `27528660028`
  - URL: <https://github.com/Ramos-dev/security-goose/actions/runs/27528660028>
  - result: arm64 / x64 both failed in `Validate Apple signing preflight`
  - root cause: Apple signing secrets are still missing

The blocker is not in Security Goose product code. The blocker is Apple release material that has not been injected into the target repo:

- `APPLE_CERTIFICATE_BASE64`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_TEAM_ID`
- `APPLE_ID`
- `APPLE_ID_PASSWORD`

Because no signed candidate zip was produced, signed install acceptance is still pending.

## Who should execute this panel

The executor needs all of the following:

- admin or equivalent workflow access on `Ramos-dev/security-goose`
- permission to read or set secrets in the `signing` GitHub environment
- a valid `Developer ID Application` certificate exported as `.p12`
- the certificate password
- an Apple Developer account with notarization permission
- the Apple ID app-specific password or equivalent notarization credential expected by the current workflow

## Step 1: Prepare the Apple inputs locally

Use a shell on a trusted machine and export the required values:

```bash
export APPLE_CERTIFICATE_BASE64="$(base64 -i /absolute/path/to/developer-id-application.p12 | tr -d '\n')"
export APPLE_CERTIFICATE_PASSWORD='replace-with-p12-password'
export APPLE_TEAM_ID='replace-with-team-id'
export APPLE_ID='replace-with-apple-id'
export APPLE_ID_PASSWORD='replace-with-app-specific-password'
```

Quick sanity check before touching GitHub:

```bash
GOOSE_DESKTOP_SIGN=true node scripts/check-security-apple-signing-env.mjs --require-signed
```

Expected result:

- `requested_mode=signed`
- `ready_for_signed_release=yes`

## Step 2: Inject secrets into the target repo `signing` environment

Run the following commands from a shell already authenticated with `gh`:

```bash
printf '%s' "$APPLE_CERTIFICATE_BASE64" | gh secret set APPLE_CERTIFICATE_BASE64 --repo Ramos-dev/security-goose --env signing
printf '%s' "$APPLE_CERTIFICATE_PASSWORD" | gh secret set APPLE_CERTIFICATE_PASSWORD --repo Ramos-dev/security-goose --env signing
printf '%s' "$APPLE_TEAM_ID" | gh secret set APPLE_TEAM_ID --repo Ramos-dev/security-goose --env signing
printf '%s' "$APPLE_ID" | gh secret set APPLE_ID --repo Ramos-dev/security-goose --env signing
printf '%s' "$APPLE_ID_PASSWORD" | gh secret set APPLE_ID_PASSWORD --repo Ramos-dev/security-goose --env signing
```

Confirm that GitHub now sees all five names:

```bash
gh api repos/Ramos-dev/security-goose/environments/signing/secrets \
  --jq '{total_count:.total_count, names:[.secrets[].name]}'
```

Expected result:

- `total_count` is at least `5`
- `names` contains all five Apple entries

## Step 3: Reconfirm workflow readiness

From the candidate worktree:

```bash
node scripts/check-security-github-release-readiness.mjs
```

Expected result:

- `branch_on_target_repo=yes`
- `required_workflow_files_present=yes`
- `required_workflows_registered=yes`
- `signing_environment_present=yes`
- `ready_for_remote_signed_rehearsal=yes`

## Step 4: Trigger the real signed rehearsal

Use the workflow file path, not the display name:

```bash
gh workflow run bundle-desktop-manual.yml \
  -R Ramos-dev/security-goose \
  --ref codex/goose-v1a-bootstrap \
  -f branch=codex/goose-v1a-bootstrap \
  -f signing=true \
  -f environment=signing
```

Fetch the newest workflow-dispatch run on the candidate branch:

```bash
gh run list \
  -R Ramos-dev/security-goose \
  --branch codex/goose-v1a-bootstrap \
  --event workflow_dispatch \
  --limit 5 \
  --json databaseId,displayTitle,headSha,status,conclusion,url,createdAt
```

Watch the chosen run:

```bash
RUN_ID='replace-with-run-id'
gh run watch "$RUN_ID" -R Ramos-dev/security-goose --exit-status
```

Expected success shape for both arm64 and x64 jobs:

- `Validate Apple signing preflight` passes
- `Build App` passes
- `Validate bundle metadata and signing boundary` passes
- release evidence artifacts are uploaded

## Step 5: Download and inspect evidence artifacts

```bash
RUN_ID='replace-with-run-id'
EVIDENCE_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/security-goose-signed-evidence.XXXXXX")"

gh run download "$RUN_ID" -R Ramos-dev/security-goose \
  -n Security-Goose-macos-release-evidence-arm64 \
  -D "$EVIDENCE_ROOT/arm64"

gh run download "$RUN_ID" -R Ramos-dev/security-goose \
  -n Security-Goose-macos-release-evidence-x64 \
  -D "$EVIDENCE_ROOT/x64"
```

Inspect:

```bash
sed -n '1,220p' "$EVIDENCE_ROOT/arm64/summary.md"
sed -n '1,220p' "$EVIDENCE_ROOT/x64/summary.md"
```

Signed success shape:

- `requested_mode=signed`
- `ready_for_signed_release=yes`
- `bundle_check=ok`
- `codesign_team` is not `not set`
- `spctl` contains `accepted`
- `stapler` contains `The validate action worked.`

If either architecture fails this gate, the release remains `No-Go`.

## Step 6: Download the signed candidate zip

At minimum, download the arm64 app artifact:

```bash
RUN_ID='replace-with-run-id'
CANDIDATE_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/security-goose-signed-candidate.XXXXXX")"

gh run download "$RUN_ID" -R Ramos-dev/security-goose \
  -n Goose-darwin-arm64 \
  -D "$CANDIDATE_ROOT/arm64"
```

Unpack it:

```bash
cd "$CANDIDATE_ROOT/arm64"
ditto -x -k "Security Goose.zip" .
```

The signed app path should be:

```bash
export SECURITY_GOOSE_APP="$CANDIDATE_ROOT/arm64/Security Goose.app"
```

## Step 7: Signed install acceptance checklist

Record quarantine and signing evidence before first launch:

```bash
xattr -l "$SECURITY_GOOSE_APP" || true
spctl -a -vv "$SECURITY_GOOSE_APP"
xcrun stapler validate "$SECURITY_GOOSE_APP"
```

If the app was downloaded through a channel that adds quarantine, record that fact first, then clear it only for local acceptance:

```bash
xattr -dr com.apple.quarantine "$SECURITY_GOOSE_APP"
```

Then verify all of the following:

1. Finder shows the app name as `Security Goose`.
2. The app icon is the Security Goose icon, not the default Electron icon.
3. First launch opens the Security Goose main UI, not bare Electron `default_app`.
4. Default copy is `zh-CN` aligned.
5. Settings still show the expected provider/model defaults from `distro/security-cn/config/desktop-env.example` and `model-catalog.json`.
6. The Task Templates view still shows the six built-in security task templates.
7. `漏洞研判` and `告警分析` still work as task-template-backed entry paths.
8. The six built-in security tasks still work through Goose-native task template entry paths.
9. Recommended security extensions state is still visible and not regressed.

If a real Intel Mac is not available, keep x64 limited to CI signed evidence and record that real-machine acceptance is still pending.

## Step 8: Final Go / No-Go template

Use the following template for the final release decision:

```md
# Security Goose signed release decision

- Candidate branch: `codex/goose-v1a-bootstrap`
- Candidate commit: `replace-with-head-sha`
- Signed rehearsal run: `replace-with-run-url`

## Evidence

- arm64 evidence: `pass|fail`
- x64 evidence: `pass|fail`
- arm64 `ready_for_signed_release=yes`: `yes|no`
- arm64 `bundle_check=ok`: `yes|no`
- arm64 `spctl accepted`: `yes|no`
- arm64 `stapler validate`: `yes|no`
- x64 `ready_for_signed_release=yes`: `yes|no`
- x64 `bundle_check=ok`: `yes|no`
- x64 `spctl accepted`: `yes|no`
- x64 `stapler validate`: `yes|no`

## Install acceptance

- arm64 signed app launch acceptance: `pass|fail|not run`
- x64 real-machine acceptance: `pass|fail|not run`
- zh-CN default copy intact: `yes|no`
- provider/model defaults intact: `yes|no`
- Recipes security tasks intact: `yes|no`
- six built-in security recipes intact: `yes|no`

## Decision

- Final decision: `Go|No-Go`
- Reason:
  - `replace-with-one-paragraph-release-decision`

## Remaining blockers

- `none` or list exact blockers
```

## Current decision before Apple secrets exist

Right now the correct release status is:

- `No-Go`

Reason:

- the repo and workflow path are ready
- the real signed rehearsal already proved the blocker is missing Apple signing material
- no signed candidate zip has been produced yet
- signed install acceptance therefore cannot be completed yet
