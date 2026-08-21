import { readFileSync } from "node:fs";

export function readCoreTeam(path) {
  let document;
  try {
    document = JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    throw new Error(`Could not read core team file ${path}: ${error.message}`);
  }

  if (!Array.isArray(document.owners) || !Array.isArray(document.members)) {
    throw new Error(`${path} must contain owners and members arrays.`);
  }

  const parsedPeople = [
    ...document.owners.map((entry) => person(entry, "owner", path)),
    ...document.members.map((entry) => person(entry, "member", path)),
  ];
  const people = parsedPeople.map(({ bots, ...entry }) => entry);
  if (people.length === 0) {
    throw new Error(`${path} has no people.`);
  }

  const byGithub = new Map();
  const byPubkey = new Map();
  for (const entry of people) {
    const github = entry.github.toLowerCase();
    if (byGithub.has(github)) {
      throw new Error(`More than one core team entry uses ${entry.github}.`);
    }
    if (byPubkey.has(entry.pubkey)) {
      throw new Error(`More than one core team entry uses ${entry.pubkey}.`);
    }
    byGithub.set(github, entry);
    byPubkey.set(entry.pubkey, entry);
  }

  return {
    people,
    owners: people.filter((entry) => entry.role === "owner"),
    members: people.filter((entry) => entry.role === "member"),
    byGithub,
    byPubkey,
    botsByPerson: new Map(
      parsedPeople.map((entry) => [entry.pubkey, entry.bots]),
    ),
  };
}

function person(entry, role, path) {
  if (
    !entry ||
    typeof entry.name !== "string" ||
    !entry.name.trim() ||
    typeof entry.github !== "string" ||
    !entry.github.trim() ||
    typeof entry.pubkey !== "string" ||
    !/^[0-9a-f]{64}$/i.test(entry.pubkey) ||
    typeof entry.capacity !== "number" ||
    !Number.isFinite(entry.capacity) ||
    entry.capacity <= 0 ||
    !Array.isArray(entry.interest) ||
    entry.interest.length === 0 ||
    entry.interest.some(
      (interest) => typeof interest !== "string" || !interest.trim(),
    )
  ) {
    throw new Error(
      `Every person in ${path} must have a name, GitHub handle, hexadecimal ` +
        "pubkey, positive capacity, and non-empty interest list.",
    );
  }

  const bots = entry.bots || {};
  if (
    typeof bots !== "object" ||
    Array.isArray(bots) ||
    Object.entries(bots).some(
      ([name, pubkey]) =>
        !name.trim() ||
        typeof pubkey !== "string" ||
        !/^[0-9a-f]{64}$/i.test(pubkey),
    )
  ) {
    throw new Error(
      `Bots for ${JSON.stringify(entry.name)} in ${path} must map names to hexadecimal pubkeys.`,
    );
  }

  return {
    name: entry.name.trim(),
    github: entry.github.trim(),
    pubkey: entry.pubkey.toLowerCase(),
    role,
    capacity: entry.capacity,
    interest: entry.interest.map((interest) => interest.trim()),
    bots: Object.entries(bots).map(([name, pubkey]) => ({
      name: name.trim(),
      pubkey: pubkey.toLowerCase(),
      role: "bot",
    })),
  };
}

export function issueReferenceFromChannel(channel) {
  const description = channel.about || channel.description || "";
  const url = description.match(
    /https:\/\/github\.com\/([^/\s]+)\/([^/\s]+)\/issues\/([1-9]\d*)/i,
  );
  if (url) {
    return {
      repository: `${url[1]}/${url[2]}`,
      number: Number.parseInt(url[3], 10),
      source: "description",
    };
  }

  const name = channel.name || "";
  const legacy = name.match(/^([^\s]+\/[^\s]+)\s+#([1-9]\d*)(?:\s|$)/);
  if (legacy) {
    return {
      repository: legacy[1],
      number: Number.parseInt(legacy[2], 10),
      source: "legacy-name",
    };
  }

  const canonical = name.match(/^#?([1-9]\d*)(?:\s|$)/);
  return canonical
    ? {
        repository: null,
        number: Number.parseInt(canonical[1], 10),
        source: "name",
      }
    : null;
}

export function channelMatchesIssue(channel, issue) {
  const reference = issueReferenceFromChannel(channel);
  if (!reference || reference.number !== issue.number) {
    return false;
  }
  return (
    !reference.repository ||
    reference.repository.toLowerCase() === issue.repository.toLowerCase()
  );
}

export function repositoryFromIssueUrl(url) {
  try {
    const parts = new URL(url).pathname.split("/").filter(Boolean);
    return parts.length >= 4 && parts[2] === "issues"
      ? `${parts[0]}/${parts[1]}`
      : null;
  } catch {
    return null;
  }
}
