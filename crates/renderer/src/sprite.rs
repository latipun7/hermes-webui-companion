//! Spritesheet parser for the Petdex format.
//!
//! Parses animated pet spritesheets in the standard Codex Pet format:
//! 8 columns × 9 rows grid, 192×208 pixel frames, 9 animation states.

use image::{DynamicImage, GenericImageView, RgbaImage};
use std::io::Cursor;

/// Standard animation states for the Petdex format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationState {
    Idle = 0,
    RunningRight = 1,
    RunningLeft = 2,
    Waving = 3,
    Jumping = 4,
    Failed = 5,
    Waiting = 6,
    Running = 7,
    Review = 8,
}

impl AnimationState {
    pub fn from_row(row: u32) -> Option<Self> {
        match row {
            0 => Some(Self::Idle),
            1 => Some(Self::RunningRight),
            2 => Some(Self::RunningLeft),
            3 => Some(Self::Waving),
            4 => Some(Self::Jumping),
            5 => Some(Self::Failed),
            6 => Some(Self::Waiting),
            7 => Some(Self::Running),
            8 => Some(Self::Review),
            _ => None,
        }
    }
}

/// Dimensions of a single frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSize {
    pub width: u32,
    pub height: u32,
}

/// Grid layout of the spritesheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridLayout {
    pub columns: u32,
    pub rows: u32,
    pub frame_size: FrameSize,
}

/// Standard Petdex layout: 8×9 grid, 192×208px frames.
pub const STANDARD_LAYOUT: GridLayout =
    GridLayout { columns: 8, rows: 9, frame_size: FrameSize { width: 192, height: 208 } };

/// Error type for spritesheet parsing.
#[derive(Debug)]
pub enum SpritesheetError {
    /// The image data could not be decoded.
    Decode(String),
    /// The spritesheet dimensions do not match the expected layout.
    InvalidDimensions { expected: (u32, u32), actual: (u32, u32) },
    /// The requested frame index is out of bounds.
    FrameOutOfBounds { row: u32, col: u32, max_row: u32, max_col: u32 },
}

/// A single animation frame extracted from the spritesheet.
#[derive(Debug, Clone)]
pub struct Frame {
    pub image: RgbaImage,
    pub state: AnimationState,
    pub col: u32,
}

/// Parsed spritesheet ready for rendering.
#[derive(Debug)]
pub struct Spritesheet {
    source: DynamicImage,
    layout: GridLayout,
}

impl Spritesheet {
    /// Parse a spritesheet from raw bytes (WebP or PNG).
    pub fn from_bytes(data: &[u8]) -> Result<Self, SpritesheetError> {
        let img = image::load(Cursor::new(data), image::ImageFormat::WebP)
            .or_else(|_| image::load(Cursor::new(data), image::ImageFormat::Png))
            .map_err(|e| SpritesheetError::Decode(e.to_string()))?;

        Ok(Self { source: img, layout: STANDARD_LAYOUT })
    }

    /// Validate that the loaded image matches the expected grid dimensions.
    pub fn validate(&self) -> Result<(), SpritesheetError> {
        let expected_w = self.layout.columns * self.layout.frame_size.width;
        let expected_h = self.layout.rows * self.layout.frame_size.height;
        let (actual_w, actual_h) = self.source.dimensions();

        if actual_w != expected_w || actual_h != expected_h {
            return Err(SpritesheetError::InvalidDimensions {
                expected: (expected_w, expected_h),
                actual: (actual_w, actual_h),
            });
        }
        Ok(())
    }

    /// Extract a single frame by grid position (row, col).
    pub fn frame(&self, row: u32, col: u32) -> Result<Frame, SpritesheetError> {
        if row >= self.layout.rows || col >= self.layout.columns {
            return Err(SpritesheetError::FrameOutOfBounds {
                row,
                col,
                max_row: self.layout.rows.saturating_sub(1),
                max_col: self.layout.columns.saturating_sub(1),
            });
        }

        let fs = &self.layout.frame_size;
        let x = col * fs.width;
        let y = row * fs.height;
        let sub = self.source.crop_imm(x, y, fs.width, fs.height);

        Ok(Frame {
            image: sub.to_rgba8(),
            state: AnimationState::from_row(row).unwrap_or(AnimationState::Idle),
            col,
        })
    }

    /// Extract all frames for a given animation state (one full row).
    pub fn state_frames(&self, state: AnimationState) -> Result<Vec<Frame>, SpritesheetError> {
        let row = state as u32;
        (0..self.layout.columns).map(|col| self.frame(row, col)).collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a minimal valid spritesheet image (1536×1872 RGB).
    fn make_test_spritesheet() -> Vec<u8> {
        let (w, h) = (1536, 1872);
        // Just a solid RGB image encoded as PNG
        let img = RgbaImage::from_pixel(w, h, image::Rgba([128, 128, 128, 255]));
        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        buf.into_inner()
    }

    #[test]
    fn parse_valid_png_spritesheet() {
        let data = make_test_spritesheet();
        let sheet = Spritesheet::from_bytes(&data).unwrap();
        assert!(matches!(sheet.validate(), Ok(())));
    }

    #[test]
    fn reject_invalid_dimensions() {
        let img = RgbaImage::from_pixel(100, 100, image::Rgba([0, 0, 0, 255]));
        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let data = buf.into_inner();

        let mut sheet = Spritesheet::from_bytes(&data).unwrap();
        // Override layout to still get the real dimensions mismatch
        sheet.layout = STANDARD_LAYOUT;
        let err = sheet.validate().unwrap_err();
        assert!(matches!(err, SpritesheetError::InvalidDimensions { .. }));
    }

    #[test]
    fn extract_first_frame() {
        let data = make_test_spritesheet();
        let sheet = Spritesheet::from_bytes(&data).unwrap();
        let frame = sheet.frame(0, 0).unwrap();
        assert_eq!(frame.state, AnimationState::Idle);
        assert_eq!(frame.col, 0);
        assert_eq!(frame.image.width(), 192);
        assert_eq!(frame.image.height(), 208);
    }

    #[test]
    fn extract_state_frames_idle() {
        let data = make_test_spritesheet();
        let sheet = Spritesheet::from_bytes(&data).unwrap();
        let frames = sheet.state_frames(AnimationState::Idle).unwrap();
        assert_eq!(frames.len(), 8);
        for f in &frames {
            assert_eq!(f.state, AnimationState::Idle);
        }
    }

    #[test]
    fn frame_out_of_bounds() {
        let data = make_test_spritesheet();
        let sheet = Spritesheet::from_bytes(&data).unwrap();
        assert!(sheet.frame(9, 0).is_err());
        assert!(sheet.frame(0, 8).is_err());
    }
}
