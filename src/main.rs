use device_query::{DeviceEvents, DeviceQuery, DeviceState, Keycode, MouseButton};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

struct MouseClickEvent {
    relative_time: Duration,
    position: (i32, i32),
    button: MouseButton,
}

fn main() {
    let device_state = Arc::new(DeviceState::new());
    let mouse_position = Arc::new(Mutex::new((0, 0)));
    let is_recoding: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let last_start_recoding_time = Arc::new(Mutex::new(Instant::now()));
    let mouse_click_events = Arc::new(Mutex::new(Vec::<MouseClickEvent>::new()));

    let mouse_pos_for_closure = mouse_position.clone();
    let _guard = device_state.on_mouse_move(move |position| {
        let mut pos = mouse_pos_for_closure.lock().unwrap();
        *pos = *position;
        // println!("Position: {:#?}", position);
    });

    let is_recoding_for_closure = is_recoding.clone();
    let last_start_recoding_time_for_closure = last_start_recoding_time.clone();
    let mouse_pos_for_closure = mouse_position.clone();
    let mouse_click_events_for_closure = mouse_click_events.clone();
    let _guard = device_state.on_mouse_down(move |button| {
        if !*is_recoding_for_closure.lock().unwrap() {
            return;
        }
        let mut mouse_click_events = mouse_click_events_for_closure.lock().unwrap();
        let last_start_recoding_time = last_start_recoding_time_for_closure.lock().unwrap();
        let pos = mouse_pos_for_closure.lock().unwrap();
        mouse_click_events.push(MouseClickEvent {
            relative_time: last_start_recoding_time.elapsed(),
            position: *pos,
            button: *button,
        });
    });

    let device_state_for_keyboard: Arc<DeviceState> = device_state.clone();
    let is_recoding_for_closure = is_recoding.clone();
    let last_start_recoding_time_for_closure = last_start_recoding_time.clone();
    let mouse_click_events_for_closure = mouse_click_events.clone();
    let _guard = device_state.on_key_down(move |key| {
        // println!("Keyboard key down: {:#?}", key);
        // when key is c or v
        if !(*key == Keycode::C || *key == Keycode::V) {
            return;
        }
        let keys = device_state_for_keyboard.get_keys();
        if keys.contains(&Keycode::LControl)
            && keys.contains(&Keycode::LShift)
            && keys.contains(&Keycode::LAlt)
        {
            let mut is_recoding = is_recoding_for_closure.lock().unwrap();
            if *key == Keycode::C {
                *is_recoding = !*is_recoding;
                println!("recording: {}", *is_recoding);
                if *is_recoding {
                    let mut last_start_recoding_time =
                        last_start_recoding_time_for_closure.lock().unwrap();
                    *last_start_recoding_time = Instant::now();
                    let mut mouse_click_events = mouse_click_events_for_closure.lock().unwrap();
                    mouse_click_events.clear();
                }
            } else if *key == Keycode::V {
                if *is_recoding {
                    println!("recording is not finished");
                    return;
                }
                // print click events
                let mouse_click_events = mouse_click_events_for_closure.lock().unwrap();
                for mouse_click_event in mouse_click_events.iter() {
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
    });

    loop {
        thread::sleep(Duration::from_secs(1000));
    }
}
