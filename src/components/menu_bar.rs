//! In-app menu bar for Windows and Linux
//!
//! GPUI 0.2.2 doesn't fully support native menus on Windows/Linux,
//! so this component provides an in-app menu bar as a workaround.
//!
//! On macOS, native menus are used instead (via cx.set_menus()).

use crate::utils::style::{Colors, Spacing, format_shortcut};
use gpui::prelude::*;
use gpui::*;

/// Menu item definition
pub struct MenuItemDef {
    pub label: String,
    pub shortcut: Option<String>,
    pub action: Option<Box<dyn Action>>,
    pub is_separator: bool,
}

impl Clone for MenuItemDef {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            shortcut: self.shortcut.clone(),
            action: self.action.as_ref().map(|a| a.boxed_clone()),
            is_separator: self.is_separator,
        }
    }
}

impl MenuItemDef {
    pub fn action(label: &str, shortcut: Option<&str>, action: impl Action) -> Self {
        Self {
            label: label.to_string(),
            shortcut: shortcut.map(|s| s.to_string()),
            action: Some(Box::new(action)),
            is_separator: false,
        }
    }

    pub fn separator() -> Self {
        Self {
            label: String::new(),
            shortcut: None,
            action: None,
            is_separator: true,
        }
    }
}

/// Menu definition (a dropdown menu with items)
#[derive(Clone)]
pub struct MenuDef {
    pub name: String,
    pub items: Vec<MenuItemDef>,
}

/// In-app menu bar component
pub struct MenuBar {
    /// Currently open menu index (None if no menu is open)
    open_menu: Option<usize>,
    /// Menu definitions
    menus: Vec<MenuDef>,
    /// Focus handle for keyboard navigation
    focus_handle: FocusHandle,
}

