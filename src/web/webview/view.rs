use cosmic::iced::advanced::{
    self,
    graphics::core::event,
    layout,
    renderer::{self},
    widget::Tree,
    Clipboard, Layout, Shell, Widget,
};
use cosmic::iced::event::Status;
use cosmic::iced::keyboard;
use cosmic::iced::mouse::{self, Interaction};
use cosmic::iced::widget::image::{Handle, Image};
use cosmic::iced::{Event, Length, Rectangle};
use cosmic::iced::{Point, Size};
use cosmic::theme::Theme;
use cosmic::Element;
use cosmic::Task;
use url::Url;

use crate::{engine, ImageInfo, PageType, ViewId};

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]

pub enum Action {
    ChangeView(u32),
    CloseCurrentView,
    CloseView(u32),
    CreateView(PageType),
    GoBack,
    GoForward,
    GoToUrl(Url),
    Refresh,
    SendKeyboardEvent(keyboard::Event),
    SendMouseEvent(mouse::Event, Point),
    Update,
    Resize(Size<u32>),
}

pub struct WebView<Engine, Message>
where
    Engine: engine::Engine,
{
    engine: Engine,
    view_size: Size<u32>,
    current_view_index: Option<usize>,
    view_ids: Vec<ViewId>,
    on_close_view: Option<Message>,
    on_create_view: Option<Message>,
    on_url_change: Option<Box<dyn Fn(String) -> Message>>,
    url: String,
    on_title_change: Option<Box<dyn Fn(String) -> Message>>,
    title: String,
}

impl<Engine: engine::Engine + Default, Message: Send + Clone + 'static> WebView<Engine, Message> {
    fn get_current_view_id(&self) -> ViewId {
        *self
            .view_ids
            .get(
                self.current_view_index
                    .expect("The current view index is not set."),
            )
            .expect("Could find view index for view, may be closed.")
    }

    fn index_as_view_id(&self, index: u32) -> usize {
        *self
            .view_ids
            .get(index as usize)
            .expect("Failed to find index")
    }
}

impl<Engine: engine::Engine + Default, Message: Send + Clone + 'static> Default
    for WebView<Engine, Message>
{
    fn default() -> Self {
        WebView {
            engine: Engine::default(),
            view_size: Size {
                width: 1920,
                height: 1080,
            },
            current_view_index: None,
            view_ids: Vec::new(),
            on_close_view: None,
            on_create_view: None,
            on_url_change: None,
            url: String::new(),
            on_title_change: None,
            title: String::new(),
        }
    }
}

impl<Engine: engine::Engine + Default, Message: Send + Clone + 'static> WebView<Engine, Message> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_create_view(mut self, on_create_view: Message) -> Self {
        self.on_create_view = Some(on_create_view);
        self
    }

    pub fn on_close_view(mut self, on_close_view: Message) -> Self {
        self.on_close_view = Some(on_close_view);
        self
    }

    pub fn on_url_change(mut self, on_url_change: impl Fn(String) -> Message + 'static) -> Self {
        self.on_url_change = Some(Box::new(on_url_change));
        self
    }

    pub fn on_title_change(
        mut self,
        on_title_change: impl Fn(String) -> Message + 'static,
    ) -> Self {
        self.on_title_change = Some(Box::new(on_title_change));
        self
    }

    pub fn update(&mut self, action: Action) -> Task<Message> {}
}
