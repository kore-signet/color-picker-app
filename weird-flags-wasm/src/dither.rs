use image::{imageops::{self, ColorMap}, RgbImage};

use crate::{pattern::{self, MatrixSize}, ColorMapper, DitherMethod, ProgressReporter};

pub fn color_pick(image: &mut RgbImage, mapper: ColorMapper, dither_mode: DitherMethod, progress: ProgressReporter<'_>) {
    match dither_mode {
        DitherMethod::None => simple_color_pick(image, mapper, progress),
        DitherMethod::FloydSteinberg => floyd_steinberg_pick(image, mapper, progress),
        DitherMethod::Pattern2x2 => pattern::dither(image, MatrixSize::Two, 0.09, mapper, progress),
        DitherMethod::Pattern4x4 => pattern::dither(image, MatrixSize::Four, 0.09, mapper, progress),
        DitherMethod::Pattern8x8 => pattern::dither(image, MatrixSize::Eight, 0.09, mapper, progress),
    }
}

// no dithering
pub fn simple_color_pick(image: &mut RgbImage, mapper: ColorMapper, progress: ProgressReporter<'_>) {
    let total_pixels = (image.width() * image.height()) as usize;

    for (index, pixel) in image.pixels_mut().enumerate() {
        if index % 1000 == 0 {
            let pct = (index as f64 / total_pixels as f64) * 100.0f64;
            progress.log_pct("dither", pct).unwrap();
        }

        mapper.map_color(pixel);
    }
}

pub fn floyd_steinberg_pick(image: &mut RgbImage, mapper: ColorMapper, progress: ProgressReporter<'_>) {
    progress.log_str("dither", "processing...").unwrap();
    imageops::dither(image, &mapper);
    progress.log_str("dither", "processed!").unwrap();
}

// pub fn pattern_pick(image: &mut RgbImage, mapper: ColorMapper, matrix_size: MatrixSize, progress: ProgressReporter<'_>) {
    // patter
// }