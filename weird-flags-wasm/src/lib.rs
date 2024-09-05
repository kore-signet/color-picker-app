mod utils;
mod dither;
mod pattern;

use std::{cell::RefCell, io::Cursor, str::FromStr};

use fnv::FnvHashMap;
use image::{imageops::ColorMap, EncodableLayout, ImageFormat, RgbImage};
use kasi_kule::{Jab, UCS};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

struct ProgressReporter<'a> {
    inner: &'a js_sys::Function,
}

impl<'a> ProgressReporter<'a> {
    fn log_pct(&self, stage: &'static str, progress: f64) -> Result<(), JsValue> {
        self.inner.call2(
            &JsValue::null(),
            &JsValue::from_str(stage),
            &JsValue::from_str(&format!("{progress:.2}%")),
        )?;
        Ok(())
    }

    fn log_str(&self, stage: &'static str, progress: &str) -> Result<(), JsValue> {
        self.inner.call2(
            &JsValue::null(),
            &JsValue::from_str(stage),
            &JsValue::from_str(progress),
        )?;
        Ok(())
    }
}

enum DitherMethod {
    None,
    FloydSteinberg,
    Pattern2x2,
    Pattern4x4,
    Pattern8x8
}

impl FromStr for DitherMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "none" => DitherMethod::None,
            "floyd-steinberg" => DitherMethod::FloydSteinberg,
            "pattern-2x2" => DitherMethod::Pattern2x2,
            "pattern-4x4" => DitherMethod::Pattern4x4,
            "pattern-8x8" => DitherMethod::Pattern8x8,
            _ => return Err(())
        })
    }
}

struct ColorMapper {
    pub palette_rgb: Vec<[u8; 3]>,
    palette_jab: Vec<Jab<UCS>>,
    cache: RefCell<FnvHashMap<[u8; 3], usize>>
}

impl ColorMapper {
    pub fn build_from_image(image: &image::RgbImage, colors_to_extract: u8, extract_quality: u8) -> ColorMapper {
        let palette = palette_extract::get_palette_with_options(
            &image.as_bytes(),
            palette_extract::PixelEncoding::Rgb,
            palette_extract::Quality::new(extract_quality),
            palette_extract::MaxColors::new(colors_to_extract),
            palette_extract::PixelFilter::White,
        );
        
        let palette: Vec<[u8; 3]> = palette.into_iter().map(|palette_extract::Color { r, g, b}| [r,g,b]).collect();
        let jabs: Vec<Jab<UCS>> = palette.iter().map(|v| Jab::from(kasi_kule::sRGB::from(*v))).collect();

        ColorMapper {
            palette_rgb: palette,
            palette_jab: jabs,
            cache: RefCell::new(FnvHashMap::with_capacity_and_hasher(4096, Default::default())),
        }
    }
}

impl ColorMap for ColorMapper {
    type Color = image::Rgb<u8>;

    fn index_of(&self, color: &Self::Color) -> usize {
        if let Some(idx) = self.cache.borrow().get(&color.0) {
            return *idx
        };

        let to_match_jab: Jab<UCS> = Jab::from(kasi_kule::sRGB::from(color.0));
       
        let mut best = (0, f32::MAX);

        for (idx, color) in self.palette_jab.iter().enumerate() {
            let dist = color.squared_difference(&to_match_jab);
            if dist < best.1 {
                best = (idx, dist);
            }
        }


        self.cache.borrow_mut().insert(color.0, best.0);

        best.0
    }

    fn map_color(&self, color: &mut Self::Color) {
        color.0 = self.palette_rgb[self.index_of(color)];
    }

    fn lookup(&self, index: usize) -> Option<Self::Color> {
        self.palette_rgb.get(index).map(|v| image::Rgb(*v))
    }

    fn has_lookup(&self) -> bool {
        true
    }

}


#[wasm_bindgen]
pub fn process(
    original_image: Vec<u8>,
    color_image: Vec<u8>,
    colors_to_extract: u8,
    extract_quality: u8,
    dither_mode: String,
    callback: &js_sys::Function,
) -> Result<Vec<u8>, JsValue> {
    let progress = ProgressReporter { inner: callback };

    progress.log_str("wah", &dither_mode);

    progress.log_str("image-load", "loading original image")?;
    let mut to_transform = image::load_from_memory(&original_image).unwrap().to_rgb8();
    progress.log_str("image-load", "loading color source")?;
    let color_source = image::load_from_memory(&color_image).unwrap().to_rgb8();
    progress.log_str("image-load", "loaded!")?;

    progress.log_str("extract-color", "extracting palette")?;
    let mapper = ColorMapper::build_from_image(&color_source, colors_to_extract, extract_quality);
    progress.log_str("extract-color", "palette extracted")?;

    let dither_mode = DitherMethod::from_str(&dither_mode).unwrap();

    progress.log_str("extract-color", "dither method parsed");

    dither::color_pick(&mut to_transform, mapper, dither_mode, progress);

    let mut output = Cursor::new(Vec::with_capacity(to_transform.as_bytes().len()));
    to_transform.write_to(&mut output, ImageFormat::Png).unwrap();
    Ok(output.into_inner())
}
