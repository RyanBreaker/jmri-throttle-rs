use crate::app::WsConnection;
use eframe::egui;
use eframe::egui::{Button, Ui, Vec2};
use jmri_throttle_rs::message::{Address, Direction, Function, Velocity, WiMessage, WiMessageType};
use std::collections::HashSet;

static BUTTON_SIZE: Vec2 = Vec2::new(50.0, 50.0);

pub struct Throttle {
    pub velocity: Velocity,
    pub address: Address,
    pub functions: HashSet<Function>,
    pub direction: Direction,
}

impl Throttle {
    pub fn new(address: Address) -> Throttle {
        Self {
            address,
            velocity: 0,
            functions: HashSet::new(),
            direction: Direction::default(),
        }
    }

    fn message(&self, message_type: WiMessageType) -> WiMessage {
        WiMessage::new(self.address, message_type)
    }

    fn adjust_velocity(&mut self, delta: Velocity, connection: &mut WsConnection) {
        self.velocity += delta;
        connection.send(self.message(WiMessageType::Velocity(self.velocity)));
    }

    pub fn draw(&mut self, connection: &mut WsConnection, ui: &mut Ui) {
        ui.add_space(15.0);
        ui.horizontal_top(|ui| {
            ui.add_space(15.0);
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

            ui.add_space(15.0);
            egui::Grid::new(format!("{}ThrottleColumns", self.address))
                .num_columns(2)
                .spacing([10.0, 15.0])
                .show(ui, |ui| {
                    if ui.add(Button::new("+1").min_size(BUTTON_SIZE)).clicked() {
                        self.adjust_velocity(1, connection);
                    }
                    if ui.add(Button::new("-1").min_size(BUTTON_SIZE)).clicked() {
                        self.adjust_velocity(-1, connection);
                    }
                    ui.end_row();
                    if ui.add(Button::new("+10").min_size(BUTTON_SIZE)).clicked() {
                        self.adjust_velocity(10, connection);
                    }
                    if ui.add(Button::new("-10").min_size(BUTTON_SIZE)).clicked() {
                        self.adjust_velocity(-10, connection);
                    }
                    ui.end_row();
                    if ui
                        .add(
                            Button::new("Stop")
                                .selected(self.velocity == 0)
                                .min_size(BUTTON_SIZE),
                        )
                        .clicked()
                    {
                        self.velocity = 0;
                        connection.send(self.message(WiMessageType::Velocity(0)));
                    }
                    if ui
                        .add(
                            Button::new("E-stop")
                                .selected(self.velocity < 0)
                                .min_size(BUTTON_SIZE),
                        )
                        .clicked()
                    {
                        self.velocity = -1;
                        connection.send(self.message(WiMessageType::Velocity(-1)));
                    }
                    ui.end_row();
                });

                ui.label("Direction:");
                    if ui
                        .selectable_value(&mut self.direction, Direction::Reverse, "Reverse")
                        .clicked()
                    {
                        connection.send(self.message(WiMessageType::Direction(Direction::Reverse)));
                    }
                    if ui
                        .selectable_value(&mut self.direction, Direction::Forward, "Forward")
                        .clicked()
                    {
                        connection.send(self.message(WiMessageType::Direction(Direction::Forward)));
                    }
        });

        ui.separator();

        ui.horizontal_wrapped(|ui| {
            for f in 0..=28 {
                if ui
                    .add(
                        Button::new(format!("F{f}"))
                            .min_size(BUTTON_SIZE)
                            .selected(self.functions.contains(&f)),
                    )
                    .clicked()
                {
                    connection.send(self.message(WiMessageType::FunctionPressed(f)))
                }
            }
        });

        ui.separator();

        if ui.button("Release").clicked() {
            connection.send(self.message(WiMessageType::RemoveAddress));
        }
    }
}
