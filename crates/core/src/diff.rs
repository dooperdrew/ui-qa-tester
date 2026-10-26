//! Perceptual hashing (dHash), visual baseline regression diffing, and PNG bounding-box
//! annotation (`M1.3`).

use crate::model::{BoundingRect, UiElement};
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualDiffSummary {
    pub width: u32,
    pub height: u32,
    pub changed_pixels: u64,
    pub total_pixels: u64,
    pub changed_ratio: f64,
    pub ssim_approx: f64,
    pub changed_bounding_box: Option<BoundingRect>,
}

/// Computes a 64-bit difference hash (dHash) by sampling a 9×8 luminance grid.
pub fn compute_dhash(img: &RgbaImage) -> u64 {
    let w = img.width().max(1);
    let h = img.height().max(1);
    let mut grid = [[0u16; 9]; 8];

    for (row, grid_row) in grid.iter_mut().enumerate() {
        let y = ((row as u32) * h / 8).min(h - 1);
        for (col, cell) in grid_row.iter_mut().enumerate() {
            let x = ((col as u32) * w / 9).min(w - 1);
            let p = img.get_pixel(x, y).0;
            *cell = (p[0] as u16 * 77 + p[1] as u16 * 150 + p[2] as u16 * 29) >> 8;
        }
    }

    let mut hash = 0u64;
    let mut bit = 0;
    for grid_row in &grid {
        for col in 0..8 {
            if grid_row[col] > grid_row[col + 1] {
                hash |= 1u64 << bit;
            }
            bit += 1;
        }
    }
    hash
}

/// Hamming distance between two 64-bit perceptual hashes (`0..=64`).
pub fn dhash_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Compares `baseline` and `current` frames, producing a diff summary and a visual diff heatmap image.
pub fn compare_frames(
    baseline: &RgbaImage,
    current: &RgbaImage,
    pixel_threshold: u8,
) -> (VisualDiffSummary, RgbaImage) {
    let width = baseline.width().min(current.width()).max(1);
    let height = baseline.height().min(current.height()).max(1);
    let mut diff_img = current.clone();

    let mut changed_pixels = 0u64;
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut sum_sq_err = 0f64;

    for y in 0..height {
        for x in 0..width {
            let a = baseline.get_pixel(x, y).0;
            let b = current.get_pixel(x, y).0;
            let dr = a[0].abs_diff(b[0]);
            let dg = a[1].abs_diff(b[1]);
            let db = a[2].abs_diff(b[2]);
            let max_d = dr.max(dg).max(db);

            let err = (dr as f64 + dg as f64 + db as f64) / (3.0 * 255.0);
            sum_sq_err += err * err;

            if max_d > pixel_threshold {
                changed_pixels += 1;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                // Highlight changed pixel in bright magenta/red on the diff image
                diff_img.put_pixel(x, y, Rgba([255, 40, 120, 255]));
            }
        }
    }

    let total_pixels = (width as u64) * (height as u64);
    let changed_ratio = (changed_pixels as f64) / (total_pixels.max(1) as f64);
    let mse = sum_sq_err / (total_pixels.max(1) as f64);
    let ssim_approx = (1.0 - mse.sqrt()).clamp(0.0, 1.0);

    let changed_bounding_box = if changed_pixels > 0 {
        let rect = BoundingRect::new(
            min_x as i32,
            min_y as i32,
            (max_x - min_x + 1).max(1),
            (max_y - min_y + 1).max(1),
        );
        draw_rect_outline(&mut diff_img, &rect, Rgba([255, 215, 0, 255]), 2);
        Some(rect)
    } else {
        None
    };

    (
        VisualDiffSummary {
            width,
            height,
            changed_pixels,
            total_pixels,
            changed_ratio,
            ssim_approx,
            changed_bounding_box,
        },
        diff_img,
    )
}

/// Produces an annotated copy of `img` with color-coded bounding boxes for UI elements and anomalies.
pub fn annotate_frame(
    img: &RgbaImage,
    elements: &[UiElement],
    anomaly_rects: &[BoundingRect],
    active_target: Option<&BoundingRect>,
) -> RgbaImage {
    let mut out = img.clone();
    for el in elements {
        let color = if el.supports_invoke {
            Rgba([0, 180, 255, 255]) // Cyan/Blue for interactive controls
        } else {
            Rgba([140, 140, 160, 255])
        };
        draw_rect_outline(&mut out, &el.bounds, color, 2);
    }

    for anomaly in anomaly_rects {
        draw_rect_outline(&mut out, anomaly, Rgba([255, 50, 50, 255]), 3);
    }

    if let Some(target) = active_target {
        draw_rect_outline(&mut out, target, Rgba([50, 255, 100, 255]), 3);
    }

    out
}

pub fn draw_rect_outline(
    img: &mut RgbaImage,
    rect: &BoundingRect,
    color: Rgba<u8>,
    thickness: u32,
) {
    let w = img.width() as i32;
    let h = img.height() as i32;
    if w <= 0 || h <= 0 || rect.width == 0 || rect.height == 0 {
        return;
    }

    let x0 = rect.x.clamp(0, w - 1);
    let y0 = rect.y.clamp(0, h - 1);
    let x1 = (rect.x + rect.width as i32 - 1).clamp(0, w - 1);
    let y1 = (rect.y + rect.height as i32 - 1).clamp(0, h - 1);

    for t in 0..(thickness as i32) {
        let top = (y0 + t).clamp(0, h - 1);
        let bottom = (y1 - t).clamp(0, h - 1);
        for x in x0..=x1 {
            img.put_pixel(x as u32, top as u32, color);
            img.put_pixel(x as u32, bottom as u32, color);
        }

        let left = (x0 + t).clamp(0, w - 1);
        let right = (x1 - t).clamp(0, w - 1);
        for y in y0..=y1 {
            img.put_pixel(left as u32, y as u32, color);
            img.put_pixel(right as u32, y as u32, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dhash_and_visual_diff_detect_changed_region() {
        let mut img_a = RgbaImage::from_pixel(120, 100, Rgba([30, 40, 50, 255]));
        let mut img_b = img_a.clone();

        for y in 20..40 {
            for x in 30..60 {
                img_b.put_pixel(x, y, Rgba([240, 240, 240, 255]));
            }
        }
        for y in 0..50 {
            for x in 0..60 {
                img_a.put_pixel(x, y, Rgba([10, 10, 10, 255]));
            }
        }

        let (summary, _diff_img) = compare_frames(&img_a, &img_b, 15);
        assert!(summary.changed_pixels > 0);
        assert!(summary.ssim_approx < 1.0);
        assert!(summary.changed_bounding_box.is_some());
    }
}
