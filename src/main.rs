use device_query::{DeviceEvents, DeviceQuery, DeviceState, Keycode};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use enigo::*;

struct MouseClickEvent {
    relative_time: Duration,
    position: (i32, i32),
    button: MouseButton,
}

struct MouseRecorder {
    mouse_click_events: Vec<MouseClickEvent>,
    mouse_position: (i32, i32),
    is_recording: bool,
    last_event_time: Instant,
}

impl MouseRecorder {
    fn new() -> Self {
        MouseRecorder {
            mouse_click_events: Vec::new(),
            mouse_position: (0, 0),
            is_recording: false,
            last_event_time: Instant::now(),
        }
    }

    fn start_recording(&mut self) {
        println!("start recording");
        self.mouse_click_events.clear();
        self.last_event_time = Instant::now();
        self.is_recording = true;
    }

    fn stop_recording(&mut self) {
        println!("stop recording");
        self.is_recording = false;
    }

    fn toggle_recording(&mut self) {
        if self.is_recording {
            self.stop_recording();
        } else {
            self.start_recording();
        }
    }

    fn get_mouse_click_events(&self) -> &Vec<MouseClickEvent> {
        &self.mouse_click_events
    }

    fn record_mouse_click_event(&mut self, button: MouseButton) {
        self.mouse_click_events.push(MouseClickEvent {
            relative_time: if self.mouse_click_events.is_empty() {
                Duration::from_secs(0)
            } else {
                self.last_event_time.elapsed()
            },
            position: self.mouse_position,
            button,
        });
        self.last_event_time = Instant::now();
    }
}

fn main() {
    let device_state = Arc::new(DeviceState::new());
    let recorder = Arc::new(Mutex::new(MouseRecorder::new()));

    let _guard = {
        let recorder: Arc<Mutex<MouseRecorder>> = recorder.clone();
        device_state.on_mouse_move(move |position| {
            handle_mouse_move(recorder.clone(), *position);
        })
    };

    let _guard = {
        let recorder: Arc<Mutex<MouseRecorder>> = recorder.clone();

        device_state.on_mouse_down(move |button| {
            if *button != 1 && *button != 2 && *button != 3 {
                return;
            }
            handle_mouse_down(recorder.clone(), match *button {
                1 => MouseButton::Left,
                2 => MouseButton::Right,
                3 => MouseButton::Middle,
                _ => panic!("impossible"),
            });
        })
    };

    let _guard = {
        let recorder: Arc<Mutex<MouseRecorder>> = recorder.clone();
        let device_state_for_closure = device_state.clone();
        device_state.on_key_down(move |key| {
            handle_key_down(device_state_for_closure.clone(), recorder.clone(), *key);
        })
    };

    loop {
        thread::sleep(Duration::from_secs(1000));
    }
}

fn handle_mouse_move(recorder: Arc<Mutex<MouseRecorder>>, position: (i32, i32)) {
    let mut recorder = recorder.lock().unwrap();
    recorder.mouse_position = position;
}

fn handle_mouse_down(recorder: Arc<Mutex<MouseRecorder>>, button: MouseButton) {
    let mut recorder = recorder.lock().unwrap();
    if !recorder.is_recording {
        return;
    }
    recorder.record_mouse_click_event(button);
}

fn handle_key_down(
    device_state: Arc<DeviceState>,
    recorder: Arc<Mutex<MouseRecorder>>,
    key: Keycode,
) {
    if key != Keycode::C && key != Keycode::X {
        return;
    }
    let keys = device_state.get_keys();
    if keys.contains(&Keycode::LControl)
        && keys.contains(&Keycode::LShift)
        && keys.contains(&Keycode::LAlt)
    {
        let mut recorder = recorder.lock().unwrap();
        if key == Keycode::C {
            recorder.toggle_recording();
        } else if key == Keycode::X {
            if recorder.is_recording {
                println!("recording is not finished");
                return;
            }
            let click_events = recorder.get_mouse_click_events();
            // use enigo to replay mouse click events
            let mut enigo = Enigo::new();
            let mut last_pos = (-1, -1);
            for click_event in click_events {
                thread::sleep(click_event.relative_time);
                let cur_pos: (i32, i32) = device_state.get_mouse().coords;
                if cur_pos != last_pos && last_pos != (-1, -1) {
                    println!("mouse moved, stop replaying");
                    return;
                }
                enigo.mouse_move_to(click_event.position.0, click_event.position.1);
                last_pos = click_event.position;
                enigo.mouse_down(click_event.button);
                enigo.mouse_up(click_event.button);
            }
        }
    }
}
