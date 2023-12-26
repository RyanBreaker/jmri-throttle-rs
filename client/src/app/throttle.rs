use crate::app::WsConnection;
use eframe::egui;
use eframe::egui::Vec2;
use jmri_throttle_rs::message::{Address, Direction, Function, Velocity, WiMessage, WiMessageType};
use std::collections::HashSet;

static BUTTON_SIZE: Vec2 = Vec2::new(50.0, 50.0);

pub struct Throttle {
    pub velocity: Velocity,
    pub address: Address,
    pub functions: HashSet<Function>,
    pub direction: Direction,
    // connection: Connection,
}

impl Throttle {
    pub fn new(address: Address) -> Throttle {
        Self {
            address,
            velocity: 0,
            functions: HashSet::new(),
            direction: Direction::default(),
            // connection,
        }
    }

    fn message(&self, message_type: WiMessageType) -> WiMessage {
        WiMessage::new(self.address, message_type)
    }

    pub fn draw(&mut self, connection: &mut WsConnection, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Slider::new(&mut self.velocity, 0..=126)
                        .vertical()
                        .integer()
                        .trailing_fill(true),
                )
                .changed()
            {
                connection.send(self.message(WiMessageType::Velocity(self.velocity)));
            }

            egui::Grid::new(format!("{}ThrottleColumns", self.address))
                .num_columns(2)
                // .spacing([40.0, 4.0])
                .show(ui, |ui| {
                    if ui.button("+1").clicked() {
                        connection.send(self.message(WiMessageType::Velocity(self.velocity + 1)));
                    }
                    if ui.button("-1").clicked() {
                        connection.send(self.message(WiMessageType::Velocity(self.velocity - 1)));
                    }
                    ui.end_row();
                    if ui.button("+10").clicked() {
                        connection.send(self.message(WiMessageType::Velocity(self.velocity + 10)));
                    }
                    if ui.button("-10").clicked() {
                        connection.send(self.message(WiMessageType::Velocity(self.velocity - 10)));
                    }
                    ui.end_row();
                    if ui.button("Stop").clicked() {
                        connection.send(self.message(WiMessageType::Velocity(0)));
                    }
                    if ui.button("E-stop").clicked() {
                        connection.send(self.message(WiMessageType::Velocity(-1)));
                    }
                    ui.end_row();
                });

            if ui
                .selectable_value(&mut self.direction, Direction::Forward, "Forward")
                .clicked()
            {
                connection.send(self.message(WiMessageType::Direction(Direction::Forward)));
            }
            if ui
                .selectable_value(&mut self.direction, Direction::Reverse, "Reverse")
                .clicked()
            {
                connection.send(self.message(WiMessageType::Direction(Direction::Reverse)));
            }
        });

        ui.separator();

        ui.horizontal_wrapped(|ui| {
            for f in 0..=28 {
                if ui
                    .add(
                        egui::Button::new(format!("F{f}"))
                            .min_size([50.0, 50.0].into())
                            .selected(self.functions.contains(&f)),
                    )
                    .clicked()
                {
                    connection.send(self.message(WiMessageType::FunctionPressed(f)))
                }
            }
        });
    }
}
