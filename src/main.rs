use device_query::{DeviceEvents, DeviceQuery, DeviceState, Keycode};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use enigo::*;

#[derive(Clone)]
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
    should_interrupt_replay: bool,
}

impl MouseRecorder {
    fn new() -> Self {
        MouseRecorder {
            mouse_click_events: Vec::new(),
            mouse_position: (0, 0),
            is_recording: false,
            last_event_time: Instant::now(),
            should_interrupt_replay: false,
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

    fn get_mouse_click_events(&mut self) -> Vec<MouseClickEvent> {
        self.mouse_click_events.clone()
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

#[derive(Debug)]
enum DeviceEvent {
    MouseMove((i32, i32)),
    MouseDown(usize),
    KeyDown(Keycode),
}

struct ReplayEvent {
    events: Vec<MouseClickEvent>,
}

struct LastEnigoEvent {
    position: (i32, i32),
    ts: Instant,
}

impl LastEnigoEvent {
    fn new() -> Self {
        LastEnigoEvent {
            position: (-1, -1),
            ts: Instant::now(),
        }
    }

    fn update(&mut self, position: (i32, i32)) {
        self.position = position;
        self.ts = Instant::now();
    }
}

fn main() {
    let device_state = Arc::new(DeviceState::new());
    let recorder = Arc::new(Mutex::new(MouseRecorder::new()));
    let (tx, rx) = mpsc::channel::<DeviceEvent>();
    let (replay_tx, replay_rx) = mpsc::channel::<ReplayEvent>();

    let last_enigo_event = Arc::new(Mutex::new(LastEnigoEvent::new()));
    let device_state_for_listener_thread = device_state.clone();

    let _listener_thread = thread::spawn(move || {
        let device_state = device_state_for_listener_thread.clone();
        let tx = tx.clone();
        let _guard = {
            let tx = tx.clone();
            device_state.on_mouse_move(move |position| {
                tx.send(DeviceEvent::MouseMove(*position)).unwrap();
            })
        };

        let _guard = {
            let tx = tx.clone();
            device_state.on_mouse_down(move |button| {
                tx.send(DeviceEvent::MouseDown(*button)).unwrap();
            })
        };

        let _guard = {
            let tx = tx.clone();
            device_state.on_key_down(move |key| {
                tx.send(DeviceEvent::KeyDown(*key)).unwrap();
            })
        };

        loop {
            thread::sleep(Duration::from_secs(1000));
        }
    });


    let recorder_for_replay_thread = recorder.clone();
    let last_enigo_event_for_replay_thread = last_enigo_event.clone();
    let _replay_thread = thread::spawn(move || {
        let mut enigo: Enigo = Enigo::new();
        loop {
            let replay_event = replay_rx.recv().unwrap();
            {
                let mut recorder = recorder_for_replay_thread.lock().unwrap();
                recorder.should_interrupt_replay = false;
            }
            for click_event in replay_event.events {
                thread::sleep(click_event.relative_time);
                {
                    let recorder = recorder_for_replay_thread.lock().unwrap();
                    if recorder.should_interrupt_replay {
                        println!("replay interrupted");
                        break;
                    }
                }
                last_enigo_event_for_replay_thread.lock().unwrap().update(click_event.position);
                enigo.mouse_move_to(click_event.position.0, click_event.position.1);
                thread::sleep(Duration::from_millis(10));
                enigo.mouse_down(click_event.button);
                thread::sleep(Duration::from_millis(10));
                enigo.mouse_up(click_event.button);
            }
        }
    });

    loop {
        let event = rx.recv().unwrap();
        match event {
            DeviceEvent::MouseMove(position) => {
                let last_enigo_event = last_enigo_event.lock().unwrap();
                if last_enigo_event.ts.elapsed() < Duration::from_millis(100) && last_enigo_event.position == position {
                    continue;
                }
                recorder.lock().unwrap().should_interrupt_replay = true;
                handle_mouse_move(recorder.clone(), position);
            }
            DeviceEvent::MouseDown(button) => {
                let last_enigo_event = last_enigo_event.lock().unwrap();
                if last_enigo_event.ts.elapsed() < Duration::from_millis(100) {
                    continue;
                }
                recorder.lock().unwrap().should_interrupt_replay = true;
                handle_mouse_down(recorder.clone(), match button {
                    1 => MouseButton::Left,
                    2 => MouseButton::Right,
                    3 => MouseButton::Middle,
                    _ => panic!("impossible"),
                });
            }
            DeviceEvent::KeyDown(key) => {
                let last_enigo_event = last_enigo_event.lock().unwrap();
                if last_enigo_event.ts.elapsed() < Duration::from_millis(100) {
                    continue;
                }
                recorder.lock().unwrap().should_interrupt_replay = true;
                handle_key_down(device_state.clone(), recorder.clone(), key, replay_tx.clone());
            }
        }
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
    tx: mpsc::Sender<ReplayEvent>,
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
            tx.send(ReplayEvent {
                events: recorder.get_mouse_click_events(),
            }).unwrap();
        }
    }
}
