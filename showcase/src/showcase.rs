//! The component showcase: Ely GPUI components running on the mdrv-gpui-ce
//! fork, Android touch first.
//!
//! Layout: a header, a wrapped row of section chips, then ONE component family
//! at a time (buttons, forms, data display, lists, feedback, overlays,
//! calendar, typography, motion, kids, layout/primitives, settings) — the
//! active one is chosen by `Showcase.section` and only its Div is built each
//! frame, which keeps a phone smooth. Simple widget state lives on `Showcase`
//! fields, and pieces that need a `&mut Window` at creation time (the
//! `TextInput`s) are kept with `window.use_keyed_state` inside the section
//! methods that use them — keyed state persists across frames, the same idiom
//! Ely's own gallery uses, because `Showcase::new` runs before the window
//! hands us a `Window`.
//!
//! All colors come from `cx.theme()` (`ely_gpui_component::theme`), which also
//! keeps us clear of the fork's palette-crate `Hsla` quirks (no `.r/.g/.b`, no
//! inherent `.opacity()`).

use ely_gpui_component::{
    buttons::{
        Button, ButtonGroup, ButtonVariant, FloatingActionButton, IconButton, SegmentedControl,
    },
    calendar::{CalendarMonthView, Event},
    data_display::{
        Avatar, AvatarGroup, Badge, CountBadge, DotBadge, Presence, Stars, Tag, Tone, UserChip,
    },
    feedback::{
        Alert, Banner, Callout, ConnectionStatus, EmptyState, InlineMessage, SaveState,
        SavingIndicator, SyncState, SyncStatus,
    },
    forms::{
        Calendar, CheckState, Checkbox, CheckboxGroup, Choice, ChoiceChips, ColorSwatch, Input,
        PasswordInput, PinInput, RadioGroup, Rating, SearchInput, Slider, Stepper, Switch,
        TagInput, TextInput,
    },
    layout::{Accordion, AccordionItem, Card, CardHeader, Well},
    lists::{List, ListItem, SelectableList},
    misc::{Flashcards, Quiz, QuizQuestion, Stopwatch},
    motion::{
        ProgressBar, ProgressRing, Pulse, SkeletonAvatar, SkeletonCard, SkeletonText, Spinner,
    },
    overlays::{Dialog, Popover},
    primitives::{Divider, Icon, IconName, Severity},
    settings::{SettingsRow, SettingsSection},
    theme::{ActiveTheme, AvatarSize, ControlSize, Mode, Radius, TextSize, Theme},
    typography::{
        AnimatedNumber, Blockquote, Caption, Code, Heading, Highlight, Kbd, KbdCombo, Label,
        Overline, Paragraph, Subtitle, Title,
    },
};
use gpui::{
    actions, div, prelude::*, px, rems, App, Context, Div, Entity, FocusHandle, FontWeight,
    IntoElement, ParentElement, Render, SharedString, Styled, Window,
};

/// The in-app BACK action: the fork translates the Android BACK key into the
/// `escape` keystroke, a binding maps it here, and the root handles it —
/// canceling the dialog instead of letting the activity finish.
actions!(ely_examples, [Back]);

/// Where the app theme comes from: follow the OS, or force light/dark.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThemePref {
    /// Follow the Android system night-mode setting (the default).
    System,
    Light,
    Dark,
}

/// The root window entity.
pub struct Showcase {
    /// Focus root so key events (back/escape) have a dispatch path.
    pub focus: FocusHandle,

    /// Status-bar inset (logical px), queried once — the header is padded by
    /// it so the title and theme toggle sit below the system bar.
    pub safe_top: f32,

    /// Where the app theme comes from: follow the OS, or force one. Applied
    /// to Ely's global `Theme` on change; default is `System`.
    pub theme_pref: ThemePref,

    /// Keeps the OS-appearance observer alive so `System` follows the OS
    /// theme live (not just at startup).
    pub theme_sub: Option<gpui::Subscription>,

    // --- Header / navigation -------------------------------------------------
    /// The active section: a `Showcase::SECTIONS` id ("buttons", "forms", …).
    section: SharedString,

    // --- Overlays ------------------------------------------------------------
    /// The demo dialog's open state, held here per the showcase contract.
    dialog_open: bool,
    /// BACK with nothing else to cancel asks the reader to confirm exiting.
    quit_confirm: bool,

    // --- Buttons -------------------------------------------------------------
    /// Tap counter the demo buttons drive, so presses visibly do something.
    taps: usize,
    /// Which segment ("day" | "week" | "month").
    segment: SharedString,

    // --- Forms ---------------------------------------------------------------
    wifi: bool,
    /// Renderer render scale (1.0 native, 0.75, 0.5) — Settings row knob.
    render_scale: f32,
    sound: bool,
    /// Notification channels picked in the checkbox group.
    notifs: Vec<SharedString>,
    /// Radio group ("home" | "school").
    plan: SharedString,
    /// Horizontal radio group ("quick" | "deep").
    depth: SharedString,
    slider: f64,
    stepper: f64,
    rating: u8,
    /// Picked chips.
    chips: Vec<SharedString>,
    /// Tags in the tag input.
    tags: Vec<SharedString>,
    /// The last completed PIN code, shown back for proof of interaction.
    pin: Option<SharedString>,
    /// The last search-history pick.
    searched: Option<SharedString>,

    // --- Lists ---------------------------------------------------------------
    /// Keys currently selected in the `SelectableList`.
    list_sel: Vec<SharedString>,
}

