import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  ISSUE_FIRST_COMMENT_MARKER,
  bestMatchingIssueChannels,
  channelMatchesIssue,
  coreAssignees,
  firstHumanCommentAfter,
  getOpenIssues,
  getOpenPullRequests,
  getProjectIssues,
  issueFirstNotice,
  issueReferenceFromChannel,
  linkedPullRequestForVerification,
  pullRequestIssueMentions,
  pullRequestWork,
  readCoreTeam,
  selectRecentQueueEntries,
} from "./github_manager.mjs";

const issue = {
  repository: "aaif-goose/goose",
  number: 123,
};

test("matches current and legacy issue channels", () => {
  assert.equal(
    channelMatchesIssue(
      {
        name: "123 short title",
        description:
          "Discussion for aaif-goose/goose#123: https://github.com/aaif-goose/goose/issues/123",
      },
      issue,
    ),
    true,
  );
  assert.equal(
    channelMatchesIssue({ name: "aaif-goose/goose #123" }, issue),
    true,
  );
  assert.equal(channelMatchesIssue({ name: "#123 title" }, issue), true);
});

test("does not match another repository from an explicit reference", () => {
  assert.equal(
    channelMatchesIssue(
      {
        name: "123 title",
        description: "https://github.com/example/elsewhere/issues/123",
      },
      issue,
    ),
    false,
  );
});

test("parses a legacy channel ending at the issue number", () => {
  assert.deepEqual(
    issueReferenceFromChannel({ name: "aaif-goose/goose #123" }),
    {
      repository: "aaif-goose/goose",
      number: 123,
      kind: null,
      source: "legacy-name",
    },
  );
});

test("prefers an explicit issue channel over a bare numeric name", () => {
  const explicit = {
    channel_id: "explicit",
    name: "123 real issue",
    description: "https://github.com/aaif-goose/goose/issues/123",
  };
  const stray = {
    channel_id: "stray",
    name: "123 followups",
  };
  assert.deepEqual(bestMatchingIssueChannels([stray, explicit], issue), [explicit]);
});

test("does not adopt a pull-request channel", () => {
  assert.equal(
    channelMatchesIssue(
      {
        name: "123 pull request",
        description: "https://github.com/aaif-goose/goose/pull/123",
      },
      issue,
    ),
    false,
  );
});

test("reports malformed and deferred queue entries", () => {
  const { entries, ignored } = selectRecentQueueEntries(
    [
      { id: "invalid", created_at: "today", content: "invalid" },
      { id: "old", created_at: 1, content: "old" },
      { id: "new", created_at: 2, content: "new" },
    ],
    1,
    (message) => [message.content],
  );
  assert.deepEqual(entries.map((entry) => entry.link), ["new"]);
  assert.deepEqual(ignored, [
    { message_id: "invalid", reason: "invalid-created-at" },
    {
      message_id: "old",
      link: "old",
      reason: "outside-recent-window",
    },
  ]);
});

test("retries a project read that changes while being listed", () => {
  let calls = 0;
  const issueItem = {
    content: {
      type: "Issue",
      repository: "aaif-goose/goose",
      number: 123,
    },
  };
  const result = getProjectIssues(
    () => {
      calls += 1;
      return calls === 1
        ? { totalCount: 2, items: [issueItem] }
        : { totalCount: 1, items: [issueItem] };
    },
    {
      command: "gh",
      projectNumber: 1,
      projectOwner: "aaif-goose",
      projectLimit: 1000,
      repository: "aaif-goose/goose",
    },
  );
  assert.equal(calls, 2);
  assert.equal(result.byNumber.get(123), issueItem);
});

test("matches project repository names without case sensitivity", () => {
  const issueItem = {
    content: {
      type: "Issue",
      repository: "AAIF-Goose/Goose",
      number: 123,
    },
  };
  const result = getProjectIssues(
    () => ({ totalCount: 1, items: [issueItem] }),
    {
      command: "gh",
      projectNumber: 1,
      projectOwner: "aaif-goose",
      projectLimit: 1000,
      repository: "aaif-goose/goose",
    },
  );
  assert.equal(result.byNumber.get(123), issueItem);
});

test("normalizes paginated REST issues and excludes pull requests", () => {
  const issues = getOpenIssues(
    () => [
      [
        {
          number: 123,
          title: "Issue",
          html_url: "https://github.com/aaif-goose/goose/issues/123",
          assignees: [{ login: "person" }],
        },
        { number: 124, pull_request: {} },
      ],
    ],
    { command: "gh", repository: "aaif-goose/goose" },
  );
  assert.deepEqual(issues, [
    {
      number: 123,
      title: "Issue",
      url: "https://github.com/aaif-goose/goose/issues/123",
      repository: "aaif-goose/goose",
      assignees: [{ login: "person" }],
    },
  ]);
});

