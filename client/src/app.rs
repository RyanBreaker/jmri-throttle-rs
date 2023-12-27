mod throttle;

use crate::app::throttle::Throttle;
use chrono::NaiveDateTime;
use eframe::egui::{Align, Button, Context, Layout};
use eframe::{egui, Frame, Storage};
use egui::{Grid, TextEdit, Ui, Window};
use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use jmri_throttle_rs::message::{Address, WiMessage, WiMessageType};
use log::{error, info, warn};
use std::borrow::BorrowMut;
use std::collections::HashMap;
use uuid::Uuid;

pub struct WsConnection {
    pub ws_sender: WsSender,
    pub ws_receiver: WsReceiver,
}

impl WsConnection {
    pub fn send(&mut self, message: WiMessage) {
        let message = serde_json::to_string(&message).unwrap();
        self.ws_sender.send(WsMessage::Text(message));
    }
}

#[derive(Default, Debug)]
struct State {
    pub show_connect: bool,
    pub show_new_throttle: bool,
    pub new_address: String,
    pub connecting: bool,
}

pub struct App {
    uuid: Uuid,
    url: String,
    throttles: HashMap<Address, Throttle>,
    connection: Option<WsConnection>,
    time: i64,
    state: State,
}

impl App {
    #[allow(dead_code)]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut uuid: Option<Uuid> = None;
        if let Some(storage) = cc.storage {
            if let Some(state) = eframe::get_value(storage, eframe::APP_KEY) {
                uuid = state;
            }
        }
        Self {
            uuid: uuid.unwrap_or_else(Uuid::new_v4),
            url: "localhost:4000/ws".to_string(),
            connection: None,
            time: 0,
            throttles: Default::default(),
            state: State::default(),
        }
    }

    fn connect(&mut self, ctx: &Context) {
        let ctx = ctx.clone();
        let wakeup = move || ctx.request_repaint();
        match ewebsock::connect_with_wakeup(format!("ws://{}", self.url), wakeup) {
            Ok((ws_sender, ws_receiver)) => {
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
        self.throttles.clear();
        self.connection = None;
        self.state.connecting = false;
        self.state.show_connect = false;
    }

    fn handle_messages(&mut self, _ctx: &Context) {
        if self.connection.is_none() {
            return;
        }
        let connection = self.connection.as_mut().unwrap();
        let mut closed = false;

        let mut messages = Vec::new();
        while let Some(event) = connection.ws_receiver.try_recv() {
            info!("Event: {event:?}");
            match event {
                WsEvent::Opened => {
                    info!("Connection opened");
                    self.state.connecting = false;
                    self.state.show_connect = false;
                }
                WsEvent::Message(message) => match message {
                    WsMessage::Text(message) => match serde_json::from_str::<WiMessage>(&message) {
                        Ok(message) => messages.push(message),
                        Err(e) => error!("Failed to parse message: {e}"),
                    },
                    unknown => error!("Unknown WsMessage: {unknown:?}"),
                },
                WsEvent::Error(e) => error!("WS error: {e}"),
                WsEvent::Closed => {
                    warn!("Connection closed.");
                    closed = true;
                    break;
                }
            }
        }
        if closed {
            self.disconnect();
        }
        messages.iter().for_each(|m| self.handle_message(m));
    }

    fn handle_message(&mut self, message: &WiMessage) {
        use WiMessageType::*;
        if let Time(t) = message.message_type {
            self.time = t;
            return;
        }
        if let Some(throttle) = self.throttles.get_mut(&message.address) {
            match message.message_type {
                AddAddress => {}
                RemoveAddress => {
                    self.throttles.remove(&message.address);
                }
                Velocity(v) => throttle.velocity = v,
                FunctionPressed(f) => {
                    throttle.functions.insert(f);
                }
                FunctionReleased(f) => {
                    throttle.functions.remove(&f);
                }
                Direction(d) => throttle.direction = d,
                Time(t) => self.time = t,
            }
        }
    }

    fn menu_bar(&mut self, ui: &mut Ui) {
        egui::menu::bar(ui, |ui| {
            egui::widgets::global_dark_light_mode_switch(ui);

            if self.connection.is_none() {
                if ui.button("Connect").clicked() {
                    self.state.show_connect = true;
                }
            } else if ui.button("Disconnect").clicked() {
                self.disconnect();
            }

            if self.connection.is_some() && !self.state.connecting {
                ui.separator();
                if ui
                    .add(Button::new("New Throttle").selected(self.state.show_new_throttle))
                    .clicked()
                {
                    self.state.show_new_throttle = !self.state.show_new_throttle;
                }
            }

            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                let dt = NaiveDateTime::from_timestamp_opt(self.time, 0).unwrap();
                ui.label(format!("Fast Clock: {}", dt.format("%l:%M %p")));
            });
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| self.menu_bar(ui));

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.state.show_connect {
                Window::new("Connect")
                    .max_width(250f32)
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        Grid::new("ConnectGrid").num_columns(2).show(ui, |ui| {
                            ui.label("URL:");
                            ui.add_enabled(
                                !self.state.connecting,
                                TextEdit::singleline(&mut self.url),
                            );
                        });
                        ui.add_space(15.0);
                        ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                            if self.state.connecting {
                                ui.spinner();
                            } else if ui.button("Connect").clicked() {
                                self.state.connecting = true;
                                self.connect(ctx);
                            }
                            if ui.button("Cancel").clicked() {
                                self.disconnect();
                            }
                        });
                    });
            } else if self.state.show_new_throttle {
                Window::new("New Throttle")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Address:");
                        TextEdit::singleline(&mut self.state.new_address).show(ui);
                        ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                            if ui.button("Add").clicked() {
                                if let Ok(address) = self.state.new_address.parse::<Address>() {
                                    let sender = &mut self.connection.as_mut().unwrap().ws_sender;
                                    let message =
                                        WiMessage::new(address, WiMessageType::AddAddress);
                                    let message = serde_json::to_string(&message).unwrap();
                                    sender.send(WsMessage::Text(message));

                                    // TODO: Confirm to add when we get a response from the server
                                    self.throttles.insert(address, Throttle::new(address));

                                    self.state.show_new_throttle = false;
                                    self.state.new_address = String::new();
                                } else {
                                    info!("Cannot parse address: {}", self.state.new_address);
                                }
                            }
                            if ui.button("Cancel").clicked() {
                                self.state.new_address = String::default();
                                self.state.show_new_throttle = false;
                            }
                        });
                    });
            }

            ui.heading("Throttles");

            if let Some(connection) = self.connection.borrow_mut() {
                for throttle in self.throttles.values_mut() {
                    Window::new(throttle.address.to_string())
                        .fixed_size([340.0, 400.0])
                        .show(ctx, |ui| {
                            throttle.draw(connection, ui);
                        });
                }
            }

            ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });

        self.handle_messages(ctx);
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.uuid);
    }
}