impl Showcase {
    /// Every section family as (id, label): the chip row renders one chip per
    /// pair, and `Showcase.section` holds the active id.
    pub const SECTIONS: &[(&str, &str)] = &[
        ("all", "All"),
        ("buttons", "Buttons"),
        ("forms", "Forms"),
        ("data", "Data"),
        ("lists", "Lists"),
        ("feedback", "Feedback"),
        ("overlays", "Overlays"),
        ("calendar", "Calendar"),
        ("type", "Typography"),
        ("motion", "Motion"),
        ("kids", "Kids"),
        ("layout", "Layout"),
        ("settings", "Settings"),
    ];

    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut out = Self {
            focus: cx.focus_handle(),
            section: "buttons".into(),
            theme_pref: ThemePref::System,
            theme_sub: None,
            safe_top: query_safe_top(),
            dialog_open: false,
            quit_confirm: false,
            taps: 0,
            segment: "week".into(),
            wifi: true,
            render_scale: 1.0,
            sound: false,
            notifs: vec!["push".into()],
            plan: "home".into(),
            depth: "quick".into(),
            slider: 40.,
            stepper: 2.,
            rating: 4,
            chips: vec!["math".into()],
            tags: vec!["school".into(), "reading".into()],
            pin: None,
            searched: None,
            list_sel: vec!["sun".into()],
        };
        // Default theme: follow the OS (light or dark).
        Self::apply_pref(ThemePref::System, cx);
        out
    }

    /// Map the preference to Ely's global theme. `System` resolves through
    /// gpui's window appearance, which the fork feeds from the real Android
    /// night-mode setting (and re-applies on OS theme changes).
    fn apply_pref(pref: ThemePref, cx: &mut App) {
        let mode: Mode = match pref {
            ThemePref::System => cx.window_appearance().into(),
            ThemePref::Light => Mode::Light,
            ThemePref::Dark => Mode::Dark,
        };
        Theme::set_mode_now(mode, cx);
        // Keep the system bars readable: dark icons on light bars and vice
        // versa (the fork's JNI covers status AND navigation bars).
        gpui_mobile::set_system_chrome(&gpui_mobile::SystemChromeStyle {
            status_bar_style: match mode {
                Mode::Light => gpui_mobile::StatusBarContentStyle::Dark,
                Mode::Dark => gpui_mobile::StatusBarContentStyle::Light,
            },
            ..Default::default()
        });
    }

    /// Segment id for the theme selector matching the current preference.
    fn theme_pref_id(&self) -> SharedString {
        match self.theme_pref {
            ThemePref::System => "auto",
            ThemePref::Light => "light",
            ThemePref::Dark => "dark",
        }
        .into()
    }

    /// The wrapped row of section chips: one small button per family. The
    /// active section is filled (`Primary`), the rest are `Ghost`. Wraps
    /// across multiple lines on a narrow phone.
    fn section_nav(&self, cx: &mut Context<Self>) -> Div {
        let show = cx.entity();
        let mut nav = div().flex().flex_wrap().gap_2();
        for (id, label) in Self::SECTIONS {
            let active = self.section.as_ref() == *id;
            let show = show.clone();
            nav = nav.child(
                Button::new(
                    SharedString::from(format!("nav-{id}")) as SharedString,
                    *label,
                )
                .size(ControlSize::Sm)
                .variant(if active {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Ghost
                })
                .on_click(move |_, _, cx| {
                    let id = SharedString::from(*id);
                    edit(&show, cx, |this| this.section = id);
                }),
            );
        }
        nav
    }

    /// All families stacked — the original one-page showcase. Useful for
    /// watching long-run performance: everything builds every frame.
    fn all_sections(&self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(self.sec_buttons(window, cx))
            .child(self.sec_forms(window, cx))
            .child(self.sec_data(window, cx))
            .child(self.sec_lists(window, cx))
            .child(self.sec_feedback(window, cx))
            .child(self.sec_overlays(window, cx))
            .child(self.sec_calendar(window, cx))
            .child(self.sec_typography(window, cx))
            .child(self.sec_motion(window, cx))
            .child(self.sec_kids(window, cx))
            .child(self.sec_layout(window, cx))
            .child(self.sec_settings(window, cx))
    }

    /// Buttons: every variant, size, icon button, group, segmented control and
    /// the FAB.
    fn sec_buttons(&self, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        let show = cx.entity();

        let variants: [(ButtonVariant, &str); 7] = [
            (ButtonVariant::Primary, "Primary"),
            (ButtonVariant::Secondary, "Secondary"),
            (ButtonVariant::Outline, "Outline"),
            (ButtonVariant::Ghost, "Ghost"),
            (ButtonVariant::Subtle, "Subtle"),
            (ButtonVariant::Danger, "Danger"),
            (ButtonVariant::Link, "Link"),
        ];
        let mut variant_row = row();
        for (ix, (variant, label)) in variants.into_iter().enumerate() {
            let show = show.clone();
            variant_row = variant_row.child(
                Button::new(
                    SharedString::from(format!("btn-{ix}")) as SharedString,
                    label,
                )
                .variant(variant)
                .on_click(move |_, _, cx| edit(&show, cx, |this| this.taps += 1)),
            );
        }
        let size_row = row()
            .child({
                let show = show.clone();
                Button::new("btn-sm", "Small")
                    .size(ControlSize::Sm)
                    .on_click(move |_, _, cx| edit(&show, cx, |this| this.taps += 1))
            })
            .child({
                let show = show.clone();
                Button::new("btn-md", "Medium")
                    .on_click(move |_, _, cx| edit(&show, cx, |this| this.taps += 1))
            })
            .child({
                let show = show.clone();
                Button::new("btn-lg", "Large")
                    .size(ControlSize::Lg)
                    .on_click(move |_, _, cx| edit(&show, cx, |this| this.taps += 1))
            })
            .child(Button::new("btn-off", "Disabled").disabled(true));
        let icon_row = row()
            .child({
                let show = show.clone();
                Button::new("btn-add", "Add shelf")
                    .icon(IconName::Plus)
                    .on_click(move |_, _, cx| edit(&show, cx, |this| this.taps += 1))
            })
            .child({
                let show = show.clone();
                Button::new("btn-next", "Next")
                    .trailing_icon(IconName::ChevronRight)
                    .on_click(move |_, _, cx| edit(&show, cx, |this| this.taps += 1))
            })
            .child({
                let show = show.clone();
                IconButton::new("ib-1", IconName::Trash2)
                    .variant(ButtonVariant::Subtle)
                    .on_click(move |_, _, cx| edit(&show, cx, |this| this.taps += 1))
            })
            .child({
                let show = show.clone();
                IconButton::new("ib-2", IconName::Star)
                    .on_click(move |_, _, cx| edit(&show, cx, |this| this.taps += 1))
            });
        let group_row = row()
            .child(
                ButtonGroup::new()
                    .button(Button::new("bg-day", "Day").variant(ButtonVariant::Outline))
                    .button(Button::new("bg-week", "Week").variant(ButtonVariant::Outline))
                    .button(Button::new("bg-month", "Month").variant(ButtonVariant::Outline)),
            )
            .child({
                let show = show.clone();
                FloatingActionButton::new("ely-fab", IconName::Plus)
                    .label("New")
                    .on_click(move |_, _, cx| edit(&show, cx, |this| this.taps += 1))
            });
        let segmented = {
            let show = show.clone();
            SegmentedControl::new("ely-seg", self.segment.clone())
                .segment("day", "Day", None)
                .segment("week", "Week", None)
                .segment("month", "Month", None)
                .on_change(move |value, _, cx| edit(&show, cx, |this| this.segment = value.clone()))
        };
        section(
            "Buttons",
            "Every variant, size, icon button, group, segmented control and the FAB. Taps count below.",
            cx,
            card(cx)
                .child(variant_row)
                .child(size_row)
                .child(icon_row)
                .child(group_row)
                .child(
                    row()
                        .child(segmented)
                        .child(Caption::new(format!(
                            "Taps: {} · segment: {}",
                            self.taps, self.segment
                        ))),
                ),
        )
    }

    /// Inputs & forms: text, password, search, switches, checkboxes, radios,
    /// slider, stepper, rating, chips, tags, PIN.
    fn sec_forms(&self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let show = cx.entity();

        // Text inputs need a `&mut Window` at creation, so they are kept in the
        // window's keyed state (observed + notifying automatically), the same
        // way Ely's gallery holds them. Keyed state persists across frames, so
        // text typed before a section switch is still here after.
        let name = window.use_keyed_state("ely-name", cx, |window, cx| {
            TextInput::new(window, cx).placeholder("Type a name")
        });
        let notes = window.use_keyed_state("ely-notes", cx, |window, cx| {
            TextInput::new(window, cx)
                .multi_line(2, 5)
                .placeholder("Two to five lines of notes")
        });
        let secret = window.use_keyed_state("ely-secret", cx, |window, cx| {
            TextInput::new(window, cx).masked()
        });
        let query = window.use_keyed_state("ely-query", cx, |window, cx| {
            TextInput::new(window, cx).placeholder("Search the shelf")
        });

        let text_inputs = col()
            .child(specimen(
                "TextInput — the greeting tracks what you type",
                Input::new(&name).clearable(),
                cx,
            ))
            .child(Caption::new(format!(
                "Hello, {}!",
                if name.read(cx).text().is_empty() {
                    "friend"
                } else {
                    name.read(cx).text()
                }
            )))
            .child(specimen("Multi-line TextInput", Input::new(&notes), cx))
            .child(specimen(
                "PasswordInput (masked TextInput)",
                PasswordInput::new(&secret),
                cx,
            ))
            .child({
                let show = show.clone();
                specimen(
                    &format!(
                        "SearchInput with history — picked: {}",
                        self.searched.as_deref().unwrap_or("nothing yet")
                    ),
                    SearchInput::new("ely-search", &query)
                        .history(["dragons", "planets", "recipes"])
                        .on_pick(move |pick, _, cx| {
                            let pick = pick.clone();
                            edit(&show, cx, |this| this.searched = Some(pick));
                        }),
                    cx,
                )
            });
        let toggles = col()
            .child({
                let show = show.clone();
                Switch::new("ely-wifi", self.wifi)
                    .label("Wi-Fi")
                    .on_change(move |on, _, cx| edit(&show, cx, |this| this.wifi = on))
            })
            .child({
                let show = show.clone();
                Switch::new("ely-sound", self.sound)
                    .label("Chime on finish")
                    .on_change(move |on, _, cx| edit(&show, cx, |this| this.sound = on))
            })
            .child(
                Switch::new("ely-locked", true)
                    .label("Locked by a grown-up")
                    .disabled(true),
            )
            .child({
                let check_all = match self.notifs.len() {
                    0 => CheckState::Off,
                    2 => CheckState::On,
                    _ => CheckState::Mixed,
                };
                let show = show.clone();
                Checkbox::new("ely-notifs-all", check_all)
                    .label("All channels")
                    .on_change(move |on, _, cx| {
                        edit(&show, cx, |this| {
                            this.notifs = if on {
                                vec!["email".into(), "push".into()]
                            } else {
                                Vec::new()
                            };
                        })
                    })
            })
            .child({
                let show = show.clone();
                CheckboxGroup::new(
                    "ely-notifs",
                    [Choice::new("email", "Email"), Choice::new("push", "Push")],
                )
                .selected(self.notifs.clone())
                .on_change(move |next, _, cx| {
                    let next = next.to_vec();
                    edit(&show, cx, |this| this.notifs = next)
                })
            });
        let radios = row()
            .child({
                let show = show.clone();
                specimen(
                    "RadioGroup",
                    RadioGroup::new(
                        "ely-plan",
                        [
                            Choice::new("home", "Home"),
                            Choice::new("school", "School"),
                            Choice::new("car", "On the bus"),
                        ],
                    )
                    .selected(self.plan.clone())
                    .on_change(move |value, _, cx| {
                        let value = value.clone();
                        edit(&show, cx, |this| this.plan = value)
                    }),
                    cx,
                )
            })
            .child({
                let show = show.clone();
                specimen(
                    "RadioGroup, horizontal",
                    RadioGroup::new(
                        "ely-plan-h",
                        [Choice::new("quick", "Quick"), Choice::new("deep", "Deep")],
                    )
                    .selected(self.depth.clone())
                    .on_change(move |value, _, cx| {
                        let value = value.clone();
                        edit(&show, cx, |this| this.depth = value)
                    })
                    .horizontal(),
                    cx,
                )
            });
        let numbers = col()
            .child({
                let show = show.clone();
                specimen(
                    &format!("Slider — {}", self.slider as u32),
                    Slider::new("ely-slider", self.slider)
                        .range(0., 100.)
                        .step(1.)
                        .on_change(move |value, _, cx| edit(&show, cx, |this| this.slider = value)),
                    cx,
                )
            })
            .child({
                let show = show.clone();
                specimen(
                    &format!("Stepper — {}", self.stepper),
                    Stepper::new("ely-stepper", self.stepper)
                        .range(0., 10.)
                        .step(1.)
                        .precision(0)
                        .on_change(move |value, _, cx| {
                            edit(&show, cx, |this| this.stepper = value)
                        }),
                    cx,
                )
            })
            .child({
                let show = show.clone();
                specimen(
                    &format!("Rating — {} of 5", self.rating),
                    Rating::new("ely-rating", self.rating)
                        .max(5)
                        .on_change(move |value, _, cx| edit(&show, cx, |this| this.rating = value)),
                    cx,
                )
            });
        let pickers = col()
            .child({
                let show = show.clone();
                specimen(
                    &format!(
                        "ChoiceChips (multi) — {:?}",
                        self.chips.iter().map(|c| c.as_ref()).collect::<Vec<_>>()
                    ),
                    ChoiceChips::new(
                        "ely-chips",
                        [
                            Choice::new("math", "Math"),
                            Choice::new("art", "Art"),
                            Choice::new("code", "Code"),
                        ],
                    )
                    .multiple()
                    .selected(self.chips.clone())
                    .on_change(move |next, _, cx| {
                        let next = next.to_vec();
                        edit(&show, cx, |this| this.chips = next)
                    }),
                    cx,
                )
            })
            .child({
                let show = show.clone();
                specimen(
                    &format!(
                        "TagInput — {:?}",
                        self.tags.iter().map(|t| t.as_ref()).collect::<Vec<_>>()
                    ),
                    TagInput::new("ely-tags", self.tags.clone())
                        .placeholder("Add a tag")
                        .on_change(move |tags, _, cx| edit(&show, cx, |this| this.tags = tags)),
                    cx,
                )
            })
            .child({
                let show = show.clone();
                specimen(
                    &format!(
                        "PinInput — {}",
                        match &self.pin {
                            Some(code) => format!("you typed {code}"),
                            None => "type four digits".to_string(),
                        }
                    ),
                    PinInput::new("ely-pin", 4)
                        .masked()
                        .on_complete(move |code, _, cx| {
                            let code: SharedString = code.to_string().into();
                            edit(&show, cx, |this| this.pin = Some(code));
                        }),
                    cx,
                )
            });
        section(
            "Inputs & forms",
            "Text, password, search, switches, checkboxes, radios, slider, stepper, rating, chips, tags, PIN.",
            cx,
            card(cx)
                .child(text_inputs)
                .child(Divider::horizontal().label("toggles"))
                .child(toggles)
                .child(radios)
                .child(Divider::horizontal().label("numbers"))
                .child(numbers)
                .child(Divider::horizontal().label("picks"))
                .child(pickers),
        )
    }

    /// Data display: avatars, presence, badges, tags, stars, color swatches.
    fn sec_data(&self, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        let palette = cx.theme().colors.clone();

        let avatars = row()
            .child(specimen(
                "Avatar sizes",
                {
                    let mut sizes = row();
                    for (ix, size) in [AvatarSize::Sm, AvatarSize::Md, AvatarSize::Lg]
                        .into_iter()
                        .enumerate()
                    {
                        sizes = sizes.child(
                            Avatar::new(
                                SharedString::from(format!("av-{ix}")) as SharedString,
                                "Ada Lovelace",
                            )
                            .size(size),
                        );
                    }
                    sizes
                },
                cx,
            ))
            .child(specimen(
                "Presence + ring",
                Avatar::new("av-online", "Grace Hopper")
                    .ring()
                    .presence(Presence::Online),
                cx,
            ))
            .child(specimen(
                "AvatarGroup",
                AvatarGroup::new([
                    Avatar::new("ag-1", "Ada Lovelace"),
                    Avatar::new("ag-2", "Grace Hopper"),
                    Avatar::new("ag-3", "Alan Turing"),
                ])
                .max(2),
                cx,
            ))
            .child(specimen(
                "UserChip",
                UserChip::new("uc-1", "Ada Lovelace"),
                cx,
            ));
        let badges = row()
            .child(specimen(
                "Badges by tone",
                {
                    let mut tones = row();
                    for (ix, (tone, name)) in [
                        (Tone::Neutral, "Draft"),
                        (Tone::Accent, "New"),
                        (Tone::Info, "Info"),
                        (Tone::Success, "Done"),
                        (Tone::Warning, "Soon"),
                        (Tone::Danger, "Late"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        tones = tones.child(
                            Badge::new(SharedString::from(format!("b-{ix} {name}"))).tone(tone),
                        );
                    }
                    tones
                },
                cx,
            ))
            .child(specimen(
                "CountBadge + DotBadge over icon buttons",
                row()
                    .child(
                        CountBadge::new("ely-count", 12)
                            .max(99)
                            .over(IconButton::new("ely-count-btn", IconName::Bell)),
                    )
                    .child(
                        DotBadge::new()
                            .tone(Tone::Danger)
                            .over(IconButton::new("ely-dot-btn", IconName::Mail)),
                    ),
                cx,
            ))
            .child(specimen(
                "Tag",
                row()
                    .child(Tag::new("ely-tag-1", "Reading").icon(IconName::BookOpen))
                    .child(Tag::new("ely-tag-2", "Overdue").tone(Tone::Danger)),
                cx,
            ))
            .child(specimen("Stars", Stars::new(4.5).max(5), cx));
        let swatches = row()
            .child(specimen(
                "ColorSwatch — theme colors",
                {
                    let mut swatches = row();
                    for (ix, (name, color)) in [
                        ("accent", palette.accent),
                        ("success", palette.success),
                        ("warning", palette.warning),
                        ("danger", palette.danger),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        swatches = swatches.child(
                            ColorSwatch::new(SharedString::from(format!("sw-{ix}-{name}")), color)
                                .selected(ix == 0),
                        );
                    }
                    swatches
                },
                cx,
            ))
            .child(specimen(
                "ColorSwatch — chart hues",
                {
                    let mut hues = row();
                    for hue in 0..8 {
                        hues = hues.child(ColorSwatch::new(
                            SharedString::from(format!("sw-hue-{hue}")) as SharedString,
                            palette.hue(hue, "showcase"),
                        ));
                    }
                    hues
                },
                cx,
            ));
        section(
            "Data display",
            "Avatars, presence, badges, tags, stars and color swatches.",
            cx,
            card(cx)
                .child(avatars)
                .child(Divider::horizontal())
                .child(badges)
                .child(Divider::horizontal())
                .child(swatches),
        )
    }

    /// Lists: divided rows with leading/trailing pieces plus a multi-select.
    fn sec_lists(&self, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        let show = cx.entity();

        let show_tap = show.clone();
        let plain_list = List::new()
            .divided()
            .child(
                ListItem::new("li-ada", "Ada Lovelace")
                    .description("Wrote the first algorithm")
                    .leading(Avatar::new("li-ada-av", "Ada Lovelace").size(AvatarSize::Sm))
                    .trailing(CountBadge::new("li-ada-badge", 3)),
            )
            .child(
                ListItem::new("li-shelves", "Shelves")
                    .description("Where the books live")
                    .leading(Icon::new(IconName::Library))
                    .trailing(Icon::new(IconName::ChevronRight)),
            )
            .child(
                ListItem::new("li-struck", "Old draft")
                    .struck(true)
                    .quiet(true)
                    .leading(Icon::new(IconName::FileText)),
            )
            .child(
                ListItem::new("li-tap", "Tap me")
                    .description("A pressable row")
                    .leading(Icon::new(IconName::Sparkles))
                    .on_click(move |_, _, cx| edit(&show_tap, cx, |this| this.taps += 1)),
            );
        let show_list = show.clone();
        let selectable = specimen(
            &format!(
                "SelectableList (multi) — {:?}",
                self.list_sel.iter().map(|k| k.as_ref()).collect::<Vec<_>>()
            ),
            SelectableList::new("ely-select-list")
                .row("sun", ListItem::new("sl-sun", "Sunday"))
                .row("mon", ListItem::new("sl-mon", "Monday"))
                .row("tue", ListItem::new("sl-tue", "Tuesday"))
                .row("wed", ListItem::new("sl-wed", "Wednesday"))
                .multiple()
                .selected(self.list_sel.clone())
                .on_change(move |keys, _, cx| {
                    let keys = keys.to_vec();
                    edit(&show_list, cx, |this| this.list_sel = keys)
                }),
            cx,
        );
        section(
            "Lists",
            "Divided rows with leading/trailing pieces, plus a multi-select list.",
            cx,
            card(cx).child(plain_list).child(selectable),
        )
    }

    /// Feedback: progress, spinners, connection/sync states, save states and
    /// messages.
    fn sec_feedback(&self, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        let show = cx.entity();

        section(
            "Feedback",
            "Progress, spinners, connection/sync states, save states and messages.",
            cx,
            card(cx)
                .child(
                    row()
                        .child(specimen(
                            "ProgressBar 65%",
                            ProgressBar::new("ely-pb", 0.65),
                            cx,
                        ))
                        .child(specimen(
                            "indeterminate",
                            ProgressBar::indeterminate("ely-pb-ind"),
                            cx,
                        ))
                        .child(specimen(
                            "ProgressRing 72%",
                            ProgressRing::new("ely-ring", 0.72).percent().size(rems(3.)),
                            cx,
                        ))
                        .child(specimen("Spinner", Spinner::new("ely-spin"), cx)),
                )
                .child(Divider::horizontal())
                .child(
                    // `Connectivity`'s Reconnecting/Offline variants are not
                    // re-exported from `feedback`, so only the default
                    // (`Online`) is constructible here.
                    col()
                        .child(ConnectionStatus::new("ely-net-on", Default::default()))
                        .child(Caption::new(
                            "Connectivity also has Reconnecting and Offline states, but the \
                             enum is not re-exported from `feedback`.",
                        )),
                )
                .child(
                    row()
                        .child(specimen(
                            "SyncStatus: syncing 3",
                            SyncStatus::new("ely-sync", SyncState::Syncing(3)),
                            cx,
                        ))
                        .child(specimen(
                            "SyncStatus: failed",
                            SyncStatus::new("ely-sync-f", SyncState::Failed),
                            cx,
                        )),
                )
                .child(
                    col()
                        .child(SavingIndicator::new("ely-save-1", SaveState::Saving))
                        .child(SavingIndicator::new("ely-save-2", SaveState::Saved))
                        .child({
                            let show = show.clone();
                            SavingIndicator::new("ely-save-3", SaveState::Failed)
                                .on_retry(move |_, cx| edit(&show, cx, |this| this.taps += 1))
                        }),
                )
                .child(Divider::horizontal())
                .child(
                    col()
                        .child(
                            Alert::new("ely-alert", Severity::Warning, "Battery low")
                                .body("Plug in before the long car ride."),
                        )
                        .child(Banner::new(
                            "ely-banner",
                            Severity::Info,
                            "A new shelf style is available.",
                        ))
                        .child(
                            Callout::new(Severity::Success)
                                .title("Streak saved")
                                .child(Paragraph::new("Five days in a row — keep going!")),
                        )
                        .child(InlineMessage::new(Severity::Danger, "That shelf is full."))
                        .child(EmptyState::new(
                            "ely-empty",
                            IconName::Inbox,
                            "Nothing here yet",
                        )),
                ),
        )
    }

    /// Overlays: the open-dialog trigger plus a self-managed popover. The
    /// dialog element itself renders on the root (see `Render::render`).
    fn sec_overlays(&self, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        let show = cx.entity();

        let popover = Popover::new("ely-popover", "What can this do?", |_, _| {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .max_w(px(280.))
                .child(Paragraph::new(
                    "Ely overlays draw inside the window tree: dialogs cover, popovers anchor.",
                ))
                .child(Caption::new("Tap outside to dismiss the popover."))
        });
        section(
            "Overlays",
            "A dialog held in `Showcase.dialog_open`, plus a self-managed popover.",
            cx,
            card(cx).child(
                row()
                    .child({
                        let show = show.clone();
                        Button::new("ely-open-dialog", "Open dialog")
                            .icon(IconName::ExternalLink)
                            .on_click(move |_, _, cx| {
                                edit(&show, cx, |this| this.dialog_open = true)
                            })
                    })
                    .child(popover),
            ),
        )
    }

    /// Calendar: the mini `forms::Calendar` and `calendar::CalendarMonthView`
    /// with all-day events.
    fn sec_calendar(&self, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        // Static demos: a mini calendar and a month view with events. Dates
        // are built by `demo_date` so no jiff dependency is needed here.
        section(
            "Calendar",
            "The mini `forms::Calendar` and `calendar::CalendarMonthView` with all-day events.",
            cx,
            card(cx).child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_start()
                    .gap_4()
                    .child(Calendar::new("ely-mini-cal").selected(demo_date("2026-10-03")))
                    .child(div().min_w_0().flex_1().child(CalendarMonthView::new(
                        "ely-month",
                        demo_date("2026-10-01"),
                        [
                            Event::all_day(
                                "ely-ev-trip",
                                "Field trip",
                                demo_date("2026-10-07"),
                                demo_date("2026-10-08"),
                                1,
                            ),
                            Event::all_day(
                                "ely-ev-quiz",
                                "Memory quiz",
                                demo_date("2026-10-12"),
                                demo_date("2026-10-12"),
                                4,
                            ),
                            Event::all_day(
                                "ely-ev-party",
                                "Reading party",
                                demo_date("2026-10-23"),
                                demo_date("2026-10-23"),
                                5,
                            ),
                        ],
                    ))),
            ),
        )
    }

    /// Typography: text kinds, inline pieces and helpers.
    fn sec_typography(&self, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        section(
            "Typography",
            "Text kinds, inline pieces and helpers from the typography module.",
            cx,
            card(cx)
                .child(
                    col()
                        .child(Overline::new("Overline"))
                        .child(Title::new("Title"))
                        .child(Heading::h2("Heading 2"))
                        .child(Subtitle::new("Subtitle"))
                        .child(Paragraph::new(
                            "Paragraph: a comfortable line for longer reading on a small screen.",
                        ))
                        .child(Label::new("Label"))
                        .child(Caption::new("Caption")),
                )
                .child(Divider::horizontal())
                .child(
                    row()
                        .child(specimen("Code", Code::new("cargo check"), cx))
                        .child(specimen("Kbd", Kbd::new("enter"), cx))
                        .child(specimen("KbdCombo", KbdCombo::new("ctrl-shift-p"), cx)),
                )
                .child(specimen(
                    "Highlight::matching",
                    Highlight::matching("Ely renders on GPUI, the GPU UI framework.", "GPUI"),
                    cx,
                ))
                .child(Blockquote::new().child(Paragraph::new(
                    "A reader lives a thousand lives before they die.",
                )))
                .child(specimen(
                    "AnimatedNumber tracks the slider",
                    AnimatedNumber::new("ely-anum", self.slider)
                        .decimals(0)
                        .count_up(),
                    cx,
                )),
        )
    }

    /// Motion: spinners, skeletons, a pulsing live dot and a glow.
    fn sec_motion(&self, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        let palette = cx.theme().colors.clone();

        section(
            "Motion",
            "Spinners, skeletons, a pulsing live dot and a glow — all from the motion module.",
            cx,
            card(cx)
                .child(
                    row()
                        .child(specimen("Spinner", Spinner::new("ely-spin-2"), cx))
                        .child(specimen(
                            "Pulse on a live dot",
                            Pulse::new("ely-pulse-2")
                                .child(div().size_2().rounded_full().bg(palette.danger)),
                            cx,
                        )),
                )
                .child(
                    row()
                        .child(specimen(
                            "SkeletonAvatar",
                            SkeletonAvatar::new("ely-sk-av"),
                            cx,
                        ))
                        .child(specimen(
                            "SkeletonText",
                            SkeletonText::new("ely-sk-text", 3),
                            cx,
                        )),
                )
                .child(specimen(
                    "SkeletonCard",
                    SkeletonCard::new("ely-sk-card"),
                    cx,
                )),
        )
    }

    /// Kid corner: self-contained timers, quizzes and flashcards.
    fn sec_kids(&self, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        section(
            "Misc — kid corner",
            "Self-contained timers, quizzes and flashcards; tap through them.",
            cx,
            card(cx)
                .child(Stopwatch::new("ely-stopwatch"))
                .child(Divider::horizontal())
                .child(Quiz::new(
                    "ely-quiz",
                    [
                        QuizQuestion::new(
                            "Which planet has the most moons?",
                            ["Mars", "Saturn", "Venus"],
                            1,
                        ),
                        QuizQuestion::new("What do bees make?", ["Honey", "Milk", "Silk"], 0),
                    ],
                ))
                .child(Divider::horizontal())
                .child(Flashcards::new(
                    "ely-cards",
                    [
                        ("Bonjour", "Hello (French)"),
                        ("Gracias", "Thank you (Spanish)"),
                    ],
                )),
        )
    }

    /// Layout & primitives: cards, wells, dividers, an accordion and icons.
    fn sec_layout(&self, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        section(
            "Layout & primitives",
            "Cards, wells, dividers, an accordion, and icons from the primitives module.",
            cx,
            card(cx)
                .child(
                    Card::new().header(
                        CardHeader::new("The reading nook")
                            .description("A Card with a CardHeader.")
                            .action(IconButton::new("ely-card-more", IconName::EllipsisVertical)),
                    ),
                )
                .child(Well::new().child(Caption::new("A Well: sunken ground for quiet notes.")))
                .child(Divider::horizontal().label("or"))
                .child(
                    Accordion::new("ely-accordion")
                        .first_open()
                        .item(
                            AccordionItem::new("How do streaks work?").child(Caption::new(
                                "Read ten minutes a day and the flame stays lit.",
                            )),
                        )
                        .item(AccordionItem::new("Can two kids share a shelf?").child(
                            Caption::new("Yes — switch readers from the top-right avatar."),
                        )),
                )
                .child(
                    row()
                        .child(Icon::new(IconName::Rocket))
                        .child(Icon::new(IconName::Heart))
                        .child(Icon::new(IconName::Star))
                        .child(Icon::new(IconName::Wifi))
                        .child(Icon::new(IconName::Camera))
                        .child(Icon::new(IconName::MapPin)),
                ),
        )
    }

    /// Settings: `SettingsSection` / `SettingsRow` wrapping real controls.
    fn sec_settings(&self, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        let show = cx.entity();

        section(
            "Settings",
            "SettingsSection / SettingsRow wrapping real controls.",
            cx,
            card(cx).child(
                SettingsSection::new("Device")
                    .description("How the showcase behaves on this tablet.")
                    .row({
                        let show = show.clone();
                        SettingsRow::new("Wi-Fi")
                            .description("Join saved networks automatically.")
                            .control(
                                Switch::new("ely-set-wifi", self.wifi).on_change(
                                    move |on, _, cx| edit(&show, cx, |this| this.wifi = on),
                                ),
                            )
                    })
                    .row({
                        let show = show.clone();
                        SettingsRow::new("Sound")
                            .description("Play a chime when a timer ends.")
                            .control(Switch::new("ely-set-sound", self.sound).on_change(
                                move |on, _, cx| edit(&show, cx, |this| this.sound = on),
                            ))
                    })
                    .row({
                        let show = show.clone();
                        SettingsRow::new("Render scale")
                            .description("Render below native resolution: less sharp, much faster.")
                            .control({
                                let selected = if self.render_scale >= 0.99 {
                                    "full"
                                } else if self.render_scale >= 0.74 {
                                    "high"
                                } else {
                                    "mid"
                                };
                                SegmentedControl::new("ely-render-scale", selected)
                                    .segment("full", "100%", None)
                                    .segment("high", "75%", None)
                                    .segment("mid", "50%", None)
                                    .on_change(move |value, _, cx| {
                                        let scale = match value.as_ref() {
                                            "high" => 0.75,
                                            "mid" => 0.5,
                                            _ => 1.0,
                                        };
                                        gpui_mobile::set_render_scale(scale);
                                        edit(&show, cx, |this| this.render_scale = scale);
                                    })
                            })
                    }),
            ),
        )
    }
}

