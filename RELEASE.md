# Making a Release

You'll generally create one of two release types: a regular feature release (minor version bump like 1.20) or a bug-fixing patch release (patch version bump like 1.20.1). 

Regular releases start on main, while patch releases start with an existing release tag. goose uses GitHub actions to automate the creation of release branches. The actual releases are triggered by tags.
For bug-fixing releases, you will cherry-pick fixes into that branch, test, and then release from it. 

## Minor version releases

These are typically done once per week. There is an [action](https://github.com/block/goose/actions/workflows/minor-release.yaml) that cuts the branch every Tuesday, but it can also be triggered manually. Commits from main can be cherry-picked into this branch as needed before release.

To trigger the release, find [the corresponding PR](https://github.com/block/goose/pulls?q=is%3Apr+%22chore%28release%29%22+%22%28minor%29%22+author%3Aapp%2Fgithub-actions+) and follow the instructions in the PR description.

## Patch version releases

Minor and patch releases both trigger the creation of a branch for a follow-on patch release. These branches can be used to create patch releases, or can be safely ignored/closed.
You can cherry pick fixes into this branch. 

To trigger the release, find [the corresponding PR](https://github.com/block/goose/pulls?q=is%3Apr+%22chore%28release%29%22+%22%28patch%29%22+author%3Aapp%2Fgithub-actions+) and follow the instructions in the PR description.


## High level release flow:

* check out and cherry pick (if needed) changes to the branch you are going to release (eg the patch branch)
* Test locally if you can (just run-ui)
* Push changes to that branch, wait for build
* Download and test the .zip from the release PR
* If happy, follow the instructions on the release PR to tag and release (tagging will trigger the real release from there)
 
