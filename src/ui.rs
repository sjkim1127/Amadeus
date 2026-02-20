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

#[derive(Resource, Default)]
pub struct ChatState {
    pub input_text: String,
    pub history: Vec<Message>,
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

    // Set a sleek dark theme
    style.visuals.window_fill = egui::Color32::from_rgba_premultiplied(15, 15, 18, 230);
    style.visuals.window_rounding = egui::Rounding::same(8.0);
    style.visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 55));
    style.visuals.panel_fill = egui::Color32::from_rgba_premultiplied(15, 15, 18, 230);

    ctx.set_style(style);
}

fn chat_ui_system(
    mut contexts: EguiContexts,
    mut chat_state: ResMut<ChatState>,
    channel: Option<Res<ChatChannel>>,
) {
    if let Some(chan) = &channel {
        if let Ok(mut rx) = chan.rx.try_lock() {
            while let Ok(msg) = rx.try_recv() {
                chat_state.history.push(msg);
            }
        }
    }

    egui::Window::new("Amadeus System")
        .default_width(380.0)
        .default_height(550.0)
        .resizable(true)
        .collapsible(true)
        .title_bar(true)
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(20.0, 20.0))
        .show(contexts.ctx_mut(), |ui| {
            ui.add_space(5.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .max_height(ui.available_height() - 45.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for msg in &chat_state.history {
                        let is_user = msg.role == "user";

                        ui.horizontal_wrapped(|ui| {
                            if is_user {
                                ui.label(
                                    egui::RichText::new("Guest ❯")
                                        .color(egui::Color32::from_rgb(100, 180, 255))
                                        .strong(),
                                );
                            } else {
                                ui.label(
                                    egui::RichText::new("Amadeus ❯")
                                        .color(egui::Color32::from_rgb(255, 80, 80))
                                        .strong(),
                                );
                            }

                            let text_color = if is_user {
                                egui::Color32::from_rgb(220, 230, 255)
                            } else {
                                egui::Color32::from_rgb(255, 230, 230)
                            };

                            ui.label(
                                egui::RichText::new(&msg.content)
                                    .color(text_color)
                                    .size(14.0),
                            );
                        });
                        ui.add_space(8.0);
                    }
                });

            ui.add_space(5.0);
            ui.separator();
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                let text_edit = egui::TextEdit::singleline(&mut chat_state.input_text)
                    .hint_text("Type your message to Amadeus...")
                    .desired_width(ui.available_width() - 65.0)
                    .margin(egui::vec2(8.0, 6.0));

                let response = ui.add(text_edit);

                let send_btn = ui.add_sized([55.0, 25.0], egui::Button::new("Send"));

                if send_btn.clicked()
                    || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                {
                    let text = chat_state.input_text.trim().to_string();
                    if !text.is_empty() {
                        if let Some(chan) = &channel {
                            let _ = chan.tx.send(text.clone());
                            chat_state.history.push(Message {
                                role: "user".to_string(),
                                content: text,
                                images: None,
                            });
                        }
                        chat_state.input_text.clear();
                        response.request_focus();
                    }
                }
            });
        });
}
