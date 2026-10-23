//! A small window that appears for a few seconds whenever a new track starts, the way JRiver's
//! on-screen display does. It never steals focus or blocks input to whatever is underneath.

use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    Bounds, Context, Entity, FontWeight, SharedString, Window, WindowBounds, WindowHandle,
    WindowOptions, div, point, px, size, svg,
};

/// Makes a just-opened window sit above everything else without ever becoming the active
/// window: a click elsewhere, or switching to another app, leaves it exactly where it is
/// instead of hiding it, since it never took the focus to lose in the first place.
#[cfg(target_os = "windows")]
fn make_overlay(window: &Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GWL_STYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_BORDER, WS_CAPTION,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_THICKFRAME,
    };

    let Ok(RawWindowHandle::Win32(handle)) =
        HasWindowHandle::window_handle(window).map(|handle| handle.as_raw())
    else {
        return;
    };
    let hwnd = handle.hwnd.get() as *mut std::ffi::c_void;
    // Only the style bits themselves are touched here. A plain style write does not make
    // Windows talk back to the app; asking it to also redraw the frame right away (which an
    // earlier version of this did) could, and that is almost certainly what the crash was.
    unsafe {
        // Drop the thin system frame a plain top-level window still draws even without gpui's
        // own titlebar.
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_STYLE,
            style & !(WS_CAPTION | WS_THICKFRAME | WS_BORDER) as isize,
        );
        // Never activate, so clicking elsewhere never counts as this window losing focus;
        // sit above other windows; skip the taskbar and Alt+Tab, since it is a notice, not a
        // window of its own.
        let extended = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            extended | (WS_EX_NOACTIVATE | WS_EX_TOPMOST | WS_EX_TOOLWINDOW) as isize,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn make_overlay(_window: &Window) {}

use music::Track;
use ui::{ActiveTheme as _, Artwork};

use crate::playback::PlaybackEvent;
use crate::{AppSettings, NoticePosition, Playback, Session};

const WIDTH: f32 = 360.;
const HEIGHT: f32 = 92.;
const LOGO: &str = "brand/r3tro-logo.svg";

/// Lives only to hold the subscription open; nothing reads it directly.
pub struct TrackNotice {
    session: Entity<Session>,
    settings: Entity<AppSettings>,
    last: Option<String>,
    window: Option<WindowHandle<Card>>,
}

impl TrackNotice {
    pub fn new(
        playback: Entity<Playback>,
        session: Entity<Session>,
        settings: Entity<AppSettings>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.subscribe(&playback, |this, playback, event, cx| {
            if !matches!(event, PlaybackEvent::StartedPlayback) {
                return;
            }
            if !this.settings.read(cx).track_notifications() {
                return;
            }
            let Some(track) = playback.read(cx).track().cloned() else {
                return;
            };
            let id = track.id.clone().unwrap_or_default();
            if this.last.as_deref() == Some(id.as_str()) {
                return;
            }
            this.last = Some(id);
            let seconds = this.settings.read(cx).track_notification_seconds();
            let position = this.settings.read(cx).track_notification_position();
            let provider = track
                .id
                .as_deref()
                .and_then(|id| this.session.read(cx).provider_for(id))
                .map(|provider| SharedString::from(provider.listening_to()));
            this.show(track, provider, Duration::from_secs(u64::from(seconds)), position, cx);
        })
        .detach();

        Self {
            session,
            settings,
            last: None,
            window: None,
        }
    }

    fn show(
        &mut self,
        track: Track,
        provider: Option<SharedString>,
        shown_for: Duration,
        position: NoticePosition,
        cx: &mut Context<Self>,
    ) {
        if let Some(window) = self.window.take() {
            window.update(cx, |_, window, _| window.remove_window()).ok();
        }

        let card = size(px(WIDTH), px(HEIGHT));
        const MARGIN: f32 = 24.;
        let screen = cx.displays().into_iter().next().map(|display| display.bounds());
        let bounds = match (position, screen) {
            (NoticePosition::Center, _) | (_, None) => Bounds::centered(None, card, cx),
            (NoticePosition::TopLeft, Some(screen)) => Bounds {
                origin: point(screen.origin.x + px(MARGIN), screen.origin.y + px(MARGIN)),
                size: card,
            },
            (NoticePosition::TopRight, Some(screen)) => Bounds {
                origin: point(
                    screen.origin.x + screen.size.width - card.width - px(MARGIN),
                    screen.origin.y + px(MARGIN),
                ),
                size: card,
            },
            (NoticePosition::BottomLeft, Some(screen)) => Bounds {
                origin: point(
                    screen.origin.x + px(MARGIN),
                    screen.origin.y + screen.size.height - card.height - px(MARGIN),
                ),
                size: card,
            },
            (NoticePosition::BottomRight, Some(screen)) => Bounds {
                origin: point(
                    screen.origin.x + screen.size.width - card.width - px(MARGIN),
                    screen.origin.y + screen.size.height - card.height - px(MARGIN),
                ),
                size: card,
            },
        };
        let Ok(handle) = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                is_movable: false,
                is_resizable: false,
                ..Default::default()
            },
            |_window, cx| cx.new(|_cx| Card { track, provider }),
        ) else {
            return;
        };
        self.window = Some(handle);
        // Styling the native window right here, still inside the call that created it, can
        // make Windows dispatch a message synchronously back into gpui before this call
        // returns — and gpui then finds the window already borrowed. A short delay lets this
        // call finish and the borrow close before the native styling touches the window at all.
        cx.spawn(async move |_, cx| {
            cx.background_executor().timer(Duration::from_millis(16)).await;
            handle.update(cx, |_, window, _| make_overlay(window)).ok();
        })
        .detach();

        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(shown_for).await;
            this.update(cx, |this, cx| this.dismiss(cx)).ok();
        })
        .detach();
    }

    fn dismiss(&mut self, cx: &mut Context<Self>) {
        if let Some(window) = self.window.take() {
            window.update(cx, |_, window, _| window.remove_window()).ok();
        }
    }
}

struct Card {
    track: Track,
    provider: Option<SharedString>,
}

impl Render for Card {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let title: SharedString = self.track.name.clone().into();
        let artist: SharedString = self.track.artists.clone().into();
        let provider = self.provider.clone();

        div()
            .size_full()
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .rounded(theme.radius)
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .text_color(theme.foreground)
            .child(
                Artwork::new(self.track.cover.clone())
                    .size(px(56.))
                    .corner_radius(px(8.)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .flex_1()
                    .gap_1()
                    .child(
                        div()
                            .truncate()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_color(theme.muted_foreground)
                            .child(artist),
                    )
                    .when_some(provider, |this, provider| {
                        this.child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.primary)
                                .child(provider),
                        )
                    }),
            )
            .child(
                svg()
                    .path(LOGO)
                    .size(px(20.))
                    .flex_none()
                    .text_color(theme.primary),
            )
    }
}
