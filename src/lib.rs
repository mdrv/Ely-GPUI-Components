mod assets;

mod canvas;
pub mod buttons;
pub mod calendar;
pub mod data_display;
pub mod feedback;

pub mod forms;
pub mod i18n;
pub mod interaction;
pub mod layout;
pub mod lists;
pub mod menus;
pub mod misc;
pub mod motion;
pub mod navigation;
pub mod onboarding;
pub mod overlays;
pub mod primitives;
pub mod settings;
pub mod theme;
pub mod typography;

pub use assets::Assets;

use gpui::{App, KeyBinding};

use crate::primitives::{FocusNext, FocusPrev, IconName};

/// Loads fonts and the theme, binds Tab and text keys. Call once, first.
pub fn init(cx: &mut App) {
    // (mdrv port) the fork's `App` has no `asset_source()` probe; a missing
    // asset source surfaces when `load_fonts` below fails to register.
    assets::load_fonts(cx).expect("ely: embedded fonts failed to register");
    theme::Theme::init(cx);
    cx.bind_keys([
        KeyBinding::new("tab", FocusNext, None),
        KeyBinding::new("shift-tab", FocusPrev, None),
    ]);
    forms::bind_keys(cx);
}
