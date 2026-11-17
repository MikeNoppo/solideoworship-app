use std::ffi::OsString;
use std::fs::{self, File};
use std::hash::Hash;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::codecs::webp::WebPEncoder;
use image::{ImageEncoder, ImageReader, Limits};
use lru::LruCache;
use sdw_core::MediaHash;

use crate::MediaError;

const MAX_DECODE_ALLOC_BYTES: u64 = 256 * 1024 * 1024;
const MAX_IMAGE_EDGE_PX: u32 = 16_384;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedRgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Arc<[u8]>,
}

pub trait CachedBytes {
    fn byte_len(&self) -> usize;
}

impl CachedBytes for DecodedRgba {
    fn byte_len(&self) -> usize {
        self.pixels.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageTier {
    Thumb,
    Stage,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageCacheKey {
    pub sha256: MediaHash,
    pub tier: ImageTier,
}

pub type ImageCache = ByteLruCache<ImageCacheKey, DecodedRgba>;

pub struct ByteLruCache<K, V> {
    entries: LruCache<K, V>,
    used_bytes: usize,
    max_bytes: usize,
}

impl<K: Hash + Eq, V: CachedBytes + Clone> ByteLruCache<K, V> {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            entries: LruCache::unbounded(),
            used_bytes: 0,
            max_bytes,
        }
    }

    pub fn used_bytes(&self) -> usize {
        self.used_bytes
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        self.entries.get(key).cloned()
    }

    pub fn peek(&self, key: &K) -> Option<&V> {
        self.entries.peek(key)
    }

    pub fn insert(&mut self, key: K, value: V) -> bool {
        let size = value.byte_len();
        if size > self.max_bytes {
            return false;
        }

        if let Some(previous) = self.entries.pop(&key) {
            self.used_bytes = self.used_bytes.saturating_sub(previous.byte_len());
        }

        while self.used_bytes.saturating_add(size) > self.max_bytes {
            match self.entries.pop_lru() {
                Some((_, evicted)) => {
                    self.used_bytes = self.used_bytes.saturating_sub(evicted.byte_len());
                }
                None => break,
            }
        }

        self.used_bytes = self.used_bytes.saturating_add(size);
        self.entries.put(key, value);
        true
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let evicted = self.entries.pop(key)?;
        self.used_bytes = self.used_bytes.saturating_sub(evicted.byte_len());
        Some(evicted)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.used_bytes = 0;
    }
}

pub fn decode_image_capped(
    path: &Path,
    max_width: u32,
    max_height: u32,
) -> Result<DecodedRgba, MediaError> {
    if max_width == 0 || max_height == 0 {
        return Err(MediaError::EmptyDimensions);
    }

    let rgba = decode_rgba_source(path)?;
    let src_w = rgba.width();
    let src_h = rgba.height();
    let (dst_w, dst_h) = fit_within(src_w, src_h, max_width, max_height);

    if dst_w == src_w && dst_h == src_h {
        return Ok(DecodedRgba {
            width: src_w,
            height: src_h,
            pixels: Arc::from(rgba.into_raw()),
        });
    }

    let resized = resize_rgba(src_w, src_h, rgba.as_raw(), dst_w, dst_h)?;
    Ok(DecodedRgba {
        width: dst_w,
        height: dst_h,
        pixels: Arc::from(resized),
    })
}

pub fn generate_image_thumbnail(
    src: &Path,
    dest_webp: &Path,
    max_width: u32,
) -> Result<(u32, u32), MediaError> {
    if max_width == 0 {
        return Err(MediaError::EmptyDimensions);
    }

    let rgba = decode_rgba_source(src)?;
    let src_w = rgba.width();
    let src_h = rgba.height();
    let (dst_w, dst_h) = fit_within(src_w, src_h, max_width, max_width);
    let thumb_pixels = if dst_w == src_w && dst_h == src_h {
        rgba.into_raw()
    } else {
        resize_rgba(src_w, src_h, rgba.as_raw(), dst_w, dst_h)?
    };

    if let Some(parent) = dest_webp.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp = temp_thumb_path(dest_webp);
    let write_result = (|| -> Result<(), MediaError> {
        let file = File::create(&tmp)?;
        let writer = BufWriter::new(file);
        let encoder = WebPEncoder::new_lossless(writer);
        encoder
            .write_image(&thumb_pixels, dst_w, dst_h, image::ExtendedColorType::Rgba8)
            .map_err(|err| MediaError::ImageDecode(err.to_string()))?;
        Ok(())
    })();

    if let Err(err) = write_result {
        let _ = fs::remove_file(&tmp);
        return Err(err);
    }

    if let Err(err) = fs::rename(&tmp, dest_webp) {
        let _ = fs::remove_file(&tmp);
        if !dest_webp.is_file() {
            return Err(err.into());
        }
    }

    Ok((src_w, src_h))
}

static NEXT_THUMB_TEMP: AtomicU64 = AtomicU64::new(0);

pub(crate) fn temp_thumb_path(dest: &Path) -> PathBuf {
    let seq = NEXT_THUMB_TEMP.fetch_add(1, Ordering::Relaxed);
    let mut name = OsString::from(dest.as_os_str());
    name.push(format!(".{}.{seq}.tmp", process::id()));
    PathBuf::from(name)
}

fn decode_rgba_source(path: &Path) -> Result<image::RgbaImage, MediaError> {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_EDGE_PX);
    limits.max_image_height = Some(MAX_IMAGE_EDGE_PX);
    limits.max_alloc = Some(MAX_DECODE_ALLOC_BYTES);

    let mut reader = ImageReader::open(path)?;
    reader.limits(limits);
    let dynamic = reader
        .with_guessed_format()?
        .decode()
        .map_err(|err| MediaError::ImageDecode(err.to_string()))?;

