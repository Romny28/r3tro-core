//! R3TRØ CORE's own settings: the profile, the accent colour, the song notice, the exclusive
//! mode, hiding explicit words in lyrics and the credits. They live in this one file, apart from
//! Sonora's pages, so that a Sonora update has next to nothing here to collide with. The rest of
//! the settings module only lists their slots and calls into this file.

use std::path::Path;
use std::sync::Arc;

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Entity, FontWeight, ImageSource, ObjectFit, PathPromptOptions,
    Resource, SharedString, div, img, px, rgb, rgba,
};
use i18n::t;
use router::SettingsPage;
use state::{AppSettings, NoticePosition};
use ui::{
    ActiveTheme as _, Avatar, Button, Initials, Input, MenuItem, Picker, Switch, Text, Theme,
};

use super::{Group, SettingsView, Slot, group_heading};

/// The accent colours the picker offers: white, mint, sky blue, cyan, pink, lavender, amber, red
/// and green.
const ACCENTS: [&str; 9] = [
    "#FFFFFF", "#3DF5C2", "#7AA2FF", "#3DD9F5", "#FF7AB6", "#C58BFF", "#FFB347", "#FF6B6B",
    "#7CE38B",
];
/// How long the song notice may stay up, in seconds.
const NOTICE_DURATIONS: [u32; 6] = [2, 3, 4, 5, 8, 10];
const EXCLUSIVE_MODE: &str = "exclusive-mode";
const TRACK_NOTIFICATION_SECONDS: &str = "track-notification-seconds";
const TRACK_NOTIFICATION_POSITION: &str = "track-notification-position";

/// The people behind R3TRØ CORE: login, picture, profile and whether they are the co-creator.
const CREATORS: [(&str, &str, &str, bool); 2] = [
    (
        "Romny28",
        "https://github.com/Romny28.png",
        "https://github.com/Romny28",
        false,
    ),
    (
        "Claude (Anthropic)",
        "https://claude.ai/favicon.ico",
        "https://claude.com",
        true,
    ),
];

/// What the settings view keeps for R3TRØ CORE's rows: the two text fields of the profile.
pub(super) struct R3tro {
    profile_name: Entity<Input>,
    profile_greeting: Entity<Input>,
}

impl R3tro {
    pub(super) fn new(settings: &Entity<AppSettings>, cx: &mut Context<SettingsView>) -> Self {
        let profile_name = cx.new(|cx| Input::new("settings-profile-name-hint", cx).compact());
        let name = settings.read(cx).profile_name().to_owned();
        profile_name.update(cx, |input, cx| input.set_text(name, cx));
        let stored = settings.clone();
        cx.observe(&profile_name, move |_, input, cx| {
            let text = input.read(cx).text().trim().to_owned();
            stored.update(cx, |settings, cx| settings.set_profile_name(text, cx));
        })
        .detach();

        let profile_greeting =
            cx.new(|cx| Input::new("settings-profile-greeting-hint", cx).compact());
        let greeting = settings.read(cx).profile_greeting().to_owned();
        profile_greeting.update(cx, |input, cx| input.set_text(greeting, cx));
        let stored = settings.clone();
        cx.observe(&profile_greeting, move |_, input, cx| {
            let text = input.read(cx).text().trim().to_owned();
            stored.update(cx, |settings, cx| settings.set_profile_greeting(text, cx));
        })
        .detach();

        Self {
            profile_name,
            profile_greeting,
        }
    }
}

impl SettingsView {
    /// Lays R3TRØ CORE's slots into the groups Sonora's pages list.
    pub(super) fn r3tro_groups(&self, page: SettingsPage, groups: &mut Vec<Group>) {
        match page {
            SettingsPage::General => {
                let after_window = groups
                    .iter()
                    .position(|group| group.rows.contains(&Slot::Tray))
                    .map_or(groups.len(), |at| at + 1);
                groups.insert(
                    after_window,
                    Group::new("settings-group-notice", "icons/list-music.svg").rows([
                        Slot::R3Notice,
                        Slot::R3NoticeSeconds,
                        Slot::R3NoticePosition,
                    ]),
                );
                groups.insert(
                    0,
                    Group::new("settings-group-profile", "icons/user-round.svg")
                        .rows([Slot::R3Profile]),
                );
            }
            SettingsPage::Appearance => {
                if let Some(group) = groups.iter_mut().find(|group| group.cards == Some(Slot::Theme))
                {
                    group.rows.insert(0, Slot::R3Accent);
                }
            }
            SettingsPage::Sound => {
                if let Some(group) = groups
                    .iter_mut()
                    .find(|group| group.rows.contains(&Slot::OutputDevice))
                {
                    group.rows.push(Slot::R3Exclusive);
                }
            }
            SettingsPage::Lyrics => {
                if let Some(group) = groups
                    .iter_mut()
                    .find(|group| group.rows.contains(&Slot::Karaoke))
                {
                    group.rows.push(Slot::R3HideProfanity);
                }
            }
            _ => {}
        }
    }

