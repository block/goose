import React, { useMemo, useState } from "react";
import CodeBlock from "@theme/CodeBlock";
import apiData from "@site/src/data/gdk-api.json";
import { LANGUAGES, Language, LanguageId } from "./languages";
import styles from "./styles.module.css";

type GdkParam = {
  name: string;
  type: string;
  default: string | null;
  docs: string;
};

type GdkFunc = {
  name: string;
  docs: string;
  params: GdkParam[];
  returns: string | null;
  throws: string | null;
  isAsync: boolean;
};

type GdkItem = {
  name: string;
  kind: "object" | "callback" | "record" | "enum" | "error";
  docs: string;
  fields: GdkParam[];
  variants: { name: string; fields: GdkParam[] }[];
  methods: GdkFunc[];
};

type GdkApiDoc = {
  version: string;
  docVersion: string;
  source: string;
  functions: GdkFunc[];
  items: GdkItem[];
};

const VERSIONS = (apiData as { versions: GdkApiDoc[] }).versions;

const KIND_LABELS: Record<GdkItem["kind"], string> = {
  object: "Class",
  callback: "Interface",
  record: "Data type",
  enum: "Enum",
  error: "Error",
};

const KIND_HEADINGS: Record<GdkItem["kind"], string> = {
  object: "Classes",
  callback: "Interfaces",
  record: "Data types",
  enum: "Enums",
  error: "Errors",
};

const slug = (...parts: string[]) =>
  parts.join("-").replace(/[^a-zA-Z0-9]+/g, "-").toLowerCase();

function signature(func: GdkFunc, language: Language, owner?: string): string {
  const params = func.params
    .map((param) => {
      const type = language.type(param.type);
      const suffix = param.default ? ` = ${language.default(param.default)}` : "";
      switch (language.id) {
        case "rust":
          return `${param.name}: ${type}${suffix}`;
        case "python":
          return `${language.field(param.name)}: ${type}${suffix}`;
        default:
          return `${language.field(param.name)}: ${type}${suffix}`;
      }
    })
    .join(", ");

  const name = language.func(func.name);
  const returns = func.returns ? language.type(func.returns) : null;
  const prefix = owner ? `${owner}.` : "";

  if (language.id === "rust") {
    const asyncKeyword = func.isAsync ? "async " : "";
    const result = func.throws
      ? `Result<${returns ?? "()"}, ${func.throws}>`
      : returns;
    return `${asyncKeyword}fn ${prefix}${name}(${params})${result ? ` -> ${result}` : ""}`;
  }

  if (language.id === "python") {
    const asyncKeyword = func.isAsync ? "async " : "";
    return `${asyncKeyword}def ${prefix}${name}(${params})${returns ? ` -> ${returns}` : ""}`;
  }

  const suspend = func.isAsync ? "suspend " : "";
  const throwsAnnotation = func.throws
    ? `@Throws(${language.errorType(func.throws)}::class)\n`
    : "";
  return `${throwsAnnotation}${suspend}fun ${prefix}${name}(${params})${returns ? `: ${returns}` : ""}`;
}

