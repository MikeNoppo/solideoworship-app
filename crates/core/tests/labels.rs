use std::sync::Arc;

use sdw_core::labels::LabelSettings;
use sdw_core::{Presentation, Song, compile};

#[test]
fn aliases_and_numbered_sections_cycle_distinct_occurrences() {
    let labels = LabelSettings::default();
    let rule = labels.rule_for_shortcut('V').unwrap();
    assert!(rule.matches("bAiT__12"));
    assert!(
        labels
            .rule_for("PRE _ CHORUS 2")
            .unwrap()
            .matches("Pra-Reff")
    );
    assert!(labels.rule_for("unknown").is_none());
    let deck = compile::song(
        &Song::new(
            "Test",
            "[Verse 1]\none\ntwo\nthree\n[Reff]\nfour\n[Bait 2]\nfive\n[Bridge]\nsix\n[Verse 1]\nseven",
            None,
        ),
        2,
    );
    let mut live = Presentation::new(Arc::new(deck)).unwrap();
    assert_eq!(live.next_section(rule), Some(3));
    assert!(live.cycle_section(rule));
    assert_eq!(live.index(), 3);
    assert!(live.cycle_section(rule));
    assert_eq!(live.index(), 5);
    assert!(live.cycle_section(rule));
    assert_eq!(live.index(), 0);
    live.go_to(1);
    assert_eq!(live.next_section(rule), Some(3));
    live.go_to(2);
    assert_eq!(live.next_section(rule), Some(3));
    live.go_to(4);
    let chorus = labels.rule_for("Chorus").unwrap();
    assert!(live.cycle_section(chorus));
    assert_eq!(live.index(), 2);
    assert!(!live.cycle_section(chorus));
}

#[test]
fn settings_reject_conflicts_bounds_and_unusable_rules() {
    use sdw_core::settings::SettingsError;
    let defaults = LabelSettings::default();
    assert_eq!(defaults.validate(), Ok(()));
    assert_eq!(LabelSettings { rules: vec![] }.validate(), Ok(()));
    let mut labels = defaults.clone();
    labels.rules[1].aliases.push("bait-2".into());
    assert_eq!(labels.validate(), Err(SettingsError::ConflictingLabels));
    labels = defaults.clone();
    labels.rules[1].shortcut = Some('V');
    assert_eq!(labels.validate(), Err(SettingsError::ConflictingLabels));
    labels = defaults.clone();
    labels.rules[0].name = "-- 12".into();
    assert_eq!(labels.validate(), Err(SettingsError::InvalidLabels));
    labels = defaults.clone();
    labels.rules[0].shortcut = Some('v');
    assert_eq!(labels.validate(), Err(SettingsError::InvalidLabels));
    labels = defaults.clone();
    labels.rules[0].color.r = f32::NAN;
    assert_eq!(labels.validate(), Err(SettingsError::InvalidLabels));
    labels = defaults.clone();
    labels.rules[0].aliases = vec!["Alias".into(); 9];
    assert_eq!(labels.validate(), Err(SettingsError::InvalidLabels));
    labels = defaults.clone();
    labels.rules[0].name = "x".repeat(65).into();
    assert_eq!(labels.validate(), Err(SettingsError::InvalidLabels));
    labels.rules = vec![defaults.rules[0].clone(); 33];
    assert_eq!(labels.validate(), Err(SettingsError::InvalidLabels));
    assert_eq!(labels.clamped(), defaults);
}
