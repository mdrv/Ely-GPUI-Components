//! MDRV GPUI + Ely examples — an Android showcase of as many Ely GPUI
//! components as practical, running on the mdrv-gpui-ce fork.
//!
//! Package id on Android: `id.mdrv.ely.examples` ("MDRV Ely Examples") so it
//! installs alongside `id.mdrv.mobile.example` and the upstream
//! `dev.gpui.mobile.example`.

pub mod showcase;

use std::path::PathBuf;
use std::sync::OnceLock;

/// App-internal storage (Android: `internal_data_path`, set in android_main;
/// desktop fallback: cwd).
static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn data_dir() -> PathBuf {
    DATA_DIR
        .get()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."))
}

pub use showcase::Showcase;

#[cfg(target_os = "android")]
mod android_entry {
    use gpui::{size, AppContext, WindowBounds, WindowOptions};

    use crate::showcase::Showcase;

    pub fn start(cx: &mut gpui::App, log: &Option<std::path::PathBuf>) {
        // Receive the Android BACK key as gpui "escape" for the in-app
        // Back action (cancels the dialog; keeps the app alive).
        gpui_mobile::android::jni::set_back_sends_escape(true);
        cx.bind_keys([gpui::KeyBinding::new("escape", crate::showcase::Back, None)]);

        crate::android_log(log, "run closure: init ely theme");
        ely_gpui_component::init(cx);

        crate::android_log(log, "run closure: opening window");
        let bounds = gpui::Bounds {
            origin: gpui::Point::default(),
            size: size(gpui::px(1080.), gpui::px(1920.)),
        };
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                focus: true,
                show: true,
                kind: gpui::WindowKind::Normal,
                ..Default::default()
            },
            |_, cx| cx.new(|cx| Showcase::new(cx)),
        )
        .expect("open root window");
        crate::android_log(log, "run closure: window opened");
    }

    #[no_mangle]
    fn android_main(app: android_activity::AndroidApp) {
        if let Some(dp) = app.internal_data_path() {
            let _ = crate::DATA_DIR.set(std::path::PathBuf::from(dp));
        }
        let log_path = app
            .internal_data_path()
            .map(std::path::PathBuf::from)
            .map(|p| p.join("ely-examples.log"));

        if let Some(path) = &log_path {
            let _ = std::fs::write(path, b"");
        }
        crate::android_log(&log_path, "android_main entered");
        let panic_log = log_path.clone();
        std::panic::set_hook(Box::new(move |info| {
            crate::android_log(&panic_log, &format!("PANIC: {info}"));
        }));

        if let Some(path) = &log_path {
            gpui_mobile::android::set_log_file(path.clone());
        }
        gpui_mobile::android::init_logger();

        use gpui_mobile::android::jni as platform_jni;
        platform_jni::init_platform(&app);
        crate::android_log(&log_path, "init_platform done");
        let shared = platform_jni::shared_platform();
        crate::android_log(
            &log_path,
            &format!("shared_platform() -> {}", shared.is_some()),
        );
        let shared = shared.unwrap_or_else(|| panic!("ely-examples: platform not initialised"));

        crate::android_log(&log_path, "starting Application::with_platform");
        let run_log = log_path.clone();
        gpui::Application::with_platform(shared.into_rc()).run(move |cx| {
            start(cx, &run_log);
        });
        crate::android_log(&log_path, "Application.run returned");
        // The NativeActivity glue leaves the process parked after
        // android_main returns; exit explicitly once the loop is done.
        std::process::exit(0);
    }
}

/// Append a timestamped line to the app file log (logcat is unavailable on
/// the test ROM).
#[cfg(target_os = "android")]
pub fn android_log(path: &Option<std::path::PathBuf>, msg: &str) {
    use std::fmt::Write as _;
    let Some(path) = path else { return };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let mut line = String::new();
    let _ = write!(line, "[{stamp}] {msg}\n");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
}
