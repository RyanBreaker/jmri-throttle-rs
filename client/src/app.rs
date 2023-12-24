use eframe::egui::{Context, Vec2, Widget};
use eframe::{egui, Frame};
use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use jmri_throttle_rs::message::{Function, Velocity, WiMessage, WiMessageType};
use log::{error, info, warn};
use std::collections::HashSet;

struct WsConnection {
    ws_sender: WsSender,
    ws_receiver: WsReceiver,
}

pub struct App {
    url: String,
    value: Velocity,
    functions: HashSet<Function>,
    connection: Option<WsConnection>,
}

impl App {
    fn connect(&mut self, ctx: &Context) {
        let ctx = ctx.clone();
        let wakeup = move || ctx.request_repaint();
        match ewebsock::connect_with_wakeup("ws://localhost:4000/ws", wakeup) {
            Ok((ws_sender, ws_receiver)) => {
                info!("Connected!");
                self.connection = Some(WsConnection {
                    ws_sender,
                    ws_receiver,
                })
            }
            Err(e) => {
                error!("Failed to connect to {}: {e}", self.url)
            }
        };
    }

    fn disconnect(&mut self) {
        if let Some(ref mut conn) = self.connection {
            conn.ws_sender.close().unwrap();
            self.connection = None;
        }
    }

    fn event(&mut self, _ctx: &Context) {
        if self.connection.is_none() {
            return;
        }
        let connection = self.connection.as_mut().unwrap();

        let mut messages = Vec::new();
        while let Some(event) = connection.ws_receiver.try_recv() {
            info!("Event: {event:?}");
            match event {
                WsEvent::Opened => info!("Connection opened."),
                WsEvent::Message(message) => match message {
                    WsMessage::Text(message) => match serde_json::from_str::<WiMessage>(&message) {
                        Ok(message) => messages.push(message),
                        Err(e) => error!("Failed to parse message: {e}"),
                    },
                    unknown => error!("Unknown WsMessage: {unknown:?}"),
                },
                WsEvent::Error(e) => error!("WS error: {e}"),
                WsEvent::Closed => warn!("Connection closed."),
            }
        }
        messages.iter().for_each(|m| self.handle_message(m));
    }

    fn handle_message(&mut self, message: &WiMessage) {
        use WiMessageType::*;
        info!("Handling message: {message}");
        match message.message_type {
            AddAddress => {}
            RemoveAddress => {}
            Velocity(v) => self.value = v,
            FunctionPressed(f) => {
                self.functions.insert(f);
            }
            FunctionReleased(f) => {
                self.functions.remove(&f);
            }
            Direction(_) => {}
            Time(_) => {}
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            url: "ws://localhost:4000/ws".into(),
            value: 0,
            functions: HashSet::new(),
            connection: None,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                ui.menu_button("Add...", |ui| ui.button("Throttle"));
                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.connection.is_some() {
                if ui.button("Disconnect").clicked() {
                    info!("Disconnect");
                    self.disconnect();
                }
            } else if ui.button("Connect").clicked() {
                info!("Connect");
                self.connect(ctx);
            }

            if let Some(ref mut connection) = self.connection {
                if ui.button("Add 6733").clicked() {
                    let message = WiMessage {
                        message_type: WiMessageType::AddAddress,
                        address: 6733,
                    };
                    let message = serde_json::to_string(&message).unwrap();
                    connection.ws_sender.send(WsMessage::Text(message));
                }
            }

            ui.heading("Throttles");

            let slider = egui::Slider::new(&mut self.value, 0..=126)
                .text("Speed")
                .integer()
                .vertical();
            if slider.ui(ui).changed() {
                let message = WiMessage {
                    message_type: WiMessageType::Velocity(self.value),
                    address: 6733,
                };
                let message = serde_json::to_string(&message).unwrap();
                if let Some(connection) = self.connection.as_mut() {
                    connection.ws_sender.send(WsMessage::Text(message));
                }
            }

            ui.horizontal_wrapped(|ui| {
                for i in 0..=24 {
                    let button = egui::Button::new(format!("F{i}"))
                        .min_size(Vec2::new(100f32, 100f32))
                        .selected(self.functions.contains(&i));
                    if button.ui(ui).clicked() {
                        if let Some(ref mut connection) = self.connection {
                            let message = WiMessage {
                                message_type: WiMessageType::FunctionPressed(i),
                                address: 6733,
                            };
                            let message = WsMessage::Text(serde_json::to_string(&message).unwrap());
                            connection.ws_sender.send(message);
                        }
                    }
                }
            });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });

        if self.connection.is_some() {
            self.event(ctx);
        }
    }
}
