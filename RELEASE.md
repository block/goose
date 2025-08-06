# Making a Release

You'll generally create one of two release types: a regular feature release (minor version bump) or a bug-fixing patch release (patch version bump). Regular releases start on main, while patch releases start with an existing release tag.

## Regular release from main

1. Check out the main branch.
2. Pick the new version. Use a new minor version (e.g. if the current latest release is 1.2.3, use 1.3.0). Save it using `export VERSION=<new version>`
3. Run `just prepare-release $VERSION`. This will create a branch `release/<version>`. Push this branch and open a PR into main. The diff should show version updates to Cargo.toml/package.json and their lock files.
4. This should trigger a signed macOS build -- Github actions will comment on the PR when it's ready. Test this build. When ready to make the release, proceed to the next step.
5. Tag the release: run `just tag-push` to create the tag and push it. This will start the build process for your new release.
6. Merge the PR you created in step 2.
7. Once the release is created on [Github](https://github.com/block/goose/releases), run `just release-notes <prior release>` to generate release notes. Copy these into the release description.

## Patch release

Follow the above steps, but rather than starting on main, start on the release tag you're interested in patching. Increment the patch version instead of minor (e.g. 1.2.3 -> 1.2.4). Bug fixes should be merged to main and then cherry-picked onto this branch.

1. Before proceeding, make sure any fixes you're looking to include in a patch are merged into main, if possible.
2. Check out the release you're patching using the tag (e.g `git checkout v1.3.0`). Set the version by incrementing the patch version (`export VERSION=1.3.1`).
3. Run `just prepare-release $VERSION`.
4. Cherry-pick the relevant fixes from the main branch.
5. Test this build. When ready to make the release, proceed to the next step.
6. Tag the release: run `just tag-push` to create the tag and push it. This will start the build process for your new release.
7. Once the release is created on [Github](https://github.com/block/goose/releases), run `just release-notes <prior release>` to generate release notes. Copy these into the release description.

Note that you won't merge this branch into main.
