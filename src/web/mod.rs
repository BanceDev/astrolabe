use cosmic::widget::image;

pub mod engine;
pub use engine::{Engine, PageType, PixelFormat, ViewId};

mod webview;
pub use view::{Action, WebView};
pub use webview::view;

pub use engine::ultralight::Ultralight;

#[derive(Clone, Debug, PartialEq)]
pub struct ImageInfo {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

impl Default for ImageInfo {
    fn default() -> Self {
        Self {
            pixels: vec![255; (Self::WIDTH as usize * Self::HEIGHT as usize) * 4],
            width: Self::WIDTH,
            height: Self::HEIGHT,
        }
    }
}

impl ImageInfo {
    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 800;

    fn new(mut pixels: Vec<u8>, format: PixelFormat, width: u32, height: u32) -> Self {
        assert_eq!(pixels.len() % 4, 0);

        let pixels = match format {
            PixelFormat::Rgba => pixels,
            PixelFormat::Bgra => {
                pixels.chunks_exact_mut(4).for_each(|chunk| {
                    chunk.swap(0, 2);
                });
                pixels
            }
        };

        Self {
            pixels,
            width,
            height,
        }
    }

    fn as_image(&self) -> image::Image<image::Handle> {
        image::Image::new(image::Handle::from_rgba(
            self.width,
            self.height,
            self.pixels.clone(),
        ))
    }

    fn blank(width: u32, height: u32) -> Self {
        Self {
            pixels: vec![255; (width as usize * height as usize) * 4],
            width,
            height,
        }
    }
}
