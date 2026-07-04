# gosling Release Manual Testing Checklist

Download the release builds from this PR. Once a build is ready, the actions bot will post a comment on this PR
with instructions on how to download and sign.

## Have gosling produce a test plan

Open the release candidate desktop app and have gosling produce a test plan by pointing it at this PR. Use a prompt like

> Look at the notes in PR <release PR> and investigate potential risks in this release. After familiarizing yourself with the scope of each change, produce a suggested test plan that I should follow before publishing the release.

gosling will produce a plan. Follow this plan to finish testing.
