# Patch: `goose recipe list` — column-aligned table + duplicate-location flagging

Target file: `crates/goose-cli/src/commands/recipe.rs` (aaif-goose/goose, main branch, fetched 2026-07-08)
No new dependencies — `comfy_table` and `console` are already in `goose-cli`'s `Cargo.toml`.

Only `handle_list` changes, plus two small new private helpers (`describe`, `source_path`) factored
out of logic that already existed inline. Everything else in the file (validate/deeplink/open/tests)
is untouched.

## Add this import near the top of the file (alongside the existing `use console::style;`)

```rust
use comfy_table::{presets::UTF8_FULL_CONDENSED, ContentArrangement, Table};
```

## Replace the existing `handle_list` function with:

```rust
pub fn handle_list(format: &str, verbose: bool) -> Result<()> {
    let mut recipes = match list_available_recipes() {
        Ok(recipes) => recipes,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to list recipes: {}", e));
        }
    };

    match format {
        "json" => {
            // Unchanged: flat array, so existing automation/scripts don't break.
            println!("{}", serde_json::to_string(&recipes)?);
        }
        _ => {
            if recipes.is_empty() {
                println!("No recipes found");
                return Ok(());
            }

            recipes.sort_by(|a, b| a.name.cmp(&b.name));

            // Group by name. A recipe found via more than one search path
            // (GOOSE_RECIPE_PATH + cwd, most commonly) currently prints as
            // flat, indistinguishable duplicate rows with no indication
            // that one file silently shadows the other. Group instead of
            // hiding the collision.
            let mut groups: Vec<(String, Vec<&RecipeInfo>)> = Vec::new();
            for recipe in &recipes {
                match groups.last_mut() {
                    Some((name, entries)) if *name == recipe.name => entries.push(recipe),
                    _ => groups.push((recipe.name.clone(), vec![recipe])),
                }
            }

            if verbose {
                for (name, entries) in &groups {
                    println!("{}", style(name).bold());
                    println!("  {}", describe(entries[0]));
                    if entries.len() > 1 {
                        println!(
                            "  {} found in {} locations (later paths override earlier ones):",
                            style("⚠").yellow(),
                            entries.len()
                        );
                        for (i, entry) in entries.iter().enumerate() {
                            println!("    {}. {}", i + 1, source_path(entry));
                        }
                    } else {
                        println!("  {}", source_path(entries[0]));
                    }
                    println!();
                }
                return Ok(());
            }

            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL_CONDENSED)
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(vec!["Name", "Description", "Location"]);

            for (name, entries) in &groups {
                let location = if entries.len() > 1 {
                    format!("⚠ {} locations", entries.len())
                } else {
                    source_path(entries[0])
                };
                table.add_row(vec![name.clone(), describe(entries[0]), location]);
            }

            println!("{table}");

            let dup_count = groups.iter().filter(|(_, entries)| entries.len() > 1).count();
            if dup_count > 0 {
                println!(
                    "\n{} {} recipe{} found in multiple locations. Run with --verbose for full paths.",
                    style("⚠").yellow(),
                    dup_count,
                    if dup_count == 1 { "" } else { "s" }
                );
            }
        }
    }
    Ok(())
}

fn describe(recipe: &RecipeInfo) -> String {
    match &recipe.description {
        Some(desc) if !desc.is_empty() => desc.clone(),
        _ => "(none)".to_string(),
    }
}

fn source_path(recipe: &RecipeInfo) -> String {
    match recipe.source {
        RecipeSource::Local => format!("local: {}", recipe.path),
        RecipeSource::GitHub => format!("github: {}", recipe.path),
    }
}
```

## Design choices worth knowing before you review

- **`--format json` is untouched.** Kept the flat array shape so any existing scripts/automation parsing
  it don't break — grouping is a `text`-output-only concern. Matches the CONTRIBUTING.md steer toward
  minimal, non-breaking first PRs.
- **Sorted alphabetically by name** rather than search-path order — more predictable for a listing
  command, and a prerequisite for the adjacent-grouping logic (no `HashMap`, so output order stays
  deterministic run to run).
- **No new dependency.** `comfy_table` was already pulled in by `goose-cli`'s `Cargo.toml`; this patch
  just uses it for `recipe list` too.
- **Not yet verified against the compiler.** High confidence in the `comfy-table` v7 API calls used
  here (`Table::new`, `.load_preset`, `.set_content_arrangement`, `.set_header`, `.add_row`) and the
  `RecipeInfo`/`RecipeSource` field shapes (confirmed by reading `search_recipe.rs` directly), but this
  has not been run through `cargo build` or the existing test suite yet. That's the first thing to do
  on this machine — see `../BUILD_AND_SUBMIT.md`.

## Reference: original source (unpatched) for context

Fetched from `https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/commands/recipe.rs`.
The original `handle_list` had no grouping at all — it iterated the flat `Vec<RecipeInfo>` from
`list_available_recipes()` (in `crates/goose-cli/src/recipes/search_recipe.rs`) and printed
`"{name} - {description} - {source_info}"` per entry, one line each, verbose mode just adding a
`Title:`/`Path:` sub-line. `RecipeInfo` fields: `name: String`, `source: RecipeSource` (`Local` |
`GitHub`), `path: String`, `title: Option<String>`, `description: Option<String>`. `list_available_recipes()`
simply `.extend()`s local recipes and (if configured) GitHub recipes into one `Vec` with no dedup step —
that's the root cause this patch addresses.
