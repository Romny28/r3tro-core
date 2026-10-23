use gpui::prelude::*;
use gpui::{
    App, Bounds, Context, FontWeight, Window, WindowBounds, WindowHandle, WindowOptions, div, px,
    rgb, size, svg,
};

const ACCENT: u32 = 0x3DF5C2;
const GROUND: u32 = 0x121519;
const BORDER: u32 = 0x1E252C;
const TEXT: u32 = 0xE8ECEF;
const MUTED: u32 = 0x8A949E;
const VERSION: &str = "0.1";

pub struct Splash;

impl Render for Splash {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .gap(px(22.))
            .px(px(28.))
            .bg(rgb(GROUND))
            .border_1()
            .border_color(rgb(BORDER))
            .text_color(rgb(TEXT))
            .child(
                svg()
                    .path(crate::assets::LOGO)
                    .size(px(130.))
                    .flex_none()
                    .text_color(rgb(ACCENT)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .gap(px(10.))
                            .text_size(px(34.))
                            .child(div().font_weight(FontWeight::MEDIUM).child("R3TRØ"))
                            .child(div().font_weight(FontWeight::BOLD).child("CORE")),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(ACCENT))
                            .child(format!("VERSIÓN {VERSION}")),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(MUTED))
                            .child("Iniciando… · Basado en Sonora · GPL-3.0"),
                    ),
            )
    }
}

pub fn show(cx: &mut App) -> Option<WindowHandle<Splash>> {
    let bounds = Bounds::centered(None, size(px(460.), px(270.)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            is_movable: false,
            is_resizable: false,
            ..Default::default()
        },
        |_window, cx| cx.new(|_cx| Splash),
    )
    .ok()
}

pub fn close(handle: &WindowHandle<Splash>, cx: &mut App) {
    handle
        .update(cx, |_, window, _| window.remove_window())
        .ok();
}
