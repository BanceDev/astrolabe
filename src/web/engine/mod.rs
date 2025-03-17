use crate::web::ImageInfo;
use cosmic::iced::keyboard;
use cosmic::iced::mouse::{self, Interaction};
use cosmic::iced::Point;
use cosmic::iced::Size;

pub mod webkitgtk;

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum PageType {
    Url(String),
    Html(String),
}

pub enum PixelFormat {
    Rgba,
    Bgra,
}

pub type ViewId = usize;

pub trait Engine {
    fn update(&mut self);
    fn render(&mut self, size: Size<u32>);
    fn request_render(&mut self, id: ViewId, size: Size<u32>);
    fn new_view(&mut self, size: Size<u32>, content: Option<PageType>) -> ViewId;
    fn remove_view(&mut self, id: ViewId);

    fn focus(&mut self);
    fn unfocus(&self);
    fn resize(&mut self, size: Size<u32>);

    fn handle_keyboard_event(&mut self, id: ViewId, event: keyboard::Event);
    fn handle_mouse_event(&mut self, id: ViewId, point: Point, event: mouse::Event);
    fn scroll(&mut self, id: ViewId, delta: mouse::ScrollDelta);

    fn goto(&mut self, id: ViewId, page_type: PageType);
    fn refresh(&mut self, id: ViewId);
    fn go_forward(&mut self, id: ViewId);
    fn go_back(&mut self, id: ViewId);

    fn get_url(&self, id: ViewId) -> String;
    fn get_title(&self, id: ViewId) -> String;
    fn get_cursor(&self, id: ViewId) -> Interaction;
    fn get_view(&self, id: ViewId) -> &ImageInfo;
}
