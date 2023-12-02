use device_query::{DeviceEvents, DeviceQuery, DeviceState, Keycode, MouseButton};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

struct MouseClickEvent {
    relative_time: Duration,
    position: (i32, i32),
    button: MouseButton,
}

struct MouseRecorder {
    mouse_click_events: Vec<MouseClickEvent>,
    mouse_position: (i32, i32),
    is_recording: bool,
    last_start_recording_time: Instant,
}

impl MouseRecorder {
    fn new() -> Self {
        MouseRecorder {
            mouse_click_events: Vec::new(),
            mouse_position: (0, 0),
            is_recording: false,
            last_start_recording_time: Instant::now(),
        }
    }

    fn start_recording(&mut self) {
        println!("start recording");
        self.mouse_click_events.clear();
        self.last_start_recording_time = Instant::now();
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
            relative_time: self.last_start_recording_time.elapsed(),
            position: self.mouse_position,
            button,
        });
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
            handle_mouse_down(recorder.clone(), *button);
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
    if key != Keycode::C && key != Keycode::V {
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
        } else if key == Keycode::V {
            if recorder.is_recording {
                println!("recording is not finished");
                return;
            }
            for mouse_click_event in recorder.get_mouse_click_events().iter() {
                println!(
                    "relative_time: {:#?}, position: ({},{}), button: {:#?}",
                    mouse_click_event.relative_time,
                    mouse_click_event.position.0,
                    mouse_click_event.position.1,
                    mouse_click_event.button
                );
            }
        }
    }
}