use cosmic::app::Task;
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
use url::Url;

use crate::startpage;
use crate::web::{engine, ImageInfo, PageType, ViewId};

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]

pub enum Action {
    ChangeView(u32),
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

    pub fn update(&mut self, action: Action) -> Task<Message> {
        let mut tasks = Vec::new();

        if self.current_view_index.is_some() {
            if let Some(on_url_change) = &self.on_url_change {
                let url = self.engine.get_url(self.get_current_view_id());
                if self.url != url {
                    self.url = url.clone();
                    tasks.push(cosmic::Task::done(on_url_change(url)).map(cosmic::Action::from))
                }
            }
            if let Some(on_title_change) = &self.on_title_change {
                let title = self.engine.get_title(self.get_current_view_id());
                if self.title != title {
                    self.title = title.clone();
                    tasks.push(cosmic::Task::done(on_title_change(title)).map(cosmic::Action::from))
                }
            }
        }

        match action {
            Action::ChangeView(index) => {
                {
                    self.view_size.width += 10;
                    self.view_size.height -= 10;
                    self.engine.resize(self.view_size);
                    self.view_size.width -= 10;
                    self.view_size.height += 10;
                    self.engine.resize(self.view_size);
                    self.engine
                        .request_render(self.index_as_view_id(index), self.view_size);
                }
                self.current_view_index = Some(index as usize);
            }
            Action::CloseView(index) => {
                let id = self.index_as_view_id(index);
                self.view_ids.remove(index as usize);
                self.engine.remove_view(id);

                // only change view if current or lower is closed
                if let Some(cur_idx) = self.current_view_index {
                    let cur_idx = if cur_idx == 0 { 0 } else { cur_idx - 1 };
                    if index as usize <= cur_idx {
                        {
                            self.view_size.width += 10;
                            self.view_size.height -= 10;
                            self.engine.resize(self.view_size);
                            self.view_size.width -= 10;
                            self.view_size.height += 10;
                            self.engine.resize(self.view_size);
                            self.engine.request_render(
                                self.index_as_view_id((cur_idx) as u32),
                                self.view_size,
                            );
                        }
                        self.current_view_index = Some((cur_idx) as usize);
                    }
                }

                if let Some(on_close_view) = &self.on_close_view {
                    tasks.push(cosmic::Task::done(on_close_view.clone()).map(cosmic::Action::from))
                }
            }
            Action::CreateView(page_type) => {
                let id = self.engine.new_view(self.view_size, Some(page_type));
                self.view_ids.push(id);

                if let Some(on_create_view) = &self.on_create_view {
                    tasks.push(cosmic::Task::done(on_create_view.clone()).map(cosmic::Action::from))
                }
            }
            Action::GoBack => {
                self.engine.go_back(self.get_current_view_id());
            }
            Action::GoForward => {
                self.engine.go_forward(self.get_current_view_id());
            }
            Action::GoToUrl(url) => {
                self.engine
                    .goto(self.get_current_view_id(), PageType::Url(url.to_string()));
            }
            Action::Refresh => {
                self.engine.refresh(self.get_current_view_id());
            }
            Action::SendKeyboardEvent(event) => {
                self.engine
                    .handle_keyboard_event(self.get_current_view_id(), event);
            }
            Action::SendMouseEvent(point, event) => {
                self.engine
                    .handle_mouse_event(self.get_current_view_id(), event, point);
            }
            Action::Update => {
                self.engine.update();
                if self.current_view_index.is_some() {
                    self.engine
                        .request_render(self.get_current_view_id(), self.view_size);
                }
                return Task::batch(tasks);
            }
            Action::Resize(size) => {
                self.view_size = size;
                self.engine.resize(size);
            }
        };

        if self.current_view_index.is_some() {
            self.engine
                .request_render(self.get_current_view_id(), self.view_size);
        }

        Task::batch(tasks)
    }

    pub fn view(&self) -> Element<Action> {
        WebViewWidget::new(
            self.engine.get_view(self.get_current_view_id()),
            self.engine.get_cursor(self.get_current_view_id()),
        )
        .into()
    }

    pub fn init(&mut self) {
        let id = self.engine.new_view(
            self.view_size,
            // TODO: put a homepage app here
            Some(PageType::Html(startpage::get_startpage())),
        );
        self.view_ids.push(id);
        self.current_view_index = Some(0);
    }

    pub fn get_current_view_title(&self) -> String {
        self.engine.get_title(self.get_current_view_id())
    }

    pub fn get_view_title(&self, index: u32) -> String {
        self.engine.get_title(self.index_as_view_id(index))
    }
}

struct WebViewWidget<'a> {
    image_info: &'a ImageInfo,
    cursor: Interaction,
}

impl<'a> WebViewWidget<'a> {
    fn new(image_info: &'a ImageInfo, cursor: Interaction) -> Self {
        Self { image_info, cursor }
    }
}

impl<Renderer> Widget<Action, Theme, Renderer> for WebViewWidget<'_>
where
    Renderer:
        cosmic::iced::advanced::image::Renderer<Handle = cosmic::iced::advanced::image::Handle>,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(
        &self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max())
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        <Image<Handle> as Widget<Action, Theme, Renderer>>::draw(
            &self.image_info.as_image(),
            tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        )
    }

    fn on_event(
        &mut self,
        _state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Action>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let size = Size::new(layout.bounds().width as u32, layout.bounds().height as u32);
        if self.image_info.width != size.width || self.image_info.height != size.height {
            shell.publish(Action::Resize(size));
        }

        match event {
            Event::Keyboard(event) => {
                shell.publish(Action::SendKeyboardEvent(event));
            }
            Event::Mouse(event) => {
                if let Some(point) = cursor.position_in(layout.bounds()) {
                    shell.publish(Action::SendMouseEvent(event, point));
                }
            }
            _ => (),
        }
        Status::Ignored
    }

    fn mouse_interaction(
        &self,
        _state: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            self.cursor
        } else {
            mouse::Interaction::Idle
        }
    }
}

impl<'a, Message: 'a, Renderer> From<WebViewWidget<'a>>
    for cosmic::iced::Element<'a, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer + advanced::image::Renderer<Handle = advanced::image::Handle>,
    WebViewWidget<'a>: Widget<Message, Theme, Renderer>,
{
    fn from(widget: WebViewWidget<'a>) -> Self {
        Self::new(widget)
    }
}
