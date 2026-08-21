import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  channelMatchesIssue,
  issueReferenceFromChannel,
  readCoreTeam,
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
      source: "legacy-name",
    },
  );
});

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
