use anyhow::Result;
use runtime::hal::{Servo, MAX_SERVOS, TorqueMode};
use cursive::views::{TextView, LinearLayout, DummyView, Panel, Dialog, EditView, SelectView};
use cursive::traits::*;
use std::sync::{Arc, Mutex};

fn main() -> Result<()> {
    let servo = Arc::new(Servo::new()?);

    // Enable continuous readout
    servo.enable_readout()?;

    let mut siv = cursive::default();

    // Create a layout for our servo data
    let mut layout = LinearLayout::vertical();

    // Add header
    let header = LinearLayout::horizontal()
        .child(TextView::new("ID").center().fixed_width(4))
        .child(TextView::new("Curr Pos").center().fixed_width(8))
        .child(TextView::new("Targ Pos").center().fixed_width(8))
        .child(TextView::new("Curr Spd").center().fixed_width(8))
        .child(TextView::new("Run Spd").center().fixed_width(8))
        .child(TextView::new("Load").center().fixed_width(8))
        .child(TextView::new("Torque").center().fixed_width(8))
        .child(TextView::new("Torq Lim").center().fixed_width(8))
        .child(TextView::new("Accel").center().fixed_width(8))
        .child(TextView::new("Volt").center().fixed_width(6))
        .child(TextView::new("Temp").center().fixed_width(6))
        .child(TextView::new("Curr").center().fixed_width(6))
        .child(TextView::new("Status").center().fixed_width(8))
        .child(TextView::new("Async").center().fixed_width(6))
        .child(TextView::new("Lock").center().fixed_width(6));
    layout.add_child(header);

    // Add rows for each servo
    for i in 0..MAX_SERVOS {
        let row = LinearLayout::horizontal()
            .child(TextView::new(format!("{:2}", i + 1)).center().with_name(format!("ID {}", i)).fixed_width(4))
            .child(TextView::new("----").center().with_name(format!("CurrPos {}", i)).fixed_width(8))
            .child(TextView::new("----").center().with_name(format!("TargPos {}", i)).fixed_width(8))
            .child(TextView::new("----").center().with_name(format!("CurrSpd {}", i)).fixed_width(8))
            .child(TextView::new("----").center().with_name(format!("RunSpd {}", i)).fixed_width(8))
            .child(TextView::new("----").center().with_name(format!("Load {}", i)).fixed_width(8))
            .child(TextView::new("----").center().with_name(format!("Torque {}", i)).fixed_width(8))
            .child(TextView::new("----").center().with_name(format!("TorqLim {}", i)).fixed_width(8))
            .child(TextView::new("----").center().with_name(format!("Accel {}", i)).fixed_width(8))
            .child(TextView::new("----").center().with_name(format!("Volt {}", i)).fixed_width(6))
            .child(TextView::new("----").center().with_name(format!("Temp {}", i)).fixed_width(6))
            .child(TextView::new("----").center().with_name(format!("Curr {}", i)).fixed_width(6))
            .child(TextView::new("----").center().with_name(format!("Status {}", i)).fixed_width(8))
            .child(TextView::new("----").center().with_name(format!("Async {}", i)).fixed_width(6))
            .child(TextView::new("----").center().with_name(format!("Lock {}", i)).fixed_width(6));
        layout.add_child(row.with_name(format!("servo_row_{}", i)));
        siv.call_on_name("ID 0", |view: &mut TextView| {
            view.set_content(">ID");
        });
    }

    // Add a dummy view to push the task count to the bottom
    layout.add_child(DummyView.full_height());

    // Add task run count at the bottom
    layout.add_child(
        Panel::new(TextView::new("Task Run Count: 0").with_name("Task Count"))
            .title("Statistics")
            .full_width()
    );

    // Add instructions
    layout.add_child(
        TextView::new("Use Up/Down to select servo, Enter to toggle torque, Q to quit")
            .center()
            .full_width()
    );

    siv.add_fullscreen_layer(layout);

    // Set up a timer to update the UI
    siv.set_fps(30);

    // Clone Arc for the callback
    let servo_clone = Arc::clone(&servo);

    // Add a variable to keep track of the selected servo
    let selected_servo = Arc::new(Mutex::new(0));

    siv.add_global_callback('q', |s| s.quit());

    // Modify Up and Down callbacks to wrap around
    let selected_servo_up = Arc::clone(&selected_servo);
    siv.add_global_callback(cursive::event::Event::Key(cursive::event::Key::Up), move |s| {
        let mut selected = selected_servo_up.lock().unwrap();
        *selected = (*selected + MAX_SERVOS - 1) % MAX_SERVOS;
        update_selected_row(s, *selected);
    });

    let selected_servo_down = Arc::clone(&selected_servo);
    siv.add_global_callback(cursive::event::Event::Key(cursive::event::Key::Down), move |s| {
        let mut selected = selected_servo_down.lock().unwrap();
        *selected = (*selected + 1) % MAX_SERVOS;
        update_selected_row(s, *selected);
    });

    // Modify Enter callback to open settings subwindow
    let servo_clone_enter = Arc::clone(&servo);
    let selected_servo_enter = Arc::clone(&selected_servo);
    siv.add_global_callback(cursive::event::Event::Key(cursive::event::Key::Enter), move |s| {
        let selected = *selected_servo_enter.lock().unwrap();
        let servo_id = selected as u8 + 1;
        open_servo_settings(s, servo_id, Arc::clone(&servo_clone_enter));
    });

    // Add a new global callback for 't' key
    let servo_clone_toggle = Arc::clone(&servo);
    let selected_servo_toggle = Arc::clone(&selected_servo);
    siv.add_global_callback('t', move |s| {
        let selected = *selected_servo_toggle.lock().unwrap();
        let servo_id = selected as u8 + 1;
        toggle_servo_torque(s, servo_id, Arc::clone(&servo_clone_toggle));
    });

    siv.set_global_callback(cursive::event::Event::Refresh, move |s| {
        match servo_clone.read_continuous() {
            Ok(data) => {
                for (i, servo_info) in data.servo.iter().enumerate() {
                    s.call_on_name(&format!("CurrPos {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:4}", servo_info.current_location));
                    });
                    s.call_on_name(&format!("TargPos {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:4}", servo_info.target_location));
                    });
                    s.call_on_name(&format!("CurrSpd {}", i), |view: &mut TextView| {
                        let speed = servo_info.current_speed as u16 & 0x7FFF; // Remove 15th bit
                        let sign = if servo_info.current_speed as u16 & 0x8000 != 0 { '-' } else { '+' };
                        view.set_content(format!("{}{:4}", sign, speed));
                    });
                    s.call_on_name(&format!("RunSpd {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:4}", servo_info.running_speed));
                    });
                    s.call_on_name(&format!("Load {}", i), |view: &mut TextView| {
                        let speed = servo_info.current_load as u16 & 0x3FF; // Remove 10th bit
                        let sign = if servo_info.current_load as u16 & 0x400 != 0 { '-' } else { '+' };
                        view.set_content(format!("{}{:4}", sign, speed));
                    });
                    s.call_on_name(&format!("Torque {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:4}", servo_info.torque_switch));
                    });
                    s.call_on_name(&format!("TorqLim {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:4}", servo_info.torque_limit));
                    });
                    s.call_on_name(&format!("Accel {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:4}", servo_info.acceleration));
                    });
                    s.call_on_name(&format!("Volt {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:2.1}V", servo_info.current_voltage as f32 / 10.0));
                    });
                    s.call_on_name(&format!("Temp {}", i), |view: &mut TextView| {
                        view.set_content(format!("{}°C", servo_info.current_temperature));
                    });
                    s.call_on_name(&format!("Curr {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:4}", servo_info.current_current));
                    });
                    s.call_on_name(&format!("Status {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:04X}", servo_info.servo_status));
                    });
                    s.call_on_name(&format!("Async {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:4}", servo_info.async_write_flag));
                    });
                    s.call_on_name(&format!("Lock {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:4}", servo_info.lock_mark));
                    });
                }
                s.call_on_name("Task Count", |view: &mut TextView| {
                    view.set_content(format!("Task Run Count: {}", data.task_run_count));
                });
            }
            Err(e) => {
                s.add_layer(Dialog::info(format!("Error reading servo data: {}", e)));
            }
        }
    });

    siv.run();

    Ok(())
}

