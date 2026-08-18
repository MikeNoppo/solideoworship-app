#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputState {
    #[default]
    Show,
    Black,
    Clear,
}

impl OutputState {
    pub fn toggled(self, target: Self) -> Self {
        if self == target { Self::Show } else { target }
    }

    pub fn shows_content(self) -> bool {
        matches!(self, Self::Show)
    }

    pub fn shows_background(self) -> bool {
        matches!(self, Self::Show | Self::Clear)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Show => "Tampil",
            Self::Black => "Hitam",
            Self::Clear => "Kosong",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OutputState;

    #[test]
    fn toggle_ke_target_yang_sama_kembali_ke_show() {
        assert_eq!(
            OutputState::Black.toggled(OutputState::Black),
            OutputState::Show
        );
        assert_eq!(
            OutputState::Clear.toggled(OutputState::Clear),
            OutputState::Show
        );
    }

    #[test]
    fn toggle_ke_target_berbeda_langsung_berpindah() {
        assert_eq!(
            OutputState::Black.toggled(OutputState::Clear),
            OutputState::Clear
        );
        assert_eq!(
            OutputState::Show.toggled(OutputState::Black),
            OutputState::Black
        );
    }

    #[test]
    fn clear_menampilkan_background_tanpa_konten() {
        assert!(OutputState::Clear.shows_background());
        assert!(!OutputState::Clear.shows_content());
    }

    #[test]
    fn black_tidak_menampilkan_apa_pun() {
        assert!(!OutputState::Black.shows_background());
        assert!(!OutputState::Black.shows_content());
    }

    #[test]
    fn show_menampilkan_background_dan_konten() {
        assert!(OutputState::Show.shows_background());
        assert!(OutputState::Show.shows_content());
    }
}
