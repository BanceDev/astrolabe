// SPDX-License-Identifier: MIT

use crate::config::Config;
use crate::fl;
use crate::startpage;
use crate::web;
use cosmic::app::{context_drawer, Action, Core, Task};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::keyboard::Key;
use cosmic::iced::{time, Alignment, Length, Subscription};
use cosmic::iced_core::keyboard::key::Named;
use cosmic::widget::menu::key_bind::{KeyBind, Modifier};
use cosmic::widget::{self, icon, menu, nav_bar};
use cosmic::{cosmic_theme, theme, Application, ApplicationExt, Apply, Element};
use futures_util::SinkExt;
use std::collections::HashMap;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// Contains items assigned to the nav bar panel.
    nav: nav_bar::Model,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    // Configuration data that persists between application runs.
    config: Config,
    // Embedded web view
    webview: web::WebView<web::Ultralight, Message>,
    // url of the webview
    webview_url: Option<String>,
    // the current view
    current_view: Option<u32>,
    // view count
    num_views: u32,
    // id for search bar
    search_id: widget::Id,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    OpenRepositoryUrl,
    SubscriptionChannel,
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
    LaunchUrl(String),
    WebView(web::Action),
    WebViewCreated,
    UrlChanged(String),
    TitleChanged(String),
    CycleWebView,
    GotoTab(u32),
    NewTab,
    CloseTab(nav_bar::Id),
    Update,
}

