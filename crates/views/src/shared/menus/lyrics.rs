use gpui::{App, Pixels, Point, point, px};
use i18n::t;
use state::Sonora;
use ui::{Menu, MenuItem};

/// A right-click opens just below the pointer by default, which would sit over whatever is
/// beneath the icon. Shifting the anchor up by roughly the menu's own height makes it open
/// above the icon instead, the way a Windows tray icon's menu does.
const MENU_HEIGHT: Pixels = px(44.);

pub(crate) fn upward(at: Point<Pixels>) -> Point<Pixels> {
    point(at.x, (at.y - MENU_HEIGHT).max(Pixels::ZERO))
}

/// The small menu a right-click on a lyrics button opens.
pub(crate) fn lyrics_options(cx: &App) -> Menu {
    let settings = Sonora::global(cx).settings.clone();
    let hidden = settings.read(cx).hide_profanity();

    Menu::new("lyrics-options").item(
        MenuItem::new(
            "lyrics-profanity-toggle",
            match hidden {
                true => t!("lyrics-profanity-show"),
                false => t!("lyrics-profanity-hide"),
            },
        )
        .icon(match hidden {
            true => "icons/eye.svg",
            false => "icons/eye-off.svg",
        })
        .on_click(move |_, _, cx| {
            settings.update(cx, |settings, cx| settings.set_hide_profanity(!hidden, cx));
        }),
    )
}