/// Pipes a component callback into a `Showcase` field edit and a repaint.
/// Ely's callbacks hand us `&mut App`, so we carry the entity handle in.
/// Status-bar inset in logical px (0 off-Android). The NativeActivity surface
/// is edge-to-edge, so the header pads itself by this to clear the status bar.
fn query_safe_top() -> f32 {
    #[cfg(target_os = "android")]
    {
        gpui_mobile::android::jni::platform()
            .and_then(|p| p.primary_window())
            .map(|w| w.safe_area_insets_logical().top)
            .unwrap_or(0.0)
    }
    #[cfg(not(target_os = "android"))]
    {
        0.0
    }
}

fn edit(show: &Entity<Showcase>, cx: &mut App, f: impl FnOnce(&mut Showcase)) {
    show.update(cx, |this, cx| {
        f(this);
        cx.notify();
    });
}

// --- Small layout helpers -----------------------------------------------------

/// A titled, noted section drawn on a theme-colored card.
fn section(title: &str, note: &str, cx: &App, body: Div) -> Div {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_0p5()
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Md))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.colors.fg)
                        .child(SharedString::from(title)),
                )
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(theme.colors.fg_muted)
                        .child(SharedString::from(note)),
                ),
        )
        .child(body)
}

/// The card a section draws its demos on. Chain `.child(...)` onto it.
fn card(cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_4()
        .rounded(theme.radius(Radius::Md))
        .border_1()
        .border_color(theme.colors.border)
        .bg(theme.colors.surface)
        .p_4()
}