/// Create a COSMIC application from the app model
impl Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.bancedev.astrolabe";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        // Create a nav bar with three page items.
        let nav = nav_bar::Model::default();

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            nav,
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => {
                        // for why in errors {
                        //     tracing::error!(%why, "error loading app config");
                        // }

                        config
                    }
                })
                .unwrap_or_default(),
            webview: web::WebView::new()
                .on_create_view(Message::WebViewCreated)
                .on_url_change(Message::UrlChanged)
                .on_title_change(Message::TitleChanged),
            webview_url: None,
            current_view: Some(0), // this will lead to a crash if init isnt called
            num_views: 1,
            search_id: widget::Id::unique(),
        };

        // map keybinds
        macro_rules! bind {
            ([$($modifier:ident),* $(,)?], $key:expr, $action:ident) => {{
                app.key_binds.insert(
                    KeyBind {
                        modifiers: vec![$(Modifier::$modifier),*],
                        key: $key,
                    },
                    MenuAction::$action,
                );
            }};
        }
        bind!([Ctrl], Key::Character("t".into()), NewTab);

        app.webview.init();
        // Create a startup command that sets the window title.
        let command = app.update_title();

        app.nav
            .insert()
            .text(app.webview.get_view_title(0))
            .data::<u32>(0)
            .icon(icon::from_name("text-html-symbolic"))
            .closable()
            .activate();

        (app, command)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<Self::Message>> {
        let menu_bar = menu::bar(vec![
            menu::Tree::with_children(
                menu::root(fl!("file")),
                menu::items(
                    &self.key_binds,
                    vec![menu::Item::Button(fl!("new-tab"), None, MenuAction::NewTab)],
                ),
            ),
            menu::Tree::with_children(
                menu::root(fl!("view")),
                menu::items(
                    &self.key_binds,
                    vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
                ),
            ),
        ]);

        vec![menu_bar.into()]
    }

    fn header_center(&self) -> Vec<Element<Self::Message>> {
        let mut elements = Vec::with_capacity(2);

        if let Some(term) = self.webview_url.clone() {
            if self.core.is_condensed() {
                elements.push(
                    widget::button::icon(widget::icon::from_name("system-search-symbolic"))
                        .on_press(Message::GotoTab(0))
                        .padding(8)
                        .selected(true)
                        .into(),
                );
            } else {
                elements.push(
                    widget::text_input::search_input("", term)
                        .width(Length::Fill)
                        .id(self.search_id.clone())
                        .on_clear(Message::NewTab)
                        .on_input(Message::UrlChanged)
                        .into(),
                );
            }
        } else {
            elements.push(
                widget::button::icon(widget::icon::from_name("system-search-symbolic"))
                    .on_press(Message::NewTab)
                    .padding(8)
                    .into(),
            );
        }

        elements
    }

    fn nav_bar(&self) -> Option<Element<cosmic::Action<Self::Message>>> {
        if !self.core().nav_bar_active() {
            return None;
        }

        let nav_model = self.nav_model()?;

        let mut nav =
            cosmic::widget::nav_bar(nav_model, |id| cosmic::Action::Cosmic(Action::NavBar(id)))
                .on_context(|id| cosmic::Action::Cosmic(Action::NavBarContext(id)))
                .close_icon(
                    widget::icon::from_name("window-close-symbolic")
                        .size(16)
                        .icon(),
                )
                .on_close(|id| cosmic::Action::App(Message::CloseTab(id)))
                .into_container()
                .width(Length::Shrink)
                .height(Length::Shrink);

        if !self.core().is_condensed() {
            nav = nav.max_width(225);
        }

        Some(Element::from(nav))
    }

    /// Enables the COSMIC application to create a nav bar with this model.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::context_drawer(
                self.about(),
                Message::ToggleContextPage(ContextPage::About),
            )
            .title(fl!("about")),
        })
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<Self::Message> {
        self.webview.view().map(Message::WebView).into()
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
    fn subscription(&self) -> Subscription<Self::Message> {
        struct MySubscription;

        Subscription::batch(vec![
            // Create a subscription which emits updates through a channel.
            Subscription::run_with_id(
                std::any::TypeId::of::<MySubscription>(),
                cosmic::iced::stream::channel(4, move |mut channel| async move {
                    _ = channel.send(Message::SubscriptionChannel).await;

                    futures_util::future::pending().await
                }),
            ),
            // Watch for application configuration changes.
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| {
                    // for why in update.errors {
                    //     tracing::error!(?why, "app config error");
                    // }
                    Message::UpdateConfig(update.config)
                }),
            time::every(std::time::Duration::from_millis(10))
                .map(|_| web::Action::Update)
                .map(Message::WebView),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::OpenRepositoryUrl => {
                _ = open::that_detached(REPOSITORY);
            }

            Message::SubscriptionChannel => {
                // For example purposes only.
            }

            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }

            Message::UpdateConfig(config) => {
                self.config = config;
            }

            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("failed to open {url:?}: {err}");
                }
            },

            Message::WebView(msg) => {
                return self.webview.update(msg);
            }

            Message::WebViewCreated => {
                self.num_views += 1;
                return cosmic::Task::done(Message::CycleWebView).map(cosmic::Action::from);
            }

            Message::UrlChanged(url) => {
                self.webview_url = Some(url);
                self.nav
                    .text_set(self.nav.active(), self.webview.get_current_view_title());
            }

            Message::TitleChanged(title) => {
                self.nav.text_set(self.nav.active(), title);
            }

            Message::CycleWebView => {
                self.current_view = Some(0);
                return self
                    .webview
                    .update(web::Action::ChangeView(self.num_views - 1));
            }

            Message::GotoTab(tab) => {
                if tab <= self.num_views {
                    return self.webview.update(web::Action::ChangeView(tab));
                }
            }

            Message::Update => {
                return self.webview.update(web::Action::Update);
            }

            Message::NewTab => {
                self.nav
                    .insert()
                    .text("")
                    .data::<u32>(self.num_views)
                    .icon(icon::from_name("text-html-symbolic"))
                    .closable()
                    .activate();

                return self
                    .webview
                    .update(web::Action::CreateView(web::PageType::Html(
                        startpage::get_startpage(),
                    )))
                    .map(cosmic::Action::from);
            }

            Message::CloseTab(id) => {
                if let Some(view_index) = self.nav.data::<u32>(id) {
                    self.num_views -= 1;
                    // if they close the last tab exit gracefully
                    if self.num_views < 1 {
                        return cosmic::iced::exit();
                    }
                    let task: Task<Message> = self
                        .webview
                        .update(web::Action::CloseView(*view_index))
                        .map(cosmic::Action::from);

                    // shift down the index of every tab above the one removed
                    let mut updates = Vec::new();
                    for tab in self.nav.iter() {
                        if let Some(index) = self.nav.data::<u32>(tab) {
                            if index > view_index {
                                updates.push((tab, index - 1));
                            }
                        }
                    }

                    for (tab, new_index) in updates {
                        self.nav.data_set::<u32>(tab, new_index);
                    }

                    self.nav.remove(id);
                    return task;
                }
            }

            _ => (),
        }
        Task::none()
    }

    /// Called when a nav item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<Self::Message> {
        // Activate the page in the model.
        self.nav.activate(id);

        // change current web view
        let mut tasks = Vec::new();
        if let Some(tab) = self.nav.data::<u32>(id) {
            tasks.push(cosmic::Task::done(Message::GotoTab(*tab)).map(cosmic::Action::from))
        }
        tasks.push(self.update_title());

        Task::batch(tasks)
    }
}

impl AppModel {
    /// The about page for this app.
    pub fn about(&self) -> Element<Message> {
        let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

        let icon = widget::svg(widget::svg::Handle::from_memory(APP_ICON));

        let title = widget::text::title3(fl!("app-title"));

        let hash = env!("VERGEN_GIT_SHA");
        let short_hash: String = hash.chars().take(7).collect();
        let date = env!("VERGEN_GIT_COMMIT_DATE");

        let link = widget::button::link(REPOSITORY)
            .on_press(Message::OpenRepositoryUrl)
            .padding(0);

        widget::column()
            .push(icon)
            .push(title)
            .push(link)
            .push(
                widget::button::link(fl!(
                    "git-description",
                    hash = short_hash.as_str(),
                    date = date
                ))
                .on_press(Message::LaunchUrl(format!("{REPOSITORY}/commits/{hash}")))
                .padding(0),
            )
            .align_x(Alignment::Center)
            .spacing(space_xxs)
            .into()
    }

    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<Message> {
        let mut window_title = fl!("app-title");

        if let Some(page) = self.nav.text(self.nav.active()) {
            window_title.push_str(" — ");
            window_title.push_str(page);
        }

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
    NewTab,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::ToggleContextPage(ContextPage::About),
            MenuAction::NewTab => Message::NewTab,
        }
    }
}