function ParamTable({
  rows,
  language,
  caption,
}: {
  rows: GdkParam[];
  language: Language;
  caption: string;
}) {
  if (rows.length === 0) return null;
  const hasDefaults = rows.some((row) => row.default);
  return (
    <table className={styles.table}>
      <thead>
        <tr>
          <th>{caption}</th>
          <th>Type</th>
          {hasDefaults && <th>Default</th>}
          <th>Description</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((row) => (
          <tr key={row.name}>
            <td>
              <code>{language.field(row.name)}</code>
            </td>
            <td>
              <code>{language.type(row.type)}</code>
            </td>
            {hasDefaults && (
              <td>{row.default ? <code>{language.default(row.default)}</code> : "—"}</td>
            )}
            <td>{row.docs || "—"}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function FuncEntry({
  func,
  language,
  owner,
}: {
  func: GdkFunc;
  language: Language;
  owner?: string;
}) {
  return (
    <div className={styles.entry} id={slug(owner ?? "fn", func.name)}>
      <h4 className={styles.entryTitle}>
        <code>{language.func(func.name)}</code>
      </h4>
      {func.docs && <p>{func.docs}</p>}
      <CodeBlock language={language.prism}>{signature(func, language, owner)}</CodeBlock>
      <ParamTable rows={func.params} language={language} caption="Parameter" />
      {func.throws && (
        <p className={styles.meta}>
          Raises <code>{language.errorType(func.throws)}</code>
        </p>
      )}
    </div>
  );
}

function ItemEntry({ item, language }: { item: GdkItem; language: Language }) {
  const dataCarrying = item.variants.some((variant) => variant.fields.length > 0);
  return (
    <section className={styles.item} id={slug(item.name)}>
      <h3 className={styles.itemTitle}>
        <code>{item.kind === "error" ? language.errorType(item.name) : item.name}</code>
        <span className={styles.badge}>{KIND_LABELS[item.kind]}</span>
      </h3>
      {item.docs && <p>{item.docs}</p>}

      <ParamTable rows={item.fields} language={language} caption="Field" />

      {item.variants.length > 0 && (
        <table className={styles.table}>
          <thead>
            <tr>
              <th>{item.kind === "error" ? "Variant" : "Case"}</th>
              <th>Associated data</th>
            </tr>
          </thead>
          <tbody>
            {item.variants.map((variant) => (
              <tr key={variant.name}>
                <td>
                  <code>
                    {item.kind === "error" && language.id === "kotlin"
                      ? `${language.errorType(item.name)}.${variant.name}`
                      : language.variant(variant.name, dataCarrying)}
                  </code>
                </td>
                <td>
                  {variant.fields.length === 0
                    ? "—"
                    : variant.fields.map((field) => (
                        <div key={field.name}>
                          <code>
                            {language.field(field.name)}: {language.type(field.type)}
                          </code>
                        </div>
                      ))}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {item.methods.map((method) => (
        <FuncEntry key={method.name} func={method} language={language} owner={item.name} />
      ))}
    </section>
  );
}

export default function GdkApiReference() {
  const [languageId, setLanguageId] = useState<LanguageId>("rust");
  const [docVersion, setDocVersion] = useState(VERSIONS[0].docVersion);

  const language = LANGUAGES.find((entry) => entry.id === languageId)!;
  const doc = useMemo(
    () => VERSIONS.find((entry) => entry.docVersion === docVersion) ?? VERSIONS[0],
    [docVersion],
  );

  const grouped = useMemo(() => {
    const order: GdkItem["kind"][] = ["object", "callback", "record", "enum", "error"];
    return order
      .map((kind) => ({ kind, items: doc.items.filter((item) => item.kind === kind) }))
      .filter((group) => group.items.length > 0);
  }, [doc]);

  return (
    <div>
      <div className={styles.toolbar}>
        <div className={styles.tabs} role="tablist" aria-label="GDK language">
          {LANGUAGES.map((entry) => (
            <button
              key={entry.id}
              type="button"
              role="tab"
              aria-selected={entry.id === languageId}
              className={entry.id === languageId ? styles.tabActive : styles.tab}
              onClick={() => setLanguageId(entry.id)}
            >
              {entry.label}
            </button>
          ))}
        </div>

        <label className={styles.version}>
          Version
          <select
            value={docVersion}
            onChange={(event) => setDocVersion(event.target.value)}
            aria-label="GDK version"
          >
            {VERSIONS.map((entry) => (
              <option key={entry.docVersion} value={entry.docVersion}>
                {entry.docVersion}.x
              </option>
            ))}
          </select>
        </label>
      </div>

      <p className={styles.meta}>
        Generated from <code>{doc.source}</code> at <code>goose-sdk {doc.version}</code>.
      </p>

      <h2 id="functions">Functions</h2>
      {doc.functions.map((func) => (
        <FuncEntry key={func.name} func={func} language={language} />
      ))}

      {grouped.map((group) => (
        <React.Fragment key={group.kind}>
          <h2 id={slug(group.kind, "types")}>{KIND_HEADINGS[group.kind]}</h2>
          {group.items.map((item) => (
            <ItemEntry key={item.name} item={item} language={language} />
          ))}
        </React.Fragment>
      ))}
    </div>
  );
}
