use clipboard_rs::{Clipboard, ClipboardContext};
use cosmic::iced::keyboard::{self};
use cosmic::iced::mouse::{self, ScrollDelta};
use cosmic::iced::{Point, Size};
use rand::Rng;
use smol_str::SmolStr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::{env::var, path::Path};
use ul_next::{
    config::Config,
    event::{self, KeyEventCreationInfo, MouseEvent, ScrollEvent},
    key_code::VirtualKeyCode,
    platform,
    renderer::Renderer,
    view,
    window::Cursor,
};

use super::{Engine, PageType, PixelFormat, ViewId};
use crate::web::ImageInfo;

struct UlClipboard {
    ctx: ClipboardContext,
}

impl platform::Clipboard for UlClipboard {
    fn clear(&mut self) {}

    fn read_plain_text(&mut self) -> Option<String> {
        Some(self.ctx.get_text().unwrap_or("".to_string()))
    }

    fn write_plain_text(&mut self, text: &str) {
        self.ctx
            .set_text(text.into())
            .expect("Failed to set contents of clipboard");
    }
}

/// Holds Ultralight View info like surfaces for rendering and urls & titles
pub struct View {
    id: ViewId,
    view: view::View,
    cursor: Arc<RwLock<mouse::Interaction>>,
    last_frame: ImageInfo,
    was_loading: bool,
    cursor_pos: Point,
}

impl View {
    fn update_cursor_pos(&mut self) {
        let cursor_pos = self.cursor_pos;
        self.view.fire_mouse_event(
            MouseEvent::new(
                ul_next::event::MouseEventType::MouseMoved,
                cursor_pos.x as i32,
                cursor_pos.y as i32,
                ul_next::event::MouseButton::None,
            )
            .expect("Ultralight failed to fire mouse input"),
        );
    }
}

/// Implementation of the Ultralight browsing engine for iced_webivew
pub struct Ultralight {
    renderer: Renderer,
    view_config: view::ViewConfig,
    views: Vec<View>,
}

impl Default for Ultralight {
    fn default() -> Self {
        let config = Config::start().build().expect("Failed to start Ultralight");
        platform::enable_platform_fontloader();
        platform::enable_platform_filesystem(platform_filesystem())
            .expect("Failed to get platform filesystem");
        platform::set_clipboard(UlClipboard {
            ctx: ClipboardContext::new().expect("Failed to get ownership of clipboard"),
        });

        let renderer = Renderer::create(config).expect("Failed to create ultralight renderer");
        let view_config = view::ViewConfig::start()
            .initial_device_scale(1.0)
            .font_family_standard("Arial")
            .is_accelerated(false)
            .build()
            .unwrap();

        Self {
            renderer,
            view_config,
            views: Vec::new(),
        }
    }
}

impl Ultralight {
    /// Creates a new Ultralight adapter
    pub fn new(font: &str, scale: f64) -> Self {
        Self {
            view_config: view::ViewConfig::start()
                .initial_device_scale(scale)
                .font_family_standard(font)
                // iced_webview does not currently support acceleration
                .is_accelerated(false)
                .build()
                .unwrap(),
            ..Default::default()
        }
    }

    fn get_view(&self, id: ViewId) -> &View {
        self.views
            .iter()
            .find(|&view| view.id == id)
            .expect("The requested View id was not found")
    }

    fn get_view_mut(&mut self, id: ViewId) -> &mut View {
        self.views
            .iter_mut()
            .find(|view| view.id == id)
            .expect("The requested View id was not found")
    }
}

impl Engine for Ultralight {
    fn update(&mut self) {
        self.renderer.update();
    }

    fn render(&mut self, size: Size<u32>) {
        // for each view save frame
        for view in self.views.iter_mut() {
            view.update_cursor_pos();
            if view.view.needs_paint() || view.was_loading && !view.view.is_loading() {
                if let Some(pixels) = view.view.surface().unwrap().lock_pixels() {
                    view.last_frame =
                        ImageInfo::new(pixels.to_vec(), PixelFormat::Bgra, size.width, size.height);
                    view.was_loading = false;
                }
            }
        }
    }

