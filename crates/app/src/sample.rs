use std::sync::Arc;

use sdw_core::Song;
use sdw_core::bible::{Book, Passage, Reference, Verse};
use sdw_core::compile;
use sdw_core::slide::SlideDeck;

const YOHANES_3: &[(u16, &str)] = &[
    (
        16,
        "Karena demikianlah Allah mengasihi isi dunia ini, sehingga dikaruniakan-Nya Anak-Nya yang tunggal, supaya barangsiapa yang percaya akan Dia jangan binasa, melainkan beroleh hidup yang kekal.",
    ),
    (
        17,
        "Karena Allah menyuruhkan Anak-Nya ke dalam dunia ini, bukannya akan menghukumkan isi dunia, melainkan supaya isi dunia itu diselamatkan oleh-Nya.",
    ),
];

const MAZMUR_23: &[(u16, &str)] = &[
    (
        1,
        "Bahwa Tuhan itulah Gembalaku, maka tiadalah aku akan kekurangan.",
    ),
    (
        2,
        "Didudukkan-Nya aku di dalam padang rumput yang hijau, dihantar-Nya aku ke tepi air yang tenang.",
    ),
    (
        3,
        "Disegarkan-Nya jiwaku, dihantar-Nya aku pada jalan yang benar oleh karena nama-Nya.",
    ),
];

// TODO: hapus setelah library SQLite (M2) dan importer Alkitab (M4) tersedia; ini hanya
// isi contoh supaya tata letak operator bisa dikerjakan dan dinilai lebih dulu.
pub fn catalogue() -> Vec<Arc<SlideDeck>> {
    let songs = [
        compile::song(
            &Song::new(
                "Kaulah Harapan",
                "[Verse 1]\nDi dalam nama-Mu ada kemenangan\nDi dalam nama-Mu ada pemulihan\nTiada yang mustahil bagi-Mu\nKaulah harapanku\n\n[Chorus]\nKaulah harapan di tengah badai\nKaulah kekuatan saat ku lemah\nKu berserah pada-Mu\n\n[Verse 2]\nKasih setia-Mu tak pernah berubah\nSetiap pagi baru adanya\nSampai selamanya",
                Some("© Contoh · CCLI 0000000"),
            ),
            2,
        ),
        compile::song(
            &Song::new(
                "Bapa Yang Kekal",
                "[Verse 1]\nBapa yang kekal, Kau setia\nDari dulu hingga s'lamanya\nTak pernah berubah kasih-Mu\n\n[Chorus]\nKau Allah yang hidup\nKau Raja mulia\nS'luruh bumi menyembah-Mu",
                Some("© Contoh · CCLI 0000000"),
            ),
            2,
        ),
    ];

    let passages = [
        passage(43, 3, YOHANES_3).map(|passage| compile::scripture(&passage, 220)),
        passage(19, 23, MAZMUR_23).map(|passage| compile::scripture(&passage, 180)),
    ];

    songs
        .into_iter()
        .chain(passages.into_iter().flatten())
        .map(Arc::new)
        .collect()
}

fn passage(book: u8, chapter: u16, verses: &[(u16, &str)]) -> Option<Passage> {
    let first = verses.first()?.0;
    let last = verses.last()?.0;
    let reference = Reference::new(Book::from_number(book)?, chapter, Some((first, last))).ok()?;
    let verses = verses
        .iter()
        .map(|&(number, text)| Verse {
            number,
            text: text.into(),
        })
        .collect();

    Passage::new(reference, "TL", verses)
}
