# SolideoWorship

Aplikasi presentasi ibadah (lirik lagu dan ayat Alkitab) yang ringan, cepat, dan open source — ditulis dengan Rust.

SolideoWorship lahir dari masalah sederhana: aplikasi seperti EasyWorship memakan banyak RAM dan terasa lambat di laptop spesifikasi rendah, padahal laptop seperti itulah yang paling sering dipakai operator multimedia di gereja. Tujuan proyek ini adalah alur kerja yang akrab bagi pengguna EasyWorship, dengan **performa dan efisiensi memori sebagai prioritas utama**.

> **Status: tahap awal pengembangan.** Semua fitur MVP (lagu dan Alkitab) sudah tersedia, tetapi belum diuji di ibadah sungguhan maupun di Windows dan macOS.

## Yang sudah berjalan

- **Dua jendela, satu state.** Jendela operator di laptop dan jendela output fullscreen di proyektor. Keduanya merender dari state yang sama, jadi tampilan proyektor tidak bisa berbeda dari yang dilihat operator.
- **Tata letak operator ala EasyWorship.** Susunan acara, Preview, dan Live berdampingan, dengan panel Library (tab Lagu/Alkitab dan pencarian) di bawahnya.
- **Library lagu di SQLite.** Pencarian full-text atas judul dan lirik langsung saat mengetik, dengan awalan kata dan toleran terhadap diakritik. Daftar hanya memuat judul; lirik lengkap baru dibaca saat lagu dipilih.
- **Editor lagu.** Tambah, ubah, dan hapus lagu dengan preview slide yang langsung berubah saat mengetik. Lagu yang diubah ikut diperbarui di susunan acara.
- **Alkitab.** Terjemahan diimpor dari file Zefania XML lalu dicari dengan referensi seperti `Yoh 3:16-18`, `1 Kor 13:4-7`, atau `Mzm 23`. Nama kitab memakai nama dan singkatan Indonesia, ayat langsung tampil saat referensi diketik, dan perikop dibagi ke beberapa slide tanpa memotong ayat di tengah.
- **Pengaturan tema.** Satu tema untuk lagu dan satu untuk Alkitab: warna latar, teks, dan footer, ukuran huruf, jarak baris, margin, perataan, serta batas baris atau karakter per slide. Perubahan langsung terlihat di preview dialog. Tema baru tampil di proyektor saat item ditayangkan berikutnya, jadi mengubah tema di tengah ibadah tidak mengejutkan jemaat.
- **File susunan acara `.sdw`.** Isi lagu dan ayat ikut tersimpan di dalam file, jadi susunan acara bisa disiapkan di rumah lalu dibuka di laptop gereja walau library-nya berbeda. Perubahan yang belum disimpan ditandai, dan aplikasi meminta konfirmasi sebelum membuangnya.
- **Pemilihan layar proyektor.** Mendeteksi semua monitor dan mendahulukan layar eksternal. Output bisa dipindah ke layar lain, dan bila layar output tidak lagi terdeteksi saat daftar layar dimuat ulang, jendela output ditutup.
- **Status output Tampil / Hitam / Kosong.** Kosong menyembunyikan teks tapi tetap menampilkan latar belakang.
- **Kompiler lagu.** Bait dipecah otomatis per jumlah baris maksimum. Potongan lanjutan diberi nomor, misalnya "Verse 1 (2)".
- **Kompiler ayat.** Ayat dikelompokkan per slide berdasarkan batas karakter tanpa pernah dipotong di tengah, dan footer referensinya (mis. "Yohanes 3:16-17 (TB)") mengikuti ayat yang sedang tampil.
- **WYSIWYG.** Semua ukuran tema dinyatakan dalam kanvas acuan 1920×1080, lalu diskalakan, jadi thumbnail, preview, dan proyektor punya tata letak yang identik.

## Menulis lirik

Judul bagian ditulis dalam kurung siku di barisnya sendiri. Baris kosong memulai slide baru, dan bagian yang lebih panjang dari batas baris per slide dipecah otomatis.

```
[Verse 1]
Di dalam nama-Mu ada kemenangan
Di dalam nama-Mu ada pemulihan

Tiada yang mustahil bagi-Mu
Kaulah harapanku

[Reff]
Kaulah harapan di tengah badai
Kaulah kekuatan saat ku lemah
```

Contoh di atas menghasilkan tiga slide: "Verse 1", "Verse 1 (2)", dan "Reff".

## Mengimpor Alkitab

Teks Alkitab tidak disertakan di aplikasi karena hak cipta penerbitnya (misalnya LAI untuk Terjemahan Baru). Gunakan file Zefania XML yang Anda punya haknya:

1. Buka tab **Alkitab** di panel Library, lalu klik **Kelola Alkitab**.
2. Klik **Pilih file…** dan pilih file `.xml` berformat Zefania (`XMLBIBLE` → `BIBLEBOOK` → `CHAPTER` → `VERS`), berenkoding UTF-8.
3. Periksa singkatannya (diisi otomatis dari nama file, misalnya `TB`), lalu klik **Impor**.

Catatan kaki dan referensi silang di dalam file diabaikan, jadi tidak ikut tampil di proyektor. Satu Alkitab lengkap (sekitar 31 ribu ayat) menambah kira-kira 4 MB ke database library.

Referensi lintas pasal (misalnya `Yoh 3:16-4:2`) belum didukung; tambahkan tiap pasal sebagai item terpisah.

## Pintasan keyboard

