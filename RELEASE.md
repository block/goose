# Making a Release

You'll generally create one of two release types: a regular feature release (minor version bump) or a bug-fixing patch release (patch version bump). Regular releases start on main, while patch releases start with an existing release tag. goose uses GitHub actions to automate the creation of release branches. The actual releases are triggered by tags.

## Minor version releases

These are typically done once per week. There is an [action](https://github.com/block/goose/actions/workflows/minor-release.yaml) that cuts the branch every Tuesday, but it can also be triggered manually. Commits from main can be cherry-picked into this branch.

To trigger the release, find [the corresponding PR](https://github.com/block/goose/pulls?q=is%3Apr+%22chore%28release%29%22+%22%28minor%29%22+author%3Aapp%2Fgithub-actions+) and follow the instructions in the PR description.

## Patch version releases

Minor and patch releases both trigger the creation of a branch for a follow-on patch release. These branches can be used to create patch releases, or can be safely ignored/closed.

To trigger the release, find [the corresponding PR](https://github.com/block/goose/pulls?q=is%3Apr+%22chore%28release%29%22+%22%28patch%29%22+author%3Aapp%2Fgithub-actions+) and follow the instructions in the PR description.
