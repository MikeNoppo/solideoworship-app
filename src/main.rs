use iced::widget::{button, column, text, Column};

// 1. STATE: Struktur data untuk menyimpan kondisi aplikasi Anda
#[derive(Default)]
struct Counter {
    value: i32,
}

// 2. MESSAGES: Event atau interaksi yang bisa dilakukan oleh pengguna
#[derive(Debug, Clone, Copy)]
pub enum Message {
    Increment,
    Decrement,
}

impl Counter {
    // 3. UPDATE: Logika untuk merespons Messages dan mengubah State
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Increment => {
                self.value += 1;
            }
            Message::Decrement => {
                self.value -= 1;
            }
        }
    }

    // 4. VIEW: Menampilkan State ke dalam bentuk UI/Widget
    pub fn view(&self) -> Column<'_, Message> {
        column![
            // Tombol tambah yang mengirimkan pesan Increment
            button("+").on_press(Message::Increment),
            
            // Menampilkan nilai counter
            text(self.value).size(50),
            
            // Tombol kurang yang mengirimkan pesan Decrement
            button("-").on_press(Message::Decrement),
        ]
    }
}

// Fungsi utama untuk menjalankan aplikasi Iced
fn main() -> iced::Result {
    iced::run(Counter::update, Counter::view)
}