/// A wrapping row of demos. Touch: rows we hand-roll stay at least 44px tall.
fn row() -> Div {
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_3()
        .min_h(px(44.))
}

/// A column of demos.
fn col() -> Div {
    div().flex().flex_col().gap_3().min_w_0()
}

/// A demo with a small caption beneath, so each piece names itself.
fn specimen(caption: &str, body: impl IntoElement, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .min_w(px(140.))
        .child(body)
        .child(
            div()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(theme.colors.fg_subtle)
                .child(SharedString::from(caption)),
        )
}

/// A `jiff` date for the static calendar demos without adding a jiff
/// dependency: the target type comes from the parameter it lands in
/// (`jiff::civil::Date` parses `"YYYY-MM-DD"`).
fn demo_date<D: std::str::FromStr>(text: &str) -> D {
    text.parse()
        .unwrap_or_else(|_| panic!("showcase: bad demo date {text}"))
}

// -----------------------------------------------------------------------------

impl Render for Showcase {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // An owned snapshot of the theme: `cx.theme()` borrows `cx`, which the
        // keyed-state calls below need mutably.
        let palette = cx.theme().colors.clone();
        let font_family = cx.theme().font_family.clone();

        // Key events dispatch along the focus path, and actions bubble up
        // it: the root on_action (escape→Back) is reachable whenever the
        // focused element is the root itself or any descendant. Claim focus
        // only when the current focus is missing OR stale (its element left
        // the tree — e.g. after switching sections): grabbing it
        // unconditionally would steal it back from overlays one frame after
        // they take it (popovers self-closed), and skipping whenever
        // *anything* is focused left BACK dead on a stale handle.
        if !self.focus.contains_focused(window, cx) {
            window.focus(&self.focus, cx);
        }

