//! Price cliffs from Anthropic's public pricing page.
//!
//! Rows like `| Claude Haiku 5.5 (for prompts over 100,000 tokens) | …`
//! declare a prompt size above which a request bills a higher tier — all
//! input tokens count, cache reads and writes included. Fetched alongside
//! the models page (same no-credential GET, same caps and override env);
//! a fetch failure or a name that matches no listed model leaves no cliff.
//! The host then compacts at the window bound as before — never at a guess.

use crate::catalog::{ModelInfo, Snapshot, display_name_in};

/// Anthropic's public pricing page, as markdown.
const DEFAULT_PRICING_URL: &str = "https://platform.claude.com/docs/en/about-claude/pricing.md";
/// Points the fetch at another copy of the page (tests, mirrors). Unset or
/// empty means [`DEFAULT_PRICING_URL`].
pub const PRICING_URL_ENV: &str = "CLAUDE_SUB_PRICING_URL";

/// The page to read: the `CLAUDE_SUB_PRICING_URL` override when set and
/// non-empty, else the public default.
pub fn pricing_url(override_url: Option<&str>) -> String {
    override_url
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .unwrap_or(DEFAULT_PRICING_URL)
        .to_string()
}

/// GET the pricing page: no credentials, same timeout and size cap as the
/// models fetch.
pub async fn fetch_pricing(url: &str) -> Result<String, String> {
    crate::models::fetch_page(url, "pricing page").await
}

/// Stamp each model's price cliff. Pricing names map through the models
/// page's own display names, so the page stays the only source of truth; a
/// name matching nothing is dropped. `prior` keeps last-known cliffs when
/// this run had no usable pricing page — cached data, not a guess.
pub fn annotate(models: &mut Vec<ModelInfo>, page: Option<&str>, prior: &Snapshot) {
    let cliffs = page.and_then(cliff_rows).unwrap_or_default();
    if cliffs.is_empty() {
        // No usable pricing page this run: last-known cliffs stand (same
        // last-good rule as the models disk cache — a failed fetch never
        // rewrites the record). A parsed page below is authoritative.
        for m in models.iter_mut() {
            if m.price_cliff.is_none() {
                m.price_cliff = prior.by_id(&m.id).and_then(|p| p.price_cliff);
            }
        }
        return;
    }
    let names = Snapshot::new(models.clone());
    for m in models.iter_mut() {
        if m.price_cliff.is_some() {
            continue;
        }
        let display = normalize(&display_name_in(&m.id, &names));
        m.price_cliff = cliffs
            .iter()
            .find(|(name, _)| normalize(name) == display)
            .map(|(_, c)| *c);
    }
}

/// `(display name, cliff)` per pricing row: first cells labelled
/// `… (for prompts over N tokens)`. `up to` rows and non-table lines skip.
/// A ragged row in a table that holds over-rows reads as a torn page, not
/// sparse data: `None` declines it so last-good cliffs stand. Shares the
/// models page's table splitter, so one definition of `table` serves both.
fn cliff_rows(page: &str) -> Option<Vec<(String, u32)>> {
    let mut out: Vec<(String, u32)> = Vec::new();
    for table in crate::models::markdown_tables(page) {
        // The label survives a cut anywhere inside the signature, and the
        // `up to` sibling qualifies the table when the over-row is gone.
        if !table
            .iter()
            .any(|row| row.iter().any(|cell| cell.contains("(for prompts")))
        {
            continue;
        }
        let width = table.first().map(|row| row.len())?;
        if table.iter().any(|row| row.len() < width) {
            return None;
        }
        for row in &table {
            if let Some((name, cliff)) = crate::models::row_label(row).and_then(over_row)
                && !out.iter().any(|(n, _)| *n == name)
            {
                out.push((name, cliff));
            }
        }
    }
    Some(out)
}

/// `Claude Haiku 5.5 (for prompts over 100,000 tokens)` →
/// `("Claude Haiku 5.5", 100_000)`.
fn over_row(cell: &str) -> Option<(String, u32)> {
    let (name, rest) = cell.split_once("(for prompts over ")?;
    let tokens = rest.strip_suffix("tokens)")?.trim();
    let n: u32 = tokens.replace(',', "").parse().ok()?;
    (!name.trim().is_empty() && n > 0).then(|| (name.trim().to_string(), n))
}

/// Whitespace- and case-folded name, for comparing the two pages' spellings.
fn normalize(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[path = "pricing_tests.rs"]
#[cfg(test)]
mod tests;
