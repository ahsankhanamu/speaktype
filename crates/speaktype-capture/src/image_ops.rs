use fast_image_resize::IntoImageView;
use image::ImageEncoder;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("resize error: {0}")]
    Resize(#[from] fast_image_resize::ResizeError),
    #[error("unsupported image pixel format")]
    Unsupported,
}

/// Downscale a PNG so its long edge is at most `max_edge` (never upscaling),
/// writing it to `dst` atomically (temp file + rename). Returns the written
/// dimensions.
pub fn downscale_png(src: &Path, dst: &Path, max_edge: u32) -> Result<(u32, u32), ImageError> {
    let (width, height) = image::image_dimensions(src)?;
    if width.max(height) <= max_edge {
        // Already within budget: copy the source bytes through the temp
        // file so the on-disk contract (write + rename) stays identical.
        let tmp = tmp_sibling(dst);
        std::fs::copy(src, &tmp)?;
        std::fs::rename(&tmp, dst)?;
        return Ok((width, height));
    }

    let (new_w, new_h) = if width >= height {
        (
            max_edge,
            (u64::from(height) * u64::from(max_edge) / u64::from(width)) as u32,
        )
    } else {
        (
            (u64::from(width) * u64::from(max_edge) / u64::from(height)) as u32,
            max_edge,
        )
    };

    let img = image::ImageReader::open(src)?.decode()?;
    let pixel_type = img.pixel_type().ok_or(ImageError::Unsupported)?;
    let mut dst_image = fast_image_resize::images::Image::new(new_w, new_h, pixel_type);
    fast_image_resize::Resizer::new().resize(
        &img,
        &mut dst_image,
        Some(&fast_image_resize::ResizeOptions::new().resize_alg(
            fast_image_resize::ResizeAlg::Convolution(fast_image_resize::FilterType::Lanczos3),
        )),
    )?;

    let tmp = tmp_sibling(dst);
    let writer = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
    image::codecs::png::PngEncoder::new(writer).write_image(
        dst_image.buffer(),
        new_w,
        new_h,
        img.color().into(),
    )?;
    std::fs::rename(&tmp, dst)?;
    Ok((new_w, new_h))
}

/// Temp sibling of `dst` (`01-mark.png` → `01-mark.png.tmp`), removed by the
/// rename on success.
fn tmp_sibling(dst: &Path) -> PathBuf {
    let mut name = dst.as_os_str().to_os_string();
    name.push(".tmp");
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tempdir() -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "speaktype_capture_test_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).expect("tempdir");
        dir
    }

    fn write_test_png(path: &std::path::Path, width: u32, height: u32) {
        let img = image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 7])
        });
        img.save_with_format(path, image::ImageFormat::Png)
            .expect("write png");
    }

    #[test]
    fn downscale_never_upscales() {
        let dir = tempdir();
        let src = dir.join("small.png");
        let dst = dir.join("out.png");
        write_test_png(&src, 800, 600);

        let dims = downscale_png(&src, &dst, 1568).expect("downscale");
        assert_eq!(dims, (800, 600));
        assert_eq!(
            image::image_dimensions(&dst).expect("dst readable"),
            (800, 600),
            "the output file must exist with source dimensions"
        );
    }

    #[test]
    fn downscale_long_edge_to_1568() {
        let dir = tempdir();
        let src = dir.join("big.png");
        let dst = dir.join("out.png");
        write_test_png(&src, 3024, 1964);

        let dims = downscale_png(&src, &dst, 1568).expect("downscale");
        assert_eq!(dims, (1568, 1018));
        assert_eq!(
            image::image_dimensions(&dst).expect("dst readable"),
            (1568, 1018)
        );
    }

    #[test]
    fn downscale_writes_via_temp_and_rename() {
        let dir = tempdir();
        let src = dir.join("src.png");
        let dst = dir.join("out.png");
        write_test_png(&src, 2000, 1000);

        downscale_png(&src, &dst, 1568).expect("downscale");
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "tmp"))
            .collect();
        assert!(leftovers.is_empty(), "no temp files may remain");
    }
}