test("lists enough pull request data to make issue-first decisions", () => {
  let arguments_;
  const pullRequests = getOpenPullRequests(
    (_command, receivedArguments) => {
      if (receivedArguments[0] === "pr") {
        arguments_ = receivedArguments;
        return [{ number: 123 }];
      }
      return [[{ number: 123, author_association: "NONE" }]];
    },
    { command: "gh", repository: "aaif-goose/goose", limit: 1000 },
  );
  assert.deepEqual(pullRequests, [
    { number: 123, authorAssociation: "NONE" },
  ]);
  assert.deepEqual(arguments_.slice(0, 8), [
    "pr",
    "list",
    "--repo",
    "aaif-goose/goose",
    "--state",
    "open",
    "--limit",
    "1000",
  ]);
  assert.match(arguments_.at(-1), /closingIssuesReferences/);
  assert.match(arguments_.at(-1), /comments/);
});

test("finds same-repository issue references in pull request bodies", () => {
  assert.deepEqual(
    pullRequestIssueMentions(
      [
        "Addresses #123.",
        "Implements aaif-goose/goose#124.",
        "See https://github.com/aaif-goose/goose/issues/125.",
        "Not https://github.com/example/elsewhere/issues/126.",
      ].join("\n"),
      "aaif-goose/goose",
    ),
    [123, 124, 125],
  );
});