fn update_selected_row(s: &mut cursive::Cursive, selected: usize) {
    for i in 0..MAX_SERVOS {
        s.call_on_name(&format!("ID {}", i), |view: &mut TextView| {
            view.set_content(format!("{:2}", i + 1));
        });
    }
    s.call_on_name(&format!("ID {}", selected), |view: &mut TextView| {
        view.set_content(format!(">{:2}", selected + 1));
    });
}

fn open_servo_settings(s: &mut cursive::Cursive, servo_id: u8, servo: Arc<Servo>) {
    let dialog = Dialog::new()
        .title(format!("Servo {} Settings", servo_id))
        .content(
            LinearLayout::vertical()
                .child(TextView::new("Position:"))
                .child(EditView::new().with_name("position"))
                .child(TextView::new("Speed:"))
                .child(EditView::new().with_name("speed"))
                .child(TextView::new("Torque:"))
                .child(SelectView::new()
                    .item("Enabled", Arc::new(TorqueMode::Enabled))
                    .item("Disabled", Arc::new(TorqueMode::Disabled))
                    .item("Stiff", Arc::new(TorqueMode::Stiff))
                    .with_name("torque"))
        )
        .button("Apply", move |s| {
            let position = s.call_on_name("position", |view: &mut EditView| {
                view.get_content().parse::<i16>().ok()
            }).unwrap();
            let speed = s.call_on_name("speed", |view: &mut EditView| {
                view.get_content().parse::<u16>().unwrap_or(0)
            }).unwrap();
            let torque_mode = s.call_on_name("torque", |view: &mut SelectView<Arc<TorqueMode>>| {
                view.selection().unwrap_or_else(|| Arc::new(TorqueMode::Enabled.into()))
            }).unwrap();

            // Apply settings
            if let Err(e) = servo.set_torque_mode(servo_id, (**torque_mode).clone()) {
                s.add_layer(Dialog::info(format!("Error setting torque mode: {}", e)));
            }

            // Move servo only if position is provided
            if let Some(pos) = position {
                if let Err(e) = servo.move_servo(servo_id, pos, 0, speed) {
                    s.add_layer(Dialog::info(format!("Error moving servo: {}", e)));
                }
            }

            s.pop_layer();
        })
        .button("Cancel", |s| {
            s.pop_layer();
        });

    s.add_layer(dialog);
}

fn toggle_servo_torque(s: &mut cursive::Cursive, servo_id: u8, servo: Arc<Servo>) {
    let servo_clone = Arc::clone(&servo);
    
    match servo_clone.read_info(servo_id) {
        Ok(info) => {
            let new_torque_mode = if info.torque_switch == 0 {
                TorqueMode::Enabled
            } else {
                TorqueMode::Disabled
            };
            
            if let Err(e) = servo_clone.set_torque_mode(servo_id, new_torque_mode) {
                s.add_layer(Dialog::info(format!("Error setting torque mode: {}", e)));
            }
        }
        Err(e) => {
            s.add_layer(Dialog::info(format!("Error reading servo info: {}", e)));
        }
    }
}