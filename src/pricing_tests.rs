use super::*;
use crate::models::{catalog_from, parse_overview};

const PRICING: &str = include_str!("../tests/fixtures/pricing.md");
const OVERVIEW: &str = include_str!("../tests/fixtures/models-overview.md");

#[test]
fn parses_over_rows() {
    assert_eq!(
        cliff_rows(PRICING),
        Some(vec![("Claude Haiku 5.5".to_string(), 100_000)]),
        "the saved pricing page declares exactly one cliff"
    );
}

#[test]
fn torn_cliff_table_declines_the_page() {
    // Cut mid-cell inside the haiku over-row: the ragged last row means the
    // page is torn, so it yields no cliffs and last-good data stands instead
    // of silently clearing them.
    let cut = PRICING.find("(for prompts over").unwrap() + 12;
    let torn = &PRICING[..cut];
    assert_eq!(cliff_rows(torn), None);
    let mut seeded = parse_overview(OVERVIEW).unwrap();
    annotate(&mut seeded, Some(PRICING), &Snapshot::new(Vec::new()));
    let prior = Snapshot::new(seeded);
    let mut models = parse_overview(OVERVIEW).unwrap();
    annotate(&mut models, Some(torn), &prior);
    let haiku = models.iter().find(|m| m.id == "claude-haiku-5-5").unwrap();
    assert_eq!(
        haiku.price_cliff,
        Some(100_000),
        "a torn pricing page must not erase last-good cliffs"
    );
}

#[test]
fn over_row_shapes() {
    assert_eq!(
        over_row("Claude Haiku 5.5 (for prompts over 100,000 tokens)"),
        Some(("Claude Haiku 5.5".to_string(), 100_000))
    );
    // `up to` and unlabelled cells never yield a cliff.
    assert_eq!(
        over_row("Claude Haiku 5.5 (for prompts up to 100,000 tokens)"),
        None
    );
    assert_eq!(over_row("Model page"), None);
    assert_eq!(over_row(""), None);
}

#[test]
fn annotate_maps_names_to_ids() {
    let mut models = parse_overview(OVERVIEW).unwrap();
    annotate(&mut models, Some(PRICING), &Snapshot::new(Vec::new()));
    for m in &models {
        let want = (m.id == "claude-haiku-5-5").then_some(100_000);
        assert_eq!(
            m.price_cliff, want,
            "{} carries {:?}, expected {:?}",
            m.id, m.price_cliff, want
        );
    }
}

#[test]
fn catalog_row_carries_cliff() {
    let mut models = parse_overview(OVERVIEW).unwrap();
    annotate(&mut models, Some(PRICING), &Snapshot::new(Vec::new()));
    let s = Snapshot::new(models);
    let cat = catalog_from(&s);
    let haiku = cat
        .models
        .iter()
        .find(|m| m.id == "claude-haiku-5-5")
        .expect("haiku row");
    assert_eq!(haiku.price_cliff_tokens, Some(100_000));
    // The [1m] beta row resolves through its canonical id.
    let haiku_1m = cat
        .models
        .iter()
        .find(|m| m.id == "claude-haiku-5-5[1m]")
        .expect("haiku [1m] row");
    assert_eq!(haiku_1m.price_cliff_tokens, Some(100_000));
    // Family and [1m] routes resolve to the same billed model: exactly the
    // haiku lineage carries the cliff — never a guessed one elsewhere.
    let mut with_cliff: Vec<&str> = cat
        .models
        .iter()
        .filter(|m| m.price_cliff_tokens.is_some())
        .map(|m| m.id.as_str())
        .collect();
    with_cliff.sort();
    assert_eq!(
        with_cliff,
        vec!["claude-haiku-5-5", "claude-haiku-5-5[1m]", "haiku"]
    );
}

#[test]
fn failed_fetch_keeps_prior_cliffs() {
    let mut seeded = parse_overview(OVERVIEW).unwrap();
    annotate(&mut seeded, Some(PRICING), &Snapshot::new(Vec::new()));
    let prior = Snapshot::new(seeded);
    let mut models = parse_overview(OVERVIEW).unwrap();
    annotate(&mut models, None, &prior);
    let haiku = models.iter().find(|m| m.id == "claude-haiku-5-5").unwrap();
    assert_eq!(haiku.price_cliff, Some(100_000), "last-good cliffs persist");
}

#[test]
fn fresh_page_clears_a_dropped_cliff() {
    // A parsed page is authoritative: a model whose over-row vanished loses
    // the stale cliff rather than guessing it still applies. (A page with no
    // over-rows at all reads as unusable, and last-good stands instead.)
    let mut seeded = parse_overview(OVERVIEW).unwrap();
    annotate(&mut seeded, Some(PRICING), &Snapshot::new(Vec::new()));
    let prior = Snapshot::new(seeded);
    let mut models = parse_overview(OVERVIEW).unwrap();
    annotate(
        &mut models,
        Some("| Claude Imaginary 9.9 (for prompts over 7,000 tokens) |  / MTok |"),
        &prior,
    );
    assert!(models.iter().all(|m| m.price_cliff.is_none()));
}

#[test]
fn unmatched_name_drops() {
    let mut models = parse_overview(OVERVIEW).unwrap();
    annotate(
        &mut models,
        Some("| Imaginary 9.9 (for prompts over 7,000 tokens) | $1 / MTok |"),
        &Snapshot::new(Vec::new()),
    );
    assert!(models.iter().all(|m| m.price_cliff.is_none()));
}
