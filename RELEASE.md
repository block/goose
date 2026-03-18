# Making a Release

You'll generally create one of two release types: a regular feature release (minor version bump like 1.20) or a bug-fixing patch release (patch version bump like 1.20.1). 

goose uses GitHub actions to automate the release process. The actual releases are triggered by tags.

## Minor version releases

These are typically done once per week. The process has two automated phases:

1. **Version bump PR** — An [action](https://github.com/block/goose/actions/workflows/minor-release.yaml) runs every Tuesday (or can be triggered manually) that creates a PR to bump the version on `main`. Review and merge this PR.

2. **Release branch + PR** — When the version bump PR merges, automation creates a `release/<version>` branch from `main` and opens a release PR with a QA checklist.

From there:
- Test locally if you can (`just run-ui`)
- Cherry-pick any last-minute fixes into the release branch if needed
- Download and test the .zip from the release PR
- When ready, follow the instructions on the release PR to tag and release

To trigger the release, find [the corresponding PR](https://github.com/block/goose/pulls?q=is%3Apr+%22chore%28release%29%22+author%3Aapp%2Fgithub-actions+) and follow the instructions in the PR description.

## Patch version releases

When a minor release is tagged, automation creates a patch release branch ready for hotfixes. Cherry-pick fixes into this branch, then trigger a patch release.

To trigger the release, find [the corresponding PR](https://github.com/block/goose/pulls?q=is%3Apr+%22chore%28release%29%22+%22%28patch%29%22+author%3Aapp%2Fgithub-actions+) and follow the instructions in the PR description.

## High level release flow:

```
minor-release (cron/manual)
  │
  ▼
version-bump PR → main (merge this)
  │
  ▼ (on merge)
release/<V> branch created from main
release PR opened (for QA, cherry-picks, testing)
  │
  ▼ (when ready)
git tag v<V> origin/release/<V> && git push origin v<V>
  │
  ├─► release PR auto-closed (no merge needed)
  ├─► release.yml builds & publishes
  └─► next patch release branch auto-created
```