| Tombol | Aksi |
| --- | --- |
| `→` / `Spasi` / `Page Down` | Slide live berikutnya |
| `←` / `Page Up` | Slide live sebelumnya |
| `Enter` | Tayangkan slide preview ke live |
| `B` | Layar hitam (tekan lagi untuk kembali) |
| `C` | Kosongkan teks (tekan lagi untuk kembali) |
| `F5` | Buka jendela output |
| `Esc` | Tutup jendela output |
| `Enter` di kotak pencarian lagu | Tambahkan hasil teratas ke susunan acara |
| `Enter` di kotak referensi ayat | Tambahkan perikop ke susunan acara |
| `Ctrl+S` / `Cmd+S` | Simpan susunan acara, atau simpan isi dialog yang sedang terbuka |
| `Ctrl+O` / `Cmd+O` | Buka file susunan acara |

Selama dialog (editor lagu, pengaturan tema) terbuka, pintasan operator dinonaktifkan supaya mengetik tidak mengubah layar proyektor.
## Roadmap

Cakupan MVP sengaja dibatasi pada **lagu dan ayat Alkitab**.

- [x] Kerangka dua jendela dan pemilih layar proyektor
- [x] Model slide, tema, serta kompiler lagu dan ayat
- [x] Tampilan operator: susunan acara, preview, live, library
- [x] Library lagu tersimpan di SQLite dengan pencarian full-text (FTS5)
- [x] Editor lagu (tambah, ubah, hapus)
- [x] Importer dan pencarian Alkitab per pasal/ayat
- [x] Pengaturan tema (warna, ukuran, perataan, pembagian slide)
- [x] Simpan dan buka susunan acara

Setelah MVP: gambar dan video sebagai latar, pilihan font, serta impor dari library EasyWorship/OpenLP.

## Prinsip performa

Prinsip-prinsip ini ditetapkan sejak awal dan sengaja dijaga:

- Library tidak pernah dimuat utuh ke memori. Daftar lagu hanya berisi judul, lirik lengkap baru dibaca saat lagu dipilih, dan ayat diambil per rentang yang ditampilkan.
- Teks yang tidak berubah disimpan sebagai `Box<str>`/`Arc<str>`, dan deck dibagi lewat `Arc`. Menayangkan slide dari Preview ke Live tidak menyalin isi slide sama sekali.
- Fungsi `view` meminjam data, tidak mengkloningnya, sehingga redraw tidak mengalokasikan string baru.
- Rendering reaktif: layar hanya digambar ulang saat ada perubahan, tidak ada render per frame.
- Akses database berjalan di thread pekerja sendiri, jadi pencarian atau penyimpanan tidak pernah menahan respons tombol operator.

## Platform

| Platform | Status |
| --- | --- |
| Windows | Target utama, diuji tiap milestone |
| macOS | Target utama, diuji tiap milestone |
| Linux | Bisa di-build dan dijalankan, belum menjadi target rilis |

## Lokasi data

Library lagu, terjemahan Alkitab, dan pengaturan tema disimpan di satu file SQLite, `library.sqlite3`:

| Platform | Folder |
| --- | --- |
| Windows | `%LOCALAPPDATA%\SolideoWorship` |
| macOS | `~/Library/Application Support/SolideoWorship` |
| Linux | `$XDG_DATA_HOME/solideoworship` (biasanya `~/.local/share/solideoworship`) |

Variabel lingkungan `SDW_DATA_DIR` mengganti folder itu, berguna untuk mencoba aplikasi tanpa menyentuh library utama.

## Build dari source

Butuh [Rust](https://rustup.rs) versi 1.92 atau lebih baru, plus compiler C untuk SQLite yang dibundel: Visual Studio Build Tools di Windows, atau Xcode Command Line Tools (`xcode-select --install`) di macOS.

Di Linux, dialog buka/simpan memakai `xdg-desktop-portal`, atau `zenity` bila portal tidak tersedia.

```sh
git clone https://github.com/MikeNoppo/solideoworship-app.git
cd solideoworship-app
cargo run -p sdw-app
```

Untuk build rilis yang dioptimasi:

```sh
cargo build --release -p sdw-app
```

Hasilnya ada di `target/release/sdw-app` (atau `sdw-app.exe` di Windows).

## Struktur proyek

```
crates/
├── core/     sdw-core    — tipe domain dan logika murni (lagu, ayat, slide, tema, kompiler, navigasi live)
├── library/  sdw-library — penyimpanan SQLite (lagu, Alkitab, tema, migrasi), importer Zefania, dan format file .sdw
└── app/      sdw-app     — aplikasi GUI berbasis iced
```

`sdw-core` sengaja tidak bergantung pada iced maupun SQLite, jadi logika domain bisa diuji tanpa GUI dan perubahan di sana tidak memicu kompilasi ulang iced yang lambat. Lagu dan ayat sama-sama dikompilasi menjadi `SlideDeck`. Mesin live dan renderer hanya mengenal `Slide`, sehingga jenis konten baru cukup ditambahkan sebagai kompiler baru.

## Pengembangan

```sh
cargo check                  # cek tipe dengan cepat
cargo test --workspace       # semua test, termasuk library SQLite di memori
cargo clippy --all-targets   # lint
cargo fmt                    # format kode
```

`cargo clippy` memunculkan peringatan *future-incompatibility* untuk `block v0.1.6`. Peringatan itu berasal dari dependensi macOS milik `display-info`, bukan dari kode proyek ini.

Kontribusi sangat diterima. Commit memakai format [Conventional Commits](https://www.conventionalcommits.org), dan komentar kode serta teks antarmuka ditulis dalam Bahasa Indonesia.

## Lisensi

Belum ditentukan.
