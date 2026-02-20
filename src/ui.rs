use crate::llm::local::Message;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use std::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Resource)]
pub struct ChatChannel {
    pub tx: UnboundedSender<String>,
    pub rx: Mutex<UnboundedReceiver<Message>>,
}

#[derive(Resource)]
pub struct ChatState {
    pub input_text: String,
    pub history: Vec<Message>,
    pub is_thinking: bool,
    pub show_settings: bool,
    pub tts_enabled: bool,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            input_text: String::new(),
            history: Vec::new(),
            is_thinking: false,
            show_settings: false,
            tts_enabled: true,
        }
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .init_resource::<ChatState>()
            .add_systems(Startup, configure_egui)
            .add_systems(Update, chat_ui_system);
    }
}

fn configure_egui(mut contexts: EguiContexts) {
    let ctx = contexts.ctx_mut();
    let mut style = (*ctx.style()).clone();

    style.visuals.window_fill = egui::Color32::from_rgba_premultiplied(15, 15, 18, 230);
    style.visuals.window_rounding = egui::Rounding::same(8.0);
    style.visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 55));
    style.visuals.panel_fill = egui::Color32::from_rgba_premultiplied(15, 15, 18, 230);
    style.spacing.item_spacing = egui::vec2(8.0, 4.0);

    ctx.set_style(style);
}

/// Lightweight markdown-ish renderer using egui RichText.
/// Handles: **bold**, `inline code`, ```code blocks```, and bullet lists.
fn render_markdown(ui: &mut egui::Ui, text: &str) {
    let mut in_code_block = false;
    let mut code_block_content = String::new();

    for line in text.lines() {
        if line.starts_with("```") {
            if in_code_block {
                // End of code block — render accumulated code
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(30, 30, 38))
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::same(6.0))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(&code_block_content)
                                .color(egui::Color32::from_rgb(180, 220, 180))
                                .monospace()
                                .size(12.5),
                        );
                    });
                code_block_content.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            if !code_block_content.is_empty() {
                code_block_content.push('\n');
            }
            code_block_content.push_str(line);
            continue;
        }

        // Bullet list
        if line.starts_with("- ") || line.starts_with("* ") {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new("  •")
                        .color(egui::Color32::from_rgb(255, 120, 120))
                        .size(13.0),
                );
                render_inline_markdown(ui, &line[2..]);
            });
        } else if line.is_empty() {
            ui.add_space(4.0);
        } else {
            ui.horizontal_wrapped(|ui| {
                render_inline_markdown(ui, line);
            });
        }
    }

    // Handle unclosed code block
    if in_code_block && !code_block_content.is_empty() {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(30, 30, 38))
            .rounding(egui::Rounding::same(4.0))
            .inner_margin(egui::Margin::same(6.0))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(&code_block_content)
                        .color(egui::Color32::from_rgb(180, 220, 180))
                        .monospace()
                        .size(12.5),
                );
            });
    }
}

/// Render inline markdown: **bold** and `code`
fn render_inline_markdown(ui: &mut egui::Ui, text: &str) {
    let text_color = egui::Color32::from_rgb(255, 230, 230);
    let code_color = egui::Color32::from_rgb(180, 220, 180);
    let code_bg = egui::Color32::from_rgb(35, 35, 45);

    let mut remaining = text;
    while !remaining.is_empty() {
        // Check for **bold**
        if let Some(start) = remaining.find("**") {
            if start > 0 {
                ui.label(
                    egui::RichText::new(&remaining[..start])
                        .color(text_color)
                        .size(14.0),
                );
            }
            let after_start = &remaining[start + 2..];
            if let Some(end) = after_start.find("**") {
                ui.label(
                    egui::RichText::new(&after_start[..end])
                        .color(text_color)
                        .strong()
                        .size(14.0),
                );
                remaining = &after_start[end + 2..];
                continue;
            }
        }

        // Check for `inline code`
        if let Some(start) = remaining.find('`') {
            if start > 0 {
                ui.label(
                    egui::RichText::new(&remaining[..start])
                        .color(text_color)
                        .size(14.0),
                );
            }
            let after_start = &remaining[start + 1..];
            if let Some(end) = after_start.find('`') {
                ui.label(
                    egui::RichText::new(&after_start[..end])
                        .color(code_color)
                        .background_color(code_bg)
                        .monospace()
                        .size(13.0),
                );
                remaining = &after_start[end + 1..];
                continue;
            }
        }

        // Plain text — no more markdown markers
        ui.label(egui::RichText::new(remaining).color(text_color).size(14.0));
        break;
    }
}