    /// The title and detail a search reads for one of R3TRØ CORE's slots.
    pub(super) fn r3tro_meta(&self, slot: Slot) -> Option<(SharedString, SharedString)> {
        let meta = match slot {
            Slot::R3Profile => (
                t!("settings-group-profile"),
                t!("settings-profile-name-hint"),
            ),
            Slot::R3Accent => (t!("settings-accent"), t!("settings-accent-detail")),
            Slot::R3Exclusive => (
                t!("settings-exclusive-mode"),
                t!("settings-exclusive-mode-detail"),
            ),
            Slot::R3Notice => (
                t!("settings-track-notifications"),
                t!("settings-track-notifications-detail"),
            ),
            Slot::R3NoticeSeconds => (
                t!("settings-track-notification-seconds"),
                t!("settings-track-notification-seconds-detail"),
            ),
            Slot::R3NoticePosition => (
                t!("settings-track-notification-position"),
                t!("settings-track-notification-position-detail"),
            ),
            Slot::R3HideProfanity => (
                t!("settings-hide-profanity"),
                t!("settings-hide-profanity-detail"),
            ),
            _ => return None,
        };
        Some(meta)
    }

    /// Builds one of R3TRØ CORE's rows.
    pub(super) fn r3tro_row(&self, slot: Slot, cx: &mut Context<Self>) -> AnyElement {
        match slot {
            Slot::R3Profile => self.profile_editor(cx),
            Slot::R3Accent => self.accent_row(cx),
            Slot::R3Exclusive => self.exclusive_mode_row(cx),
            Slot::R3Notice => self.track_notifications_row(cx),
            Slot::R3NoticeSeconds => self.track_notification_seconds_row(cx),
            Slot::R3NoticePosition => self.track_notification_position_row(cx),
            Slot::R3HideProfanity => self.hide_profanity_row(cx),
            _ => div().into_any_element(),
        }
    }

    /// The profile card: picture, name, nickname or info line, and an optional background.
    fn profile_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = *cx.theme();
        let (name, background, avatar) = {
            let settings = self.settings.read(cx);
            (
                settings.profile_name().to_owned(),
                settings.profile_background().to_owned(),
                settings.profile_avatar().to_owned(),
            )
        };
        let account = match self.session.read(cx).state() {
            state::SessionState::SignedIn(profile) => Some(profile.display_name.to_string()),
            _ => None,
        };
        let shown = match name.is_empty() {
            false => name.clone(),
            true => account.unwrap_or_default(),
        };

        let for_background = self.settings.clone();
        let for_background_clear = self.settings.clone();
        let for_avatar = self.settings.clone();
        let for_avatar_clear = self.settings.clone();
        let mut card = div()
            .relative()
            .overflow_hidden()
            .w_full()
            .rounded(theme.radius)
            .border_1()
            .border_color(theme.border)
            .bg(theme.secondary);
        if !background.is_empty() {
            card = card
                .child(
                    img(ImageSource::Resource(Resource::Path(Arc::from(Path::new(
                        &background,
                    )))))
                    .absolute()
                    .size_full()
                    .object_fit(ObjectFit::Cover),
                )
                .child(div().absolute().size_full().bg(rgba(0x000000a6)));
        }