impl MenuBar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            open_menu: None,
            menus: Self::create_menu_definitions(),
            focus_handle: cx.focus_handle(),
        }
    }

    fn create_menu_definitions() -> Vec<MenuDef> {
        use crate::{
            CloseWindow, DisableFilters, EnableFilters, NextFrame, NextImage, OpenFile,
            OpenInExternalEditor, OpenInExternalViewer, OpenInExternalViewerAndQuit, PreviousFrame,
            PreviousImage, Quit, ResetFilters, RevealInFinder, SaveFile, SaveFileToDownloads,
            SortAlphabetical, SortByModified, ToggleAnimationPlayPause, ToggleDebug, ToggleFilters,
            ToggleHelp, ToggleSettings, ZoomIn, ZoomOut, ZoomReset,
        };

        vec![
            MenuDef {
                name: "File".to_string(),
                items: vec![
                    MenuItemDef::action(
                        "Open File...",
                        Some(&format_shortcut("O", false, false)),
                        OpenFile,
                    ),
                    MenuItemDef::action(
                        "Save File...",
                        Some(&format_shortcut("S", false, false)),
                        SaveFile,
                    ),
                    MenuItemDef::action(
                        "Save to Downloads...",
                        Some(&format_shortcut("S", false, true)),
                        SaveFileToDownloads,
                    ),
                    MenuItemDef::separator(),
                    MenuItemDef::action(
                        "Reveal in Explorer",
                        Some(&format_shortcut("R", false, false)),
                        RevealInFinder,
                    ),
                    MenuItemDef::action(
                        "Open in External Viewer",
                        Some(&format_shortcut("V", false, true)),
                        OpenInExternalViewer,
                    ),
                    MenuItemDef::action(
                        "Open in Viewer and Quit",
                        Some(&format_shortcut("V", true, true)),
                        OpenInExternalViewerAndQuit,
                    ),
                    MenuItemDef::action(
                        "Open in External Editor",
                        Some(&format_shortcut("E", false, false)),
                        OpenInExternalEditor,
                    ),
                    MenuItemDef::separator(),
                    MenuItemDef::action(
                        "Close Window",
                        Some(&format_shortcut("W", false, false)),
                        CloseWindow,
                    ),
                    MenuItemDef::action("Quit", Some(&format_shortcut("Q", false, false)), Quit),
                ],
            },
            MenuDef {
                name: "Edit".to_string(),
                items: vec![MenuItemDef::action(
                    "Settings...",
                    Some(&format_shortcut(",", false, false)),
                    ToggleSettings,
                )],
            },
            MenuDef {
                name: "View".to_string(),
                items: vec![
                    MenuItemDef::action("Zoom In", Some("+"), ZoomIn),
                    MenuItemDef::action("Zoom Out", Some("-"), ZoomOut),
                    MenuItemDef::action("Reset Zoom", Some("0"), ZoomReset),
                    MenuItemDef::separator(),
                    MenuItemDef::action(
                        "Toggle Filters",
                        Some(&format_shortcut("F", false, false)),
                        ToggleFilters,
                    ),
                    MenuItemDef::action(
                        "Disable Filters",
                        Some(&format_shortcut("1", false, false)),
                        DisableFilters,
                    ),
                    MenuItemDef::action(
                        "Enable Filters",
                        Some(&format_shortcut("2", false, false)),
                        EnableFilters,
                    ),
                    MenuItemDef::action(
                        "Reset Filters",
                        Some(&format_shortcut("R", true, false)),
                        ResetFilters,
                    ),
                    MenuItemDef::separator(),
                    MenuItemDef::action("Toggle Help", Some("H"), ToggleHelp),
                    MenuItemDef::action("Toggle Debug", Some("F12"), ToggleDebug),
                ],
            },
            MenuDef {
                name: "Navigate".to_string(),
                items: vec![
                    MenuItemDef::action("Next Image", Some("→"), NextImage),
                    MenuItemDef::action("Previous Image", Some("←"), PreviousImage),
                    MenuItemDef::separator(),
                    MenuItemDef::action(
                        "Sort Alphabetically",
                        Some(&format_shortcut("A", true, false)),
                        SortAlphabetical,
                    ),
                    MenuItemDef::action(
                        "Sort by Modified Date",
                        Some(&format_shortcut("M", true, false)),
                        SortByModified,
                    ),
                ],
            },
            MenuDef {
                name: "Animation".to_string(),
                items: vec![
                    MenuItemDef::action("Play/Pause", Some("O"), ToggleAnimationPlayPause),
                    MenuItemDef::action("Next Frame", Some("]"), NextFrame),
                    MenuItemDef::action("Previous Frame", Some("["), PreviousFrame),
                ],
            },
        ]
    }

    /// Close the currently open menu
    pub fn close_menu(&mut self, cx: &mut Context<Self>) {
        if self.open_menu.is_some() {
            self.open_menu = None;
            cx.notify();
        }
    }

    /// Check if any menu is open
    pub fn is_menu_open(&self) -> bool {
        self.open_menu.is_some()
    }

    fn render_menu_button(&self, index: usize, menu: &MenuDef, cx: &mut Context<Self>) -> Div {
        let is_open = self.open_menu == Some(index);
        let menu_name = menu.name.clone();

        div()
            .id(SharedString::from(format!("menu-{}", index)))
            .px(Spacing::sm())
            .py(px(4.0))
            .text_size(px(13.0))
            .text_color(Colors::text())
            .cursor_pointer()
            .when(is_open, |el| el.bg(rgb(0x3d3d3d)))
            .hover(|el| el.bg(rgb(0x3d3d3d)))
            .on_click(cx.listener(move |this, _event, _window, cx| {
                if this.open_menu == Some(index) {
                    this.open_menu = None;
                } else {
                    this.open_menu = Some(index);
                }
                cx.notify();
            }))
            .on_mouse_move(
                cx.listener(move |this, _event: &MouseMoveEvent, _window, cx| {
                    // If a menu is already open, switch to this menu on hover
                    if this.open_menu.is_some() && this.open_menu != Some(index) {
                        this.open_menu = Some(index);
                        cx.notify();
                    }
                }),
            )
            .child(menu_name)
    }

    fn render_dropdown(&self, menu: &MenuDef, cx: &mut Context<Self>) -> Div {
        let items: Vec<_> = menu
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| self.render_menu_item(i, item, cx))
            .collect();

        div()
            .absolute()
            .top(px(24.0))
            .left_0()
            .min_w(px(220.0))
            .bg(rgb(0x2d2d2d))
            .border_1()
            .border_color(rgb(0x3d3d3d))
            .rounded_b(px(4.0))
            .shadow_lg()
            .py(px(4.0))
            .children(items)
    }

    fn render_menu_item(&self, index: usize, item: &MenuItemDef, cx: &mut Context<Self>) -> Div {
        if item.is_separator {
            return div()
                .h(px(1.0))
                .mx(Spacing::sm())
                .my(px(4.0))
                .bg(rgb(0x444444));
        }

        let action = item.action.as_ref().map(|a| a.boxed_clone());
        let label = item.label.clone();
        let shortcut = item.shortcut.clone();

        div()
            .id(SharedString::from(format!("menu-item-{}", index)))
            .flex()
            .flex_row()
            .justify_between()
            .items_center()
            .px(Spacing::md())
            .py(px(6.0))
            .text_size(px(13.0))
            .text_color(Colors::text())
            .cursor_pointer()
            .hover(|el| el.bg(rgb(0x3d3d3d)))
            .when_some(action, |el, action| {
                el.on_click(cx.listener(move |this, _event, window, cx| {
                    // Close the menu
                    this.open_menu = None;
                    cx.notify();
                    // Dispatch the action
                    window.dispatch_action(action.boxed_clone(), cx);
                }))
            })
            .child(div().flex_1().child(label))
            .when_some(shortcut, |el, shortcut| {
                el.child(
                    div()
                        .ml(Spacing::lg())
                        .text_size(px(11.0))
                        .text_color(rgb(0x888888))
                        .child(shortcut),
                )
            })
    }
}

impl Render for MenuBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menus: Vec<_> = self
            .menus
            .iter()
            .enumerate()
            .map(|(i, menu)| {
                let is_open = self.open_menu == Some(i);
                div()
                    .relative()
                    .child(self.render_menu_button(i, menu, cx))
                    .when(is_open, |el| el.child(self.render_dropdown(menu, cx)))
            })
            .collect();

        div()
            .id("menu-bar")
            .track_focus(&self.focus_handle)
            .w_full()
            .h(px(28.0))
            .flex()
            .flex_row()
            .items_center()
            .bg(rgb(0x252525))
            .border_b_1()
            .border_color(rgb(0x3d3d3d))
            .children(menus)
            // Click outside dropdown to close
            .when(self.open_menu.is_some(), |el| {
                el.on_mouse_down_out(cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                    this.open_menu = None;
                    cx.notify();
                }))
            })
    }
}
