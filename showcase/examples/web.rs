#![cfg_attr(target_family = "wasm", no_main)]

//! The showcase in a browser: the same `Showcase` root as Android, one
//! window on a canvas (`gpui_web` — wgpu over WebGPU/WebGL). Build with
//! `scripts/web.sh` in the repo root; serve the `web/` dir statically.

use std::{rc::Rc, sync::Arc};

use ely_examples::showcase::Back;
use ely_examples::Showcase;
use ely_gpui_component::Assets;
use gpui::{
    px, size, App, AppContext, Bounds, KeyBinding, Point, WindowBounds, WindowKind, WindowOptions,
};
use gpui_web::{WebBackendPreference, WebPlatform};
use wasm_bindgen::prelude::*;

fn start(cx: &mut App) {
    ely_gpui_component::init(cx);
    // Escape triggers the in-app Back action (cancels the dialog) on web too.
    cx.bind_keys([KeyBinding::new("escape", Back, None)]);
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
        |_, cx| cx.new(|cx| Showcase::new(cx)),
    )
    .expect("open root window");
}

/// The showcase in a browser: one window on a canvas.
#[wasm_bindgen(start)]
pub fn web_main() {
    gpui_platform::web_init();
    // One thread: a static site gets no shared memory. Force WebGL2: the
    // WebGPU probe hangs/crashes some environments (headless Chrome), and
    // WebGL2 is the universal browser backend anyway.
    let platform = Rc::new(WebPlatform::new_with_backend(
        false,
        WebBackendPreference::WebGl,
    ));
    let http = Arc::new(platform.fetch_http_client());
    // The web platform's run loop returns immediately, so the app state is
    // only kept alive by the returned handle: Application::run would drop the
    // app as soon as web_main returns ("app was released", no frames drawn).
    let app = gpui::Application::with_platform(platform)
        .with_http_client(http)
        .with_assets(Assets);
    let handle = app.run_embedded(start);
    std::mem::forget(handle);
}