        card.child(
            div()
                .relative()
                .flex()
                .flex_col()
                .gap_3()
                .p_4()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_4()
                        .child(match avatar.is_empty() {
                            true => Initials::new(shown, px(64.)).into_any_element(),
                            false => Avatar::new(Some(format!("file://{avatar}")))
                                .size(px(64.))
                                .into_any_element(),
                        })
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(div().w(px(260.)).child(self.r3tro.profile_name.clone()))
                                .child(
                                    div()
                                        .w(px(260.))
                                        .child(self.r3tro.profile_greeting.clone()),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .w(px(96.))
                                        .flex_none()
                                        .text_size(theme.text(Text::Small))
                                        .text_color(theme.muted_foreground)
                                        .child(t!("settings-profile-picture")),
                                )
                                .child(
                                    Button::new("profile-avatar")
                                        .small()
                                        .label(t!("settings-profile-avatar"))
                                        .on_click(move |_, _, cx| {
                                            choose_profile_image(for_avatar.clone(), true, cx)
                                        }),
                                )
                                .when(!avatar.is_empty(), |this| {
                                    this.child(
                                        Button::new("profile-avatar-clear")
                                            .small()
                                            .ghost()
                                            .label(t!("settings-profile-avatar-clear"))
                                            .on_click(move |_, _, cx| {
                                                for_avatar_clear.update(cx, |settings, cx| {
                                                    settings.set_profile_avatar(String::new(), cx)
                                                });
                                            }),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .w(px(96.))
                                        .flex_none()
                                        .text_size(theme.text(Text::Small))
                                        .text_color(theme.muted_foreground)
                                        .child(t!("settings-profile-background-label")),
                                )
                                .child(
                                    Button::new("profile-background")
                                        .small()
                                        .label(t!("settings-profile-background"))
                                        .on_click(move |_, _, cx| {
                                            choose_profile_image(for_background.clone(), false, cx)
                                        }),
                                )
                                .when(!background.is_empty(), |this| {
                                    this.child(
                                        Button::new("profile-background-clear")
                                            .small()
                                            .ghost()
                                            .label(t!("settings-profile-background-clear"))
                                            .on_click(move |_, _, cx| {
                                                for_background_clear.update(cx, |settings, cx| {
                                                    settings
                                                        .set_profile_background(String::new(), cx)
                                                });
                                            }),
                                    )
                                }),
                        ),
                ),
        )
        .into_any_element()
    }

    fn accent_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = *cx.theme();
        let look = self.look(cx);
        let chosen = self.settings.read(cx).accent().to_owned();

        let swatches = ACCENTS.into_iter().enumerate().map(|(index, hex)| {
            let selected = chosen.eq_ignore_ascii_case(hex);
            let color = rgb(u32::from_str_radix(&hex[1..], 16).unwrap_or(0xffffff));
            div()
                .id(("accent", index))
                .size(px(22.))
                .flex_none()
                .rounded_full()
                .bg(color)
                .border_2()
                .border_color(match selected {
                    true => theme.foreground,
                    false => theme.border,
                })
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    let overrides = this.settings.update(cx, |settings, cx| {
                        settings.set_accent(hex.to_owned(), cx);
                        settings.theme_overrides()
                    });
                    Theme::fade(look, &overrides, cx);
                    cx.notify();
                }))
        });
        let by_default = Button::new("accent-default")
            .ghost()
            .small()
            .label(t!("settings-accent-default"))
            .selected(chosen.is_empty())
            .on_click(cx.listener(move |this, _, _, cx| {
                let overrides = this.settings.update(cx, |settings, cx| {
                    settings.set_accent(String::new(), cx);
                    settings.theme_overrides()
                });
                Theme::fade(look, &overrides, cx);
                cx.notify();
            }));
        let automatic = Button::new("accent-auto")
            .ghost()
            .small()
            .label(t!("settings-accent-auto"))
            .selected(chosen == "auto")
            .on_click(cx.listener(move |this, _, _, cx| {
                let overrides = this.settings.update(cx, |settings, cx| {
                    settings.set_accent("auto".to_owned(), cx);
                    settings.theme_overrides()
                });
                Theme::fade(look, &overrides, cx);
                cx.notify();
            }));

        self.row(
            t!("settings-accent"),
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(by_default)
                .child(automatic)
                .children(swatches)
                .into_any_element(),
        )
    }

    fn exclusive_mode_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let chosen = self.settings.read(cx).exclusive_mode();
        let label = |mode: music::ExclusiveMode| -> SharedString {
            match mode {
                music::ExclusiveMode::Off => t!("settings-exclusive-mode-off"),
                music::ExclusiveMode::Auto => t!("settings-exclusive-mode-auto"),
                music::ExclusiveMode::Always => t!("settings-exclusive-mode-always"),
            }
        };

        let picker = Picker::new(EXCLUSIVE_MODE, &self.popovers, label(chosen))
            .width(Picker::NARROW)
            .items(music::ExclusiveMode::ALL.map(|mode| {
                MenuItem::new(mode.id(), label(mode))
                    .selected(mode == chosen)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings
                            .update(cx, |settings, cx| settings.set_exclusive_mode(mode, cx));
                        cx.notify();
                    }))
            }));

        self.row(t!("settings-exclusive-mode"), picker.into_any_element())
    }

    fn track_notifications_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let on = self.settings.read(cx).track_notifications();

        self.row(
            t!("settings-track-notifications"),
            Switch::new("track-notifications", on)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.settings
                        .update(cx, |settings, cx| settings.set_track_notifications(!on, cx));
                }))
                .into_any_element(),
        )
    }

    fn track_notification_position_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let chosen = self.settings.read(cx).track_notification_position();
        let label = |position: NoticePosition| -> SharedString {
            match position {
                NoticePosition::TopLeft => t!("settings-notice-position-top-left"),
                NoticePosition::TopRight => t!("settings-notice-position-top-right"),
                NoticePosition::BottomLeft => t!("settings-notice-position-bottom-left"),
                NoticePosition::BottomRight => t!("settings-notice-position-bottom-right"),
                NoticePosition::Center => t!("settings-notice-position-center"),
            }
        };

        let picker = Picker::new(TRACK_NOTIFICATION_POSITION, &self.popovers, label(chosen))
            .width(Picker::NARROW)
            .items(NoticePosition::ALL.map(|position| {
                MenuItem::new(position.id(), label(position))
                    .selected(position == chosen)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings.update(cx, |settings, cx| {
                            settings.set_track_notification_position(position, cx)
                        });
                        cx.notify();
                    }))
            }));

        self.row(
            t!("settings-track-notification-position"),
            picker.into_any_element(),
        )
    }

    fn track_notification_seconds_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let chosen = self.settings.read(cx).track_notification_seconds();
        let label = |seconds: u32| -> SharedString {
            SharedString::from(t!("settings-track-notification-seconds-value", seconds = seconds))
        };

        let picker = Picker::new(TRACK_NOTIFICATION_SECONDS, &self.popovers, label(chosen))
            .width(Picker::NARROW)
            .items(NOTICE_DURATIONS.map(|seconds| {
                MenuItem::new(seconds.to_string(), label(seconds))
                    .selected(seconds == chosen)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings.update(cx, |settings, cx| {
                            settings.set_track_notification_seconds(seconds, cx)
                        });
                        cx.notify();
                    }))
            }));

        self.row(
            t!("settings-track-notification-seconds"),
            picker.into_any_element(),
        )
    }

    fn hide_profanity_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let on = self.settings.read(cx).hide_profanity();

        self.row(
            t!("settings-hide-profanity"),
            Switch::new("hide-profanity", on)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.settings
                        .update(cx, |settings, cx| settings.set_hide_profanity(!on, cx));
                }))
                .into_any_element(),
        )
    }

    /// The creators of R3TRØ CORE, each opening their profile, and the note about the company.
    /// Sonora's own team follows under its own heading.
    pub(super) fn r3tro_credits(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();

        div()
            .flex()
            .flex_col()
            .child(group_heading(
                t!("settings-creators"),
                Some("icons/user-round.svg"),
                cx,
            ))
            .children(
                CREATORS
                    .into_iter()
                    .enumerate()
                    .map(|(index, (login, avatar, profile, co_creator))| {
                        div()
                            .id(("creator", index))
                            .flex()
                            .items_center()
                            .gap_3()
                            .h(theme.metrics.list_row)
                            .mx_neg_3()
                            .px_3()
                            .rounded(theme.radius)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.secondary_hover))
                            .on_click(move |_, _, cx| cx.open_url(profile))
                            .child(Avatar::new(Some(avatar)).size(theme.metrics.thumb))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(login),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .text_size(theme.text(Text::Label))
                                    .text_color(theme.muted_foreground)
                                    .child(match co_creator {
                                        true => t!("settings-role-cocreator"),
                                        false => t!("settings-role-creator"),
                                    }),
                            )
                    }),
            )
            .child(
                div()
                    .mt_3()
                    .text_size(theme.text(Text::Small))
                    .text_color(theme.muted_foreground)
                    .child(t!("settings-company")),
            )
    }
}

/// Asks for an image file and stores its path as the profile's picture or background.
fn choose_profile_image(settings: Entity<AppSettings>, avatar: bool, cx: &mut App) {
    let receiver = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: None,
    });
    cx.spawn(async move |cx| {
        let Ok(Ok(Some(paths))) = receiver.await else {
            return;
        };
        let Some(path) = paths.into_iter().next() else {
            return;
        };
        let path = path.to_string_lossy().into_owned();
        settings.update(cx, |settings, cx| match avatar {
            true => settings.set_profile_avatar(path, cx),
            false => settings.set_profile_background(path, cx),
        });
    })
    .detach();
}