    fn request_render(&mut self, id: ViewId, size: Size<u32>) {
        self.get_view_mut(id).update_cursor_pos();
        self.get_view(id).view.set_needs_paint(true);
        self.renderer.render();
        if let Some(pixels) = self.get_view(id).view.surface().unwrap().lock_pixels() {
            self.get_view_mut(id).last_frame =
                ImageInfo::new(pixels.to_vec(), PixelFormat::Rgba, size.width, size.height);
            self.get_view_mut(id).was_loading = false
        }
    }

    fn new_view(&mut self, size: Size<u32>, page_type: Option<PageType>) -> ViewId {
        let id = rand::thread_rng().gen();

        let view = self
            .renderer
            .create_view(size.width, size.height, &self.view_config, None)
            .expect("Failed to create view");

        // TODO: debug why new views are slanted unless do + 10/ - 10
        // maybe causes the fuzzyness
        view.resize(size.width + 10, size.height - 10);

        let surface = view.surface().expect("Failed to get surface of new view");
        // RGBA - ensure it has the right diamentions
        debug_assert!(surface.row_bytes() / size.width == 4);

        let cursor = Arc::new(RwLock::new(mouse::Interaction::Idle));
        let cb_cursor = cursor.clone();
        view.set_change_cursor_callback(move |_view, cursor_update| {
            *cb_cursor.write().expect("Failed to write cursor status") = match cursor_update {
                Cursor::None => mouse::Interaction::Idle,
                Cursor::Pointer => mouse::Interaction::Idle,
                Cursor::Hand => mouse::Interaction::Pointer,
                Cursor::Grab => mouse::Interaction::Grab,
                Cursor::VerticalText => mouse::Interaction::Text,
                Cursor::IBeam => mouse::Interaction::Text,
                Cursor::Cross => mouse::Interaction::Crosshair,
                Cursor::Wait => mouse::Interaction::Working,
                Cursor::Grabbing => mouse::Interaction::Grab,
                Cursor::NorthSouthResize => mouse::Interaction::ResizingVertically,
                Cursor::EastWestResize => mouse::Interaction::ResizingHorizontally,
                Cursor::NotAllowed => mouse::Interaction::NotAllowed,
                Cursor::ZoomIn => mouse::Interaction::ZoomIn,
                Cursor::ZoomOut => mouse::Interaction::ZoomIn,
                _ => mouse::Interaction::Pointer,
            };
        });

        let view = View {
            id,
            view,
            cursor,
            last_frame: ImageInfo::blank(size.width, size.height),
            was_loading: true,
            cursor_pos: Point::default(),
        };
        if let Some(page_type) = page_type {
            match page_type {
                PageType::Url(url) => view.view.load_url(&url).expect("Failed to load url"),
                PageType::Html(html) => view
                    .view
                    .load_html(&html)
                    .expect("Failed to load custom html"),
            }
            view.view.set_needs_paint(true);
        }
        self.views.push(view);
        id
    }

    fn remove_view(&mut self, id: ViewId) {
        self.views.retain(|view| view.id != id);
    }

    fn goto(&mut self, id: ViewId, page_type: PageType) {
        *self
            .get_view(id)
            .cursor
            .write()
            .expect("Failed cursor poisoned") = mouse::Interaction::Working;
        match page_type {
            PageType::Url(url) => self
                .get_view_mut(id)
                .view
                .load_url(&url)
                .expect("Failed to load url"),
            PageType::Html(html) => self
                .get_view_mut(id)
                .view
                .load_html(&html)
                .expect("Failed to load given html"),
        }
        self.get_view_mut(id).was_loading = true;
    }

    fn focus(&mut self) {
        self.views.iter().for_each(|view| view.view.focus());
    }

    fn unfocus(&self) {
        self.views.iter().for_each(|view| view.view.unfocus());
    }

    fn resize(&mut self, size: Size<u32>) {
        self.views.iter().for_each(|view| {
            view.view.resize(size.width, size.height);
            view.view.surface().unwrap().resize(size.width, size.height);
            view.view.set_needs_paint(true);
        })
    }

    fn handle_keyboard_event(&mut self, id: ViewId, event: keyboard::Event) {
        let key_event = match event {
            keyboard::Event::KeyPressed {
                key,
                location,
                modifiers,
                text,
                modified_key,
                physical_key: _,
            } => iced_key_to_ultralight_key(
                KeyPress::Press,
                Some(modified_key),
                Some(key),
                Some(location),
                modifiers,
                text,
            ),
            keyboard::Event::KeyReleased {
                key,
                modified_key: _,
                physical_key: _,
                location,
                modifiers,
            } => iced_key_to_ultralight_key(
                KeyPress::Unpress,
                None,
                Some(key),
                Some(location),
                modifiers,
                None,
            ),
            keyboard::Event::ModifiersChanged(modifiers) => {
                iced_key_to_ultralight_key(KeyPress::Press, None, None, None, modifiers, None)
            }
        };

        if let Some(key_event) = key_event {
            self.get_view_mut(id).view.fire_key_event(key_event);
        }
    }