        // Follow OS theme changes live while the preference is `System` (the
        // fork re-applies appearance on Android ConfigChanged).
        if self.theme_sub.is_none() {
            let entity = cx.entity();
            self.theme_sub = Some(window.observe_window_appearance(move |_window, app| {
                entity.update(app, |this, cx| {
                    if this.theme_pref == ThemePref::System {
                        Self::apply_pref(ThemePref::System, cx);
                    }
                });
            }));
        }

        // ---- Header ---------------------------------------------------------
        // Theme preference: System / Light / Dark, top-right beside the title.
        // `System` follows the OS; applying is immediate (`set_mode_now`).
        let theme_show = cx.entity();
        let theme_sel = SegmentedControl::new("ely-theme", self.theme_pref_id())
            .segment("auto", "Auto", None)
            .segment("light", "Light", None)
            .segment("dark", "Dark", None)
            .on_change(move |value, _, cx| {
                let pref = match value.as_ref() {
                    "light" => ThemePref::Light,
                    "dark" => ThemePref::Dark,
                    _ => ThemePref::System,
                };
                edit(&theme_show, cx, move |this| this.theme_pref = pref);
                Self::apply_pref(pref, cx);
            });
        let header = div()
            .flex()
            .flex_row()
            .items_start()
            .justify_between()
            .gap_4()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Overline::new("MDRV"))
                    .child(Title::new("MDRV GPUI Ely"))
                    .child(Caption::new(
                        "Every kept Ely GPUI component family, one section at a time.",
                    )),
            )
            .child(div().flex_none().child(theme_sel));

        // ---- Section chips ----------------------------------------------------
        let nav = self.section_nav(cx);

        let footer = Caption::new("MDRV GPUI Ely — pick a family above; “All” builds everything.");

        // ---- The active section, and only it --------------------------------
        let section_body = match self.section.as_ref() {
            "all" => self.all_sections(window, cx),
            "buttons" => self.sec_buttons(window, cx),
            "forms" => self.sec_forms(window, cx),
            "data" => self.sec_data(window, cx),
            "lists" => self.sec_lists(window, cx),
            "feedback" => self.sec_feedback(window, cx),
            "overlays" => self.sec_overlays(window, cx),
            "calendar" => self.sec_calendar(window, cx),
            "type" => self.sec_typography(window, cx),
            "motion" => self.sec_motion(window, cx),
            "kids" => self.sec_kids(window, cx),
            "layout" => self.sec_layout(window, cx),
            _ => self.sec_settings(window, cx),
        };

        // The demo dialog renders on the root, above the scroll container, so
        // its scrim covers the whole window — while Overlays or All is active
        // AND the reader asked for it (`dialog_open`). Backdrop for the
        // Cancel/escape path: `dialog_open` is the single source of truth.
        let dialog = (matches!(self.section.as_ref(), "overlays" | "all") && self.dialog_open)
            .then(|| {
                let name = window.use_keyed_state("ely-name", cx, |window, cx| {
                    TextInput::new(window, cx).placeholder("Type a name")
                });
                let shut_show = cx.entity();
                let shut = move |_: &mut Window, cx: &mut App| {
                    edit(&shut_show, cx, |this| this.dialog_open = false)
                };
                let save_show = cx.entity();
                Dialog::new("ely-dialog", "Add a reader", shut.clone())
                    .detail("Readers get their own shelf, streaks and a reading buddy.")
                    .child(Input::new(&name))
                    .action(|close| {
                        Button::new("ely-dialog-cancel", "Cancel")
                            .variant(ButtonVariant::Outline)
                            .on_click(move |_, window, cx| close(window, cx))
                    })
                    .action(move |close| {
                        let save_show = save_show.clone();
                        Button::new("ely-dialog-save", "Add").primary().on_click(
                            move |_, window, cx| {
                                edit(&save_show, cx, |this| {
                                    this.taps += 1;
                                    this.dialog_open = false;
                                });
                                close(window, cx)
                            },
                        )
                    })
                    .into_any_element()
            });

        // BACK confirmation: independent of the Overlays demo, renders on
        // any section. Confirming quits via the platform (activity finishes).
        let quit_dialog = self.quit_confirm.then(|| {
            let shut_show = cx.entity();
            let shut = move |_: &mut Window, cx: &mut App| {
                edit(&shut_show, cx, |this| this.quit_confirm = false)
            };
            let exit_show = cx.entity();
            Dialog::new("ely-quit-dialog", "Exit the showcase?", shut)
                .detail("Close MDRV GPUI Ely? The demo state resets on relaunch.")
                .action(|close| {
                    Button::new("ely-quit-cancel", "Stay")
                        .variant(ButtonVariant::Outline)
                        .on_click(move |_, window, cx| close(window, cx))
                })
                .action(move |close| {
                    let exit_show = exit_show.clone();
                    Button::new("ely-quit-exit", "Exit")
                        .primary()
                        .on_click(move |_, window, cx| {
                            edit(&exit_show, cx, |this| this.quit_confirm = false);
                            close(window, cx);
                            // Finish the activity (not just the gpui loop):
                            // the Android process only dies when the
                            // activity finishes.
                            #[cfg(target_os = "android")]
                            gpui_mobile::android::jni::finish_activity();
                            #[cfg(not(target_os = "android"))]
                            log::info!("exit confirmed (host build: nothing to finish)");
                        })
                })
                .into_any_element()
        });

        // The page: a fixed root with a vertically scrolling column inside.
        div()
            .id("showcase-root")
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &Back, _window, cx| {
                // Cancel whatever overlay is up first; only when the page is
                // bare ask the reader to confirm exiting (swallowing BACK
                // silently made the hardware key feel dead).
                if this.dialog_open || this.quit_confirm {
                    this.dialog_open = false;
                    this.quit_confirm = false;
                } else {
                    this.quit_confirm = true;
                }
                cx.notify();
            }))
            .size_full()
            .flex()
            .flex_col()
            .bg(palette.bg)
            .text_color(palette.fg)
            .font_family(font_family)
            .child(
                div()
                    .id("showcase-scroll")
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_6()
                    .px_4()
                    .py_6()
                    .pb_12()
                    // Keep the header (and its theme toggle) clear of the
                    // status bar on edge-to-edge NativeActivity surfaces.
                    .pt(px(self.safe_top + 16.))
                    .child(header)
                    .child(nav)
                    .child(section_body)
                    .child(footer),
            )
            // The dialog lives on the root, above the scroll container.
            .children(dialog)
            .children(quit_dialog)
    }
}
