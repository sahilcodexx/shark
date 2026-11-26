use crate::theme::{self, ButtonLook, ThemeColors, theme_button};
use gpui::{
    App, AppContext, Context, EventEmitter, FocusHandle, Focusable, Hsla, IntoElement,
    PromptButton, PromptHandle, PromptLevel, PromptResponse, Render, RenderablePromptHandle,
    Window, div, prelude::*, px,
};

struct SharkPrompt {
    message: String,
    detail: Option<String>,
    buttons: Vec<PromptButton>,
    destructive: bool,
    focus: FocusHandle,
    theme: ThemeColors,
}

impl EventEmitter<PromptResponse> for SharkPrompt {}

impl Focusable for SharkPrompt {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

pub(crate) fn render_prompt(
    level: PromptLevel,
    message: &str,
    detail: Option<&str>,
    buttons: &[PromptButton],
    handle: PromptHandle,
    window: &mut Window,
    cx: &mut App,
) -> RenderablePromptHandle {
    let config = crate::config::Config::load();
    let mode = match config.theme.as_str() {
        "light" => theme::ThemeMode::Light,
        "dark" => theme::ThemeMode::Dark,
        _ => theme::ThemeMode::System,
    };
    let theme = ThemeColors::resolve(mode, window, &config.background_mode);
    let view = cx.new(|cx| SharkPrompt {
        message: message.into(),
        detail: detail.map(str::to_owned),
        buttons: buttons.to_vec(),
        destructive: level == PromptLevel::Warning && buttons.len() > 1,
        focus: cx.focus_handle(),
        theme,
    });
    handle.with_view(view, window, cx)
}

impl Render for SharkPrompt {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let cancel = self.buttons.len().saturating_sub(1);
        div()
            .id("shark-prompt")
            .size_full()
            .absolute()
            .inset_0()
            .occlude()
            .track_focus(&self.focus)
            .flex()
            .items_center()
            .justify_center()
            .p_4()
            .bg(gpui::rgba(0x00000073))
            .on_key_down(cx.listener(move |_, event: &gpui::KeyDownEvent, _, cx| {
                match event.keystroke.key.as_str() {
                    "escape" => cx.emit(PromptResponse(cancel)),
                    "enter" => cx.emit(PromptResponse(0)),
                    _ => {}
                }
                cx.stop_propagation();
            }))
            .child(
                div()
                    .w(px(400.))
                    .max_w_full()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .rounded(px(14.))
                    .border_1()
                    .border_color(theme.surface_border)
                    .bg(Hsla {
                        a: 1.,
                        ..theme.card
                    })
                    .shadow(theme::floating_panel_shadows(&theme))
                    .text_color(theme.text_primary)
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(self.message.clone()),
                    )
                    .children(self.detail.clone().map(|detail| {
                        div()
                            .text_size(px(13.))
                            .text_color(theme.text_muted)
                            .child(detail)
                    }))
                    .child(
                        div().flex().justify_end().gap_2().children(
                            self.buttons
                                .iter()
                                .enumerate()
                                .rev()
                                .map(|(index, button)| {
                                    let label = if button.label().eq_ignore_ascii_case("ok") {
                                        "Close".into()
                                    } else {
                                        button.label().clone()
                                    };
                                    theme_button(
                                        index,
                                        if index == 0 && self.destructive {
                                            ButtonLook::Destructive
                                        } else if index == 0 {
                                            ButtonLook::Primary
                                        } else {
                                            ButtonLook::Secondary
                                        },
                                        &theme,
                                    )
                                    .child(label)
                                    .on_click(cx.listener(
                                        move |_, _, _, cx| {
                                            cx.emit(PromptResponse(index));
                                            cx.stop_propagation();
                                        },
                                    ))
                                }),
                        ),
                    ),
            )
    }
}