test("builds the marked issue-first notice for the repository", () => {
  const notice = issueFirstNotice("aaif-goose/goose");
  assert.match(notice, new RegExp(ISSUE_FIRST_COMMENT_MARKER));
  assert.match(notice, /Closes #ISSUE_NUMBER/);
  assert.match(
    notice,
    /aaif-goose\/goose\/blob\/main\/CONTRIBUTING\.md#from-issue-to-pull-request/,
  );
});

test("omits pull requests that already close a repository issue", () => {
  const work = pullRequestWork(
    {
      number: 123,
      closingIssuesReferences: [
        {
          number: 456,
          repository: { owner: { login: "aaif-goose" }, name: "goose" },
        },
      ],
    },
    pullRequestOptions(),
  );
  assert.equal(work, null);
});

test("returns an unclosed mention when another issue already closes", () => {
  const work = pullRequestWork(
    pullRequest({
      body: "Fixes #456. Also implements #789.",
      closingIssuesReferences: [
        {
          number: 456,
          url: "https://github.com/aaif-goose/goose/issues/456",
          repository: { owner: { login: "aaif-goose" }, name: "goose" },
        },
      ],
    }),
    pullRequestOptions(),
  );
  assert.deepEqual(work.unclosed_issue_mentions, [789]);
  assert.equal(work.eligible_for_issue_first, false);
});

test("returns issue mentions that need closing-language review", () => {
  const work = pullRequestWork(
    pullRequest({ body: "This implements #456." }),
    pullRequestOptions(),
  );
  assert.deepEqual(work.issue_mentions, [456]);
  assert.equal(work.eligible_for_issue_first, true);
  assert.equal(work.warning_state, "not-commented");
});

test("marks an unanswered issue-first notice stale after three days", () => {
  const warning = comment({
    id: "warning",
    body: `${ISSUE_FIRST_COMMENT_MARKER}\nPlease read CONTRIBUTING.md.`,
    createdAt: "2026-09-04T00:00:00Z",
    author: { login: "DOsinga" },
    viewerDidAuthor: true,
  });
  const stale = pullRequestWork(
    pullRequest({ comments: [warning] }),
    pullRequestOptions(),
  );
  assert.equal(stale.warning_state, "stale");

  const answered = pullRequestWork(
    pullRequest({
      comments: [
        warning,
        comment({
          id: "answer",
          createdAt: "2026-09-07T12:00:00Z",
          author: { login: "contributor" },
        }),
      ],
    }),
    pullRequestOptions(),
  );
  assert.equal(answered.warning_state, "answered");
});

test("does not issue-first police drafts, internal PRs, or bot PRs", () => {
  assert.equal(
    pullRequestWork(pullRequest({ isDraft: true }), pullRequestOptions()),
    null,
  );
  assert.equal(
    pullRequestWork(
      pullRequest({ author: { login: "core", is_bot: false } }),
      pullRequestOptions(),
    ),
    null,
  );
  assert.equal(
    pullRequestWork(
      pullRequest({ author: { login: "dependabot[bot]", is_bot: true } }),
      pullRequestOptions(),
    ),
    null,
  );
  assert.equal(
    pullRequestWork(
      pullRequest({ authorAssociation: "COLLABORATOR" }),
      pullRequestOptions(),
    ),
    null,
  );
  assert.equal(
    pullRequestWork(
      pullRequest({ authorAssociation: "UNKNOWN" }),
      pullRequestOptions(),
    ),
    null,
  );
});

test("ignores Dependabot even when its body mentions an unclosed issue", () => {
  assert.equal(
    pullRequestWork(
      pullRequest({
        author: { login: "dependabot[bot]", is_bot: true },
        body: "Updates a dependency for #456.",
      }),
      pullRequestOptions(),
    ),
    null,
  );
});

function pullRequest(overrides = {}) {
  return {
    number: 123,
    title: "A change",
    url: "https://github.com/aaif-goose/goose/pull/123",
    body: "No issue here.",
    createdAt: "2026-09-01T00:00:00Z",
    updatedAt: "2026-09-01T00:00:00Z",
    isDraft: false,
    author: { login: "contributor", is_bot: false },
    authorAssociation: "NONE",
    comments: [],
    closingIssuesReferences: [],
    ...overrides,
  };
}

function comment(overrides = {}) {
  return {
    id: "comment",
    body: "A comment",
    createdAt: "2026-09-01T00:00:00Z",
    url: "https://github.com/aaif-goose/goose/pull/123#issuecomment-1",
    author: { login: "contributor" },
    viewerDidAuthor: false,
    ...overrides,
  };
}

function pullRequestOptions() {
  return {
    repository: "aaif-goose/goose",
    coreTeamGithub: new Set(["core"]),
    viewerLogin: "DOsinga",
    now: Date.parse("2026-09-08T00:00:00Z"),
    staleAfterMilliseconds: 3 * 24 * 60 * 60 * 1000,
  };
}

test("uses one complete core-team schema", (context) => {
  const directory = mkdtempSync(join(tmpdir(), "buzz-core-team-"));
  context.after(() => rmSync(directory, { recursive: true }));
  const path = join(directory, "core-team.json");
  const person = {
    name: "Person",
    github: "person",
    pubkey: "1".repeat(64),
    capacity: 1,
    interest: ["testing"],
    bots: { Bot: "2".repeat(64) },
  };
  writeFileSync(
    path,
    JSON.stringify({ owners: [person], members: [] }),
  );

  const team = readCoreTeam(path);
  assert.equal(team.people.length, 1);
  assert.equal(team.botsByPerson.get(person.pubkey).length, 1);

  delete person.capacity;
  writeFileSync(
    path,
    JSON.stringify({ owners: [person], members: [] }),
  );
  assert.throws(() => readCoreTeam(path), /positive capacity/);
});

test("finds only core-team assignees without changing their spelling", () => {
  assert.deepEqual(
    coreAssignees(
      [{ login: "CoreMember" }, { login: "contributor" }, "SECOND"],
      new Set(["coremember", "second"]),
    ),
    ["CoreMember", "SECOND"],
  );
});

test("uses the first human comment after a phase transition", () => {
  const comment = firstHumanCommentAfter(
    [
      {
        id: 1,
        created_at: "2026-08-27T10:00:00Z",
        user: { login: "before", type: "User" },
      },
      {
        id: 2,
        created_at: "2026-08-27T12:00:01Z",
        user: { login: "automation", type: "Bot" },
      },
      {
        id: 4,
        created_at: "2026-08-27T12:00:03Z",
        user: { login: "later", type: "User" },
      },
      {
        id: 3,
        created_at: "2026-08-27T12:00:02Z",
        user: { login: "first", type: "User" },
      },
    ],
    "2026-08-27T12:00:00Z",
  );
  assert.equal(comment.id, 3);
});

test("selects one active linked pull request and ignores closed work", () => {
  const pullRequest = linkedPullRequestForVerification(
    [
      {
        url: "https://github.com/aaif-goose/goose/pull/1",
        state: "CLOSED",
        repository: { nameWithOwner: "aaif-goose/goose" },
      },
      {
        url: "https://github.com/aaif-goose/goose/pull/2",
        state: "OPEN",
        repository: { nameWithOwner: "aaif-goose/goose" },
      },
    ],
    "aaif-goose/goose",
  );
  assert.equal(pullRequest.url, "https://github.com/aaif-goose/goose/pull/2");
});

test("does not reconsider pull requests present during initialization", () => {
  assert.equal(
    linkedPullRequestForVerification(
      [
        {
          url: "https://github.com/aaif-goose/goose/pull/2",
          state: "OPEN",
          repository: { nameWithOwner: "aaif-goose/goose" },
        },
      ],
      "aaif-goose/goose",
      ["https://github.com/aaif-goose/goose/pull/2"],
    ),
    null,
  );
});

test("fails closed when more than one active pull request is linked", () => {
  assert.throws(
    () =>
      linkedPullRequestForVerification(
        [1, 2].map((number) => ({
          url: `https://github.com/aaif-goose/goose/pull/${number}`,
          state: "OPEN",
          repository: { nameWithOwner: "aaif-goose/goose" },
        })),
        "aaif-goose/goose",
      ),
    /More than one pull request/,
  );
});