    let rgba = dynamic.into_rgba8();
    if rgba.width() == 0 || rgba.height() == 0 {
        return Err(MediaError::EmptyDimensions);
    }
    Ok(rgba)
}

fn resize_rgba(
    src_w: u32,
    src_h: u32,
    src_bytes: &[u8],
    dst_w: u32,
    dst_h: u32,
) -> Result<Vec<u8>, MediaError> {
    let src = ImageRef::new(src_w, src_h, src_bytes, PixelType::U8x4)
        .map_err(|err| MediaError::ImageDecode(err.to_string()))?;
    let mut dst = Image::new(dst_w, dst_h, PixelType::U8x4);
    let mut resizer = Resizer::new();
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Bilinear));
    resizer
        .resize(&src, &mut dst, &options)
        .map_err(|err| MediaError::ImageDecode(err.to_string()))?;
    Ok(dst.into_vec())
}

pub fn fit_within(src_w: u32, src_h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    if src_w == 0 || src_h == 0 || max_w == 0 || max_h == 0 {
        return (1, 1);
    }
    if src_w <= max_w && src_h <= max_h {
        return (src_w, src_h);
    }

    let scale_w = f64::from(max_w) / f64::from(src_w);
    let scale_h = f64::from(max_h) / f64::from(src_h);
    let scale = scale_w.min(scale_h);

    let dst_w = ((f64::from(src_w) * scale).round() as u32).clamp(1, max_w);
    let dst_h = ((f64::from(src_h) * scale).round() as u32).clamp(1, max_h);
    (dst_w, dst_h)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::{env, fs, process};

    use image::{ImageBuffer, Rgba};
    use sdw_core::MediaHash;

    use super::{
        CachedBytes, DecodedRgba, ImageCache, ImageCacheKey, ImageTier, decode_image_capped,
        fit_within, generate_image_thumbnail,
    };

    const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const SHA_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    #[test]
    fn fit_within_preserves_aspect_ratio_without_upscaling() {
        assert_eq!(fit_within(1280, 720, 1920, 1080), (1280, 720));
        assert_eq!(fit_within(3840, 2160, 1920, 1080), (1920, 1080));
        assert_eq!(fit_within(4000, 3000, 320, 320), (320, 240));
        assert_eq!(fit_within(100, 4000, 320, 320), (8, 320));
    }

    #[test]
    fn image_cache_evicts_lru_to_respect_byte_budget() {
        let mut cache = ImageCache::new(100);
        let entry = |bytes: usize| DecodedRgba {
            width: 1,
            height: 1,
            pixels: Arc::from(vec![0u8; bytes]),
        };

        let key_a = ImageCacheKey {
            sha256: MediaHash::parse(SHA_A).unwrap(),
            tier: ImageTier::Stage,
        };
        let key_b = ImageCacheKey {
            sha256: MediaHash::parse(SHA_B).unwrap(),
            tier: ImageTier::Stage,
        };
        let key_c = ImageCacheKey {
            sha256: MediaHash::parse(SHA_C).unwrap(),
            tier: ImageTier::Stage,
        };

        assert!(cache.insert(key_a.clone(), entry(60)));
        assert!(cache.insert(key_b.clone(), entry(40)));
        assert_eq!(cache.used_bytes(), 100);

        assert!(cache.get(&key_a).is_some());
        assert!(cache.insert(key_c.clone(), entry(35)));
        assert!(cache.get(&key_b).is_none());
        assert!(cache.get(&key_a).is_some());
        assert!(cache.get(&key_c).is_some());
        assert_eq!(cache.used_bytes(), 95);
    }

    #[test]
    fn image_cache_skips_oversized_entry_without_clearing_valid_items() {
        let mut cache = ImageCache::new(64);
        let key_a = ImageCacheKey {
            sha256: MediaHash::parse(SHA_A).unwrap(),
            tier: ImageTier::Thumb,
        };
        let key_b = ImageCacheKey {
            sha256: MediaHash::parse(SHA_B).unwrap(),
            tier: ImageTier::Stage,
        };

        assert!(cache.insert(
            key_a.clone(),
            DecodedRgba {
                width: 1,
                height: 1,
                pixels: Arc::from(vec![1u8; 32]),
            }
        ));
        assert!(!cache.insert(
            key_b.clone(),
            DecodedRgba {
                width: 1,
                height: 1,
                pixels: Arc::from(vec![2u8; 128]),
            }
        ));
        assert!(cache.get(&key_a).is_some());
        assert_eq!(cache.used_bytes(), 32);
    }

    #[test]
    fn decodes_downscales_and_writes_webp_thumbnail() {
        let dir = env::temp_dir().join(format!("sdw-media-img-{}", process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let src = dir.join("big.png");
        let thumb = dir.join("thumbs").join("thumb.webp");
        let img = ImageBuffer::from_pixel(640, 360, Rgba([12u8, 34, 56, 255]));
        img.save(&src).unwrap();

        let (orig_w, orig_h) = generate_image_thumbnail(&src, &thumb, 320).unwrap();
        assert_eq!((orig_w, orig_h), (640, 360));
        assert!(thumb.is_file());

        let decoded_thumb = decode_image_capped(&thumb, 1000, 1000).unwrap();
        assert_eq!((decoded_thumb.width, decoded_thumb.height), (320, 180));

        let decoded_stage = decode_image_capped(&src, 400, 400).unwrap();
        assert_eq!((decoded_stage.width, decoded_stage.height), (400, 225));
        assert_eq!(decoded_stage.byte_len(), 400 * 225 * 4);

        let _ = fs::remove_dir_all(&dir);
    }
}
