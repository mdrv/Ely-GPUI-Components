#![cfg_attr(target_family = "wasm", no_main)]

//! Minimal web boot: same platform + wgpu + gpui_web stack, an empty root
//! view. Used to bisect web panics — if this renders, the stack is fine and
//! the problem lives in the showcase/ely content.

use std::{rc::Rc, sync::Arc};

use gpui::{
    div, px, size, App, AppContext, Bounds, IntoElement, Point, Render, Styled, WindowBounds,
    WindowKind, WindowOptions,
};
use gpui_web::{WebBackendPreference, WebPlatform};
use wasm_bindgen::prelude::*;

struct MinView;

impl gpui::Render for MinView {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<'_, Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(gpui::rgb(0x202020))
            .text_color(gpui::rgb(0xf0f0f0))
    }
}

fn start(cx: &mut App) {
    let bounds = Bounds {
        origin: Point::default(),
        size: size(px(1080.), px(1920.)),
    };
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            ..Default::default()
        },
        |_, cx| cx.new(|_| MinView),
    )
    .expect("open min window");
}

#[wasm_bindgen(start)]
pub fn web_min_main() {
    gpui_platform::web_init();
    let platform = Rc::new(WebPlatform::new_with_backend(
        false,
        WebBackendPreference::WebGl,
    ));
    let http = Arc::new(platform.fetch_http_client());
    let app = gpui::Application::with_platform(platform)
        .with_http_client(http)
        .run_embedded(start);
    std::mem::forget(app);
}
