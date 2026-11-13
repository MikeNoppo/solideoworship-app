use sdw_core::{Song, compile};

#[test]
fn heading_blocks_create_sections_without_continuations_or_empty_blocks() {
    let deck = compile::song(
        &Song::new(
            "Test",
            "unlabeled\n[Empty]\n[Verse 1]\none\ntwo\nthree\n\nfour\n[Chorus]\nfive\n[Verse 1]\nsix\n[Empty]",
            None,
        ),
        2,
    );
    let sections: Vec<_> = deck
        .sections
        .iter()
        .map(|section| (section.name.as_ref(), section.start))
        .collect();
    assert_eq!(sections, [("Verse 1", 1), ("Chorus", 4), ("Verse 1", 5)]);
    assert_eq!(deck.section_at(0), None);
    assert_eq!(deck.section_at(3), Some(&deck.sections[0]));
    assert_eq!(deck.section_at(5), Some(&deck.sections[2]));
    assert_eq!(deck.section_at(6), None);
}