fn chat_ui_system(
    mut contexts: EguiContexts,
    mut chat_state: ResMut<ChatState>,
    channel: Option<Res<ChatChannel>>,
) {
    // Receive incoming messages from Agent Core
    if let Some(chan) = &channel {
        if let Ok(mut rx) = chan.rx.try_lock() {
            while let Ok(msg) = rx.try_recv() {
                if msg.role != "user" {
                    chat_state.is_thinking = false;
                }
                chat_state.history.push(msg);
            }
        }
    }

    egui::Window::new("Amadeus System")
        .default_width(420.0)
        .default_height(580.0)
        .resizable(true)
        .collapsible(true)
        .title_bar(true)
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(20.0, 20.0))
        .show(contexts.ctx_mut(), |ui| {
            // ===== ⑤ Toolbar =====
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new("🗑 Clear").small())
                    .on_hover_text("Clear conversation history")
                    .clicked()
                {
                    chat_state.history.clear();
                    chat_state.is_thinking = false;
                    if let Some(chan) = &channel {
                        let _ = chan.tx.send("__CLEAR__".to_string());
                    }
                }

                ui.separator();

                if ui
                    .add(
                        egui::Button::new(if chat_state.show_settings {
                            "⚙ ▼"
                        } else {
                            "⚙ ▶"
                        })
                        .small(),
                    )
                    .on_hover_text("Settings")
                    .clicked()
                {
                    chat_state.show_settings = !chat_state.show_settings;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let status_color = if chat_state.is_thinking {
                        egui::Color32::from_rgb(255, 200, 50)
                    } else {
                        egui::Color32::from_rgb(80, 200, 80)
                    };
                    ui.label(egui::RichText::new("●").color(status_color).size(10.0));
                    ui.label(
                        egui::RichText::new(if chat_state.is_thinking {
                            "Thinking"
                        } else {
                            "Online"
                        })
                        .color(egui::Color32::from_rgb(140, 140, 150))
                        .size(11.0),
                    );
                });
            });

            // ===== Settings Panel =====
            if chat_state.show_settings {
                ui.add_space(4.0);
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_premultiplied(25, 25, 30, 200))
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Settings")
                                .color(egui::Color32::from_rgb(180, 180, 190))
                                .size(12.0)
                                .strong(),
                        );
                        ui.checkbox(&mut chat_state.tts_enabled, "🔊 Voice Output (TTS)");
                    });
            }

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // ===== Chat History =====
            let input_area_height = 70.0;
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .max_height(ui.available_height() - input_area_height)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    let history_snapshot: Vec<Message> = chat_state.history.clone();

                    for msg in history_snapshot.iter() {
                        match msg.role.as_str() {
                            "user" => {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        egui::RichText::new("Guest ❯")
                                            .color(egui::Color32::from_rgb(100, 180, 255))
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(&msg.content)
                                            .color(egui::Color32::from_rgb(220, 230, 255))
                                            .size(14.0),
                                    );
                                });
                            }
                            "system" => {
                                // ④ System/Tool messages
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        egui::RichText::new("⚙ System")
                                            .color(egui::Color32::from_rgb(120, 120, 140))
                                            .italics()
                                            .size(12.0),
                                    );
                                    ui.label(
                                        egui::RichText::new(&msg.content)
                                            .color(egui::Color32::from_rgb(140, 140, 160))
                                            .italics()
                                            .size(12.0),
                                    );
                                });
                            }
                            _ => {
                                // Assistant: ① Markdown rendering
                                ui.label(
                                    egui::RichText::new("Amadeus ❯")
                                        .color(egui::Color32::from_rgb(255, 80, 80))
                                        .strong(),
                                );
                                render_markdown(ui, &msg.content);
                            }
                        }
                        ui.add_space(6.0);
                    }

                    // ② Typing indicator
                    if chat_state.is_thinking {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(
                                egui::RichText::new("Amadeus가 생각 중...")
                                    .color(egui::Color32::from_rgb(200, 160, 160))
                                    .italics()
                                    .size(13.0),
                            );
                        });
                    }
                });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // ===== ③ Multiline Input =====
            let available_w = ui.available_width();
            ui.horizontal_top(|ui| {
                let text_edit = egui::TextEdit::multiline(&mut chat_state.input_text)
                    .hint_text("메시지를 입력하세요... (Shift+Enter로 줄바꿈)")
                    .desired_width(available_w - 65.0)
                    .desired_rows(1)
                    .lock_focus(true);

                let response = ui.add(text_edit);

                let send_btn = ui.add_sized(
                    [55.0, 30.0],
                    egui::Button::new(egui::RichText::new("Send").strong()),
                );

                // Enter = send, Shift+Enter = newline
                let enter_pressed = response.has_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);

                if send_btn.clicked() || enter_pressed {
                    let text = chat_state.input_text.trim().to_string();
                    if !text.is_empty() && !chat_state.is_thinking {
                        if let Some(chan) = &channel {
                            let _ = chan.tx.send(text.clone());
                            chat_state.history.push(Message {
                                role: "user".to_string(),
                                content: text,
                                images: None,
                            });
                            chat_state.is_thinking = true;
                        }
                        chat_state.input_text.clear();
                        response.request_focus();
                    }
                }
            });
        });
}