    fn handle_mouse_event(&mut self, id: ViewId, point: Point, event: mouse::Event) {
        match event {
            mouse::Event::ButtonReleased(mouse::Button::Forward) => self.go_forward(id),
            mouse::Event::ButtonReleased(mouse::Button::Back) => self.go_back(id),
            mouse::Event::ButtonPressed(mouse::Button::Left) => {
                self.get_view_mut(id).view.fire_mouse_event(
                    MouseEvent::new(
                        ul_next::event::MouseEventType::MouseDown,
                        point.x as i32,
                        point.y as i32,
                        ul_next::event::MouseButton::Left,
                    )
                    .expect("Ultralight failed to fire mouse input"),
                );
            }
            mouse::Event::ButtonReleased(mouse::Button::Left) => {
                self.get_view_mut(id).view.fire_mouse_event(
                    MouseEvent::new(
                        ul_next::event::MouseEventType::MouseUp,
                        point.x as i32,
                        point.y as i32,
                        ul_next::event::MouseButton::Left,
                    )
                    .expect("Ultralight failed to fire mouse input"),
                );
            }
            mouse::Event::ButtonPressed(mouse::Button::Right) => {
                self.get_view_mut(id).view.fire_mouse_event(
                    MouseEvent::new(
                        ul_next::event::MouseEventType::MouseDown,
                        point.x as i32,
                        point.y as i32,
                        ul_next::event::MouseButton::Right,
                    )
                    .expect("Ultralight failed to fire mouse input"),
                );
            }
            mouse::Event::ButtonReleased(mouse::Button::Right) => {
                self.get_view_mut(id).view.fire_mouse_event(
                    MouseEvent::new(
                        ul_next::event::MouseEventType::MouseUp,
                        point.x as i32,
                        point.y as i32,
                        ul_next::event::MouseButton::Right,
                    )
                    .expect("Ultralight failed to fire mouse input"),
                );
            }
            mouse::Event::CursorMoved { position: _ } => {
                self.get_view_mut(id).cursor_pos = point;
            }
            mouse::Event::WheelScrolled { delta } => self.scroll(id, delta),
            mouse::Event::CursorLeft => {
                self.unfocus();
            }
            mouse::Event::CursorEntered => {
                self.focus();
            }
            _ => (),
        }
    }

    fn refresh(&mut self, id: ViewId) {
        self.get_view_mut(id).view.reload();
    }

    fn go_forward(&mut self, id: ViewId) {
        self.get_view_mut(id).view.go_forward();
    }

    fn go_back(&mut self, id: ViewId) {
        self.get_view_mut(id).view.go_back();
    }

    fn scroll(&mut self, id: ViewId, delta: mouse::ScrollDelta) {
        let scroll_event = match delta {
            ScrollDelta::Lines { x, y } => ScrollEvent::new(
                ul_next::event::ScrollEventType::ScrollByPixel,
                x as i32 * 100,
                y as i32 * 100,
            )
            .unwrap(),
            ScrollDelta::Pixels { x, y } => ScrollEvent::new(
                ul_next::event::ScrollEventType::ScrollByPixel,
                x as i32,
                y as i32,
            )
            .unwrap(),
        };
        self.get_view_mut(id).view.fire_scroll_event(scroll_event);
    }

    fn get_url(&self, id: ViewId) -> String {
        self.get_view(id).view.url().unwrap_or_default()
    }

    fn get_title(&self, id: ViewId) -> String {
        self.get_view(id).view.title().unwrap_or_default()
    }

    fn get_cursor(&self, id: ViewId) -> mouse::Interaction {
        match self.get_view(id).cursor.read() {
            Ok(cursor) => *cursor,
            Err(_) => mouse::Interaction::Working,
        }
    }

    fn get_view(&self, id: ViewId) -> &ImageInfo {
        &self.get_view(id).last_frame
    }
}

fn platform_filesystem() -> PathBuf {
    let env = var("ULTRALIGHT_RESOURCES_DIR");
    let resources_path: PathBuf = match env {
        Ok(env) => PathBuf::from_str(&env)
            .expect("Failed to get path from ultralight resources enviroment varible"),
        Err(_) => {
            // env not set - check if its been symlinked by build.rs
            match Path::new("./resources").exists() {
                    true => Path::new("./resources").to_owned(),
                    false => panic!("ULTRALIGHT_RESOURCES_DIR was not set and ultralight-resources feature was not enabled"),
                }
        }
    };
    assert!(Path::new(&resources_path).join("cacert.pem").exists());
    assert!(Path::new(&resources_path).join("icudt67l.dat").exists());
    resources_path
        .parent() // leaves resources directory
        .expect("resources path needs to point to the resources directory")
        .into()
}

#[derive(Debug, PartialEq, Eq)]
enum KeyPress {
    Press,
    Unpress,
}

fn iced_key_to_ultralight_key(
    press: KeyPress,
    modified_key: Option<keyboard::Key>,
    key: Option<keyboard::Key>, // This one is modified by ctrl and results in wrong key
    _location: Option<keyboard::Location>,
    modifiers: keyboard::Modifiers,
    text: Option<SmolStr>,
) -> Option<event::KeyEvent> {
    let (text, virtual_key, native_key) = {
        if let Some(key) = key {
            let text = match key {
                keyboard::Key::Named(key) => {
                    if key == keyboard::key::Named::Space {
                        String::from(" ")
                    } else {
                        String::from("")
                    }
                }
                keyboard::Key::Character(_) => match text {
                    Some(text) => text.to_string(),
                    None => String::from(""),
                },
                keyboard::Key::Unidentified => return None,
            };
            let (virtual_key, native_key) = match key {
                keyboard::Key::Named(key) => match key {
                    keyboard::key::Named::Control => (VirtualKeyCode::Control, 29),
                    keyboard::key::Named::Shift => (VirtualKeyCode::Shift, 42),
                    keyboard::key::Named::Enter => (VirtualKeyCode::Return, 28),
                    keyboard::key::Named::Tab => (VirtualKeyCode::Tab, 15),
                    keyboard::key::Named::Space => (VirtualKeyCode::Space, 57),
                    keyboard::key::Named::ArrowDown => (VirtualKeyCode::Down, 108),
                    keyboard::key::Named::ArrowLeft => (VirtualKeyCode::Right, 106),
                    keyboard::key::Named::ArrowRight => (VirtualKeyCode::Up, 103),
                    keyboard::key::Named::ArrowUp => (VirtualKeyCode::Left, 105),
                    keyboard::key::Named::End => (VirtualKeyCode::End, 107),
                    keyboard::key::Named::Home => (VirtualKeyCode::Home, 102),
                    keyboard::key::Named::Backspace => (VirtualKeyCode::Back, 14),
                    keyboard::key::Named::Delete => (VirtualKeyCode::Delete, 11),
                    keyboard::key::Named::Insert => (VirtualKeyCode::Insert, 110),
                    keyboard::key::Named::Escape => (VirtualKeyCode::Escape, 1),
                    keyboard::key::Named::F1 => (VirtualKeyCode::F1, 59),
                    keyboard::key::Named::F2 => (VirtualKeyCode::F2, 60),
                    keyboard::key::Named::F3 => (VirtualKeyCode::F3, 61),
                    keyboard::key::Named::F4 => (VirtualKeyCode::F4, 62),
                    keyboard::key::Named::F5 => (VirtualKeyCode::F5, 63),
                    keyboard::key::Named::F6 => (VirtualKeyCode::F6, 64),
                    keyboard::key::Named::F7 => (VirtualKeyCode::F7, 65),
                    keyboard::key::Named::F8 => (VirtualKeyCode::F8, 66),
                    keyboard::key::Named::F9 => (VirtualKeyCode::F9, 67),
                    keyboard::key::Named::F10 => (VirtualKeyCode::F10, 68),
                    keyboard::key::Named::F11 => (VirtualKeyCode::F11, 69),
                    keyboard::key::Named::F12 => (VirtualKeyCode::F12, 70),
                    _ => return None,
                },
                keyboard::Key::Character(key) => match key.as_str() {
                    "a" => (VirtualKeyCode::A, 30),
                    "b" => (VirtualKeyCode::B, 48),
                    "c" => (VirtualKeyCode::C, 46),
                    "d" => (VirtualKeyCode::D, 32),
                    "e" => (VirtualKeyCode::E, 18),
                    "f" => (VirtualKeyCode::F, 33),
                    "g" => (VirtualKeyCode::G, 34),
                    "h" => (VirtualKeyCode::H, 35),
                    "i" => (VirtualKeyCode::I, 23),
                    "j" => (VirtualKeyCode::J, 36),
                    "k" => (VirtualKeyCode::K, 37),
                    "l" => (VirtualKeyCode::L, 38),
                    "m" => (VirtualKeyCode::M, 50),
                    "n" => (VirtualKeyCode::N, 49),
                    "o" => (VirtualKeyCode::O, 24),
                    "p" => (VirtualKeyCode::P, 25),
                    "q" => (VirtualKeyCode::Q, 16),
                    "r" => (VirtualKeyCode::R, 19),
                    "s" => (VirtualKeyCode::S, 31),
                    "t" => (VirtualKeyCode::T, 20),
                    "u" => (VirtualKeyCode::U, 22),
                    "v" => (VirtualKeyCode::V, 47),
                    "w" => (VirtualKeyCode::W, 17),
                    "x" => (VirtualKeyCode::X, 47),
                    "y" => (VirtualKeyCode::Y, 21),
                    "z" => (VirtualKeyCode::Z, 44),
                    "0" => (VirtualKeyCode::Key0, 11),
                    "1" => (VirtualKeyCode::Key1, 2),
                    "2" => (VirtualKeyCode::Key2, 3),
                    "3" => (VirtualKeyCode::Key3, 4),
                    "4" => (VirtualKeyCode::Key4, 5),
                    "5" => (VirtualKeyCode::Key5, 6),
                    "6" => (VirtualKeyCode::Key6, 7),
                    "7" => (VirtualKeyCode::Key7, 8),
                    "8" => (VirtualKeyCode::Key8, 9),
                    "9" => (VirtualKeyCode::Key9, 10),
                    "," => (VirtualKeyCode::OemComma, 51),
                    "." => (VirtualKeyCode::OemPeriod, 52),
                    ";" => (VirtualKeyCode::OemPeriod, 39),
                    "-" => (VirtualKeyCode::OemMinus, 12),
                    "_" => (VirtualKeyCode::OemMinus, 74),
                    "+" => (VirtualKeyCode::OemPlus, 78),
                    "=" => (VirtualKeyCode::OemPlus, 78),
                    "\\" => (VirtualKeyCode::Oem5, 43),
                    "|" => (VirtualKeyCode::Oem5, 43),
                    "`" => (VirtualKeyCode::Oem3, 41),
                    "?" => (VirtualKeyCode::Oem2, 53),
                    "/" => (VirtualKeyCode::Oem2, 53),
                    ">" => (VirtualKeyCode::Oem102, 52),
                    "<" => (VirtualKeyCode::Oem102, 52),
                    "[" => (VirtualKeyCode::Oem4, 26),
                    "]" => (VirtualKeyCode::Oem6, 27),
                    _ => return None,
                },
                keyboard::Key::Unidentified => return None,
            };
            (text, virtual_key, native_key)
        } else {
            return None;
        }
    };

    let modifiers = event::KeyEventModifiers {
        alt: modifiers.alt(),
        ctrl: modifiers.control(),
        meta: modifiers.logo(),
        shift: modifiers.shift(),
    };

    let ty = if modifiers.ctrl {
        event::KeyEventType::RawKeyDown
    } else if !text.is_empty() && text.is_ascii() && press == KeyPress::Press {
        event::KeyEventType::Char
    } else {
        match press {
            KeyPress::Press => event::KeyEventType::RawKeyDown,
            KeyPress::Unpress => event::KeyEventType::KeyUp,
        }
    };

    let creation_info = KeyEventCreationInfo {
        ty,
        modifiers,
        virtual_key_code: virtual_key,
        native_key_code: native_key,
        text: text.as_str(),
        unmodified_text: if let Some(keyboard::Key::Character(char)) = modified_key {
            &char.to_string()
        } else {
            text.as_str()
        },
        is_keypad: false,
        is_auto_repeat: false,
        is_system_key: false,
    };

    event::KeyEvent::new(creation_info).ok()
}
