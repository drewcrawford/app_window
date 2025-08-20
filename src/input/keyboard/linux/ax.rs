// SPDX-License-Identifier: MPL-2.0
/*
Currently screenreaders have no access to keyboard/mouse events on wayland.  For more details see
[this comment](https://github.com/AccessKit/accesskit/discussions/503#discussioncomment-11862133).

Accordingly at present each application has to send our events across the ATSPI device.  It seems that
nobody likes this solution due to privacy concerns so it is mostly being dropped and so it is neither
well-maintained nor well-understood or well-used and now accessibility is widely broken on Linux.

Well it is slightly less broken here.  I have cobbled together this implementation based on
a) tracing through the old GNOME projects, particularly
orca, at-spi2-core, and at-aspi2-apk, and gtk3 code.  (gtk4 code just ignores this whole problem
and shockingly, is not accessible on Linux!)  And b) the following snooping script, which shows
the events that are sent from other applications like Gnome Help for reverse-engineering:

```python
#!/usr/bin/env python3
import gi
gi.require_version('Atspi', '2.0')
gi.require_version('Gdk', '3.0')
from gi.repository import Atspi
from gi.repository import GLib
from gi.repository import Gdk

class KeyWatcher:
    def __init__(self):
        self.device = None
        self.mainloop = GLib.MainLoop()

    def start(self):
        """Initialize and start the key watcher."""
        print("Starting key watcher... Press keys to see events (Ctrl+C to exit)")
        self.device = Atspi.Device.new()

        # Add our callback function as the key event listener
        self.device.add_key_watcher(self.on_key_event)

        try:
            # Start the main event loop
            self.mainloop.run()
        except KeyboardInterrupt:
            print("\nStopping key watcher...")
        finally:
            self.stop()

    def stop(self):
        """Clean up and stop the key watcher."""
        self.device = None
        self.mainloop.quit()

    def on_key_event(self, device, pressed, keycode, keysym, modifiers, text):
        """Callback function that handles key events."""
        # Log all raw parameters
        print("=== Raw Event Data ===")
        print(f"device: {device!r}")
        print(f"pressed: {pressed!r}")
        print(f"keycode: {keycode!r}")
        print(f"keysym: {keysym!r}")
        print(f"modifiers: {modifiers!r} (0b{bin(modifiers)[2:]})")
        print(f"text: {text!r}")

        # Log modifier flags
        print("\n=== Modifier Flags ===")
        for mod_type in dir(Atspi.ModifierType):
            if not mod_type.startswith('_'):
                try:
                    mod_value = getattr(Atspi.ModifierType, mod_type)
                    if isinstance(mod_value, int):
                        is_set = bool(modifiers & (1 << mod_value))
                        print(f"{mod_type}: {is_set} (bit {mod_value})")
                except Exception as e:
                    print(f"{mod_type}: <error: {e}>")

        # Log key name from Gdk
        print("\n=== Gdk Key Info ===")
        key_name = Gdk.keyval_name(keysym)
        print(f"Gdk.keyval_name(keysym): {key_name!r}")

        # Get all possible key names for the keycode
        keymap = Gdk.Keymap.get_default()
        result = keymap.get_entries_for_keycode(keycode)
        if result[-1]:  # The keyvals are in the last element of the tuple
            print(f"All possible keyvals for keycode {keycode}:")
            for keyval in result[-1]:
                if keyval:  # Filter out None values
                    name = Gdk.keyval_name(keyval)
                    print(f"  {keyval}: {name!r}")

        print("-" * 50)
        return False  # Allow event propagation

def main():
    watcher = KeyWatcher()
    watcher.start()

if __name__ == "__main__":
    main()
```
 */

use crate::input::keyboard::key::KeyboardKey;
use ampsc::{ChannelConsumer, ChannelProducer};
use atspi::events::mouse::ButtonEvent;
use atspi::proxy::device_event_controller::{DeviceEvent, DeviceEventControllerProxy, EventType};
use some_executor::hint::Hint;
use some_executor::task::{Configuration, Task};
use some_executor::{Priority, SomeExecutor};
use std::sync::OnceLock;
use std::time::Instant;

mod helpers;
mod keycode;
mod keyname;
mod keysym;

use helpers::{key_is_text_input, key_to_modifier};
use keycode::key_to_x11;
use keyname::key_to_name;
use keysym::key_to_id;

static ONCE_SENDER: OnceLock<ChannelProducer<Event>> = OnceLock::new();

enum Event {
    Key(KeyboardKey, bool),
    Mouse(),
}

async fn ax_loop(mut receiver: ChannelConsumer<Event>) {
    let connection = atspi::AccessibilityConnection::new().await;
    let connection = match connection {
        Ok(c) => c,
        Err(e) => {
            logwise::error_async!(
                "Failed to connect to ATSPI: {e}",
                e = logwise::privacy::LogIt(e)
            );
            return;
        }
    };

    let start_time = Instant::now();
    let device = DeviceEventControllerProxy::new(connection.connection())
        .await
        .expect("No device event controller proxy");

    let mut modifiers: i32 = 0;

    loop {
        let event = receiver.receive().await.expect("No event");
        match event {
            Event::Key(key, pressed) => {
                let event_type = if pressed {
                    EventType::KeyPressed
                } else {
                    EventType::KeyReleased
                };
                let is_lock = key == KeyboardKey::CapsLock || key == KeyboardKey::NumLock;
                //is_lock keys are toggled on/off on RELEASE, not on press.
                //is_lock keys toggle on BEFORE the event and toggle off AFTER the event is sent!
                let late_toggle_off;
                if is_lock {
                    if !pressed {
                        if key_to_modifier(key) & modifiers == 0 {
                            //toggle on!
                            modifiers |= key_to_modifier(key);
                            late_toggle_off = false;
                        } else {
                            late_toggle_off = true;
                        }
                    } else {
                        late_toggle_off = false;
                    }
                } else {
                    late_toggle_off = false;
                }
                let is_numlock_enabled = modifiers & key_to_modifier(KeyboardKey::NumLock) != 0;
                //this struct loosely corresponds to ATK KeyEventStruct
                //https://docs.gtk.org/atk/struct.KeyEventStruct.html
                //which is the only documentation I can find but I have found some differences:
                let device_event = DeviceEvent {
                    //pressed or released
                    event_type,
                    //atk calls this 'keyval' and says 'representing a keysym value corresponding to those used by GDK and X11: see /usr/X11/include/keysymdef.h.'
                    id: key_to_id(key, is_numlock_enabled),
                    /*atk calls this 'keycode' and says 'The raw hardware code that generated the key event. This field is raraly [sic] useful.'

                    In fact it is used extensively  by orca and is not a hardware code but an X11 keycode, which is I guess
                    different from the keysym value, for more information see the function definition
                     */
                    hw_code: key_to_x11(key),
                    /*
                    atk calls this 'state' and says 'A bitmask representing the state of the modifier keys immediately after the event takes place. The meaning of the bits is currently defined to match the bitmask used by GDK in GdkEventType.state,
                    see http://developer.gnome.org/doc/API/2.0/gdk/gdk-Event-Structures.html#GdkEventKey.'

                    However by snooping, I have found this field reflects usually the values before and not after,
                    although it's complicated.
                     */
                    modifiers,
                    timestamp: start_time.elapsed().as_millis() as i32,
                    event_string: key_to_name(key, is_numlock_enabled),
                    is_text: key_is_text_input(key),
                };
                device
                    .notify_listeners_sync(&device_event)
                    .await
                    .expect("Failed to notify listeners");
                //update our modifiers AFTER sending event
                if is_lock {
                    if late_toggle_off {
                        modifiers &= !key_to_modifier(key);
                    }
                } else {
                    //standard key handling
                    if pressed {
                        modifiers |= key_to_modifier(key);
                    } else {
                        modifiers &= !key_to_modifier(key);
                    }
                }
            }
            Event::Mouse() => {
                let event = ButtonEvent {
                    item: Default::default(),
                    detail: "".to_string(),
                    mouse_x: 0,
                    mouse_y: 0,
                };
                connection
                    .send_event(event)
                    .await
                    .expect("Can't send event");
            }
        }
    }
}

fn ax_init() -> ChannelProducer<Event> {
    ONCE_SENDER
        .get_or_init(|| {
            let (sender, receiver) = ampsc::channel();

            let mut ex = some_executor::current_executor::current_executor();
            let t = Task::without_notifications(
                "linux ax".to_string(),
                Configuration::new(Hint::IO, Priority::UserInteractive, Instant::now()),
                ax_loop(receiver),
            )
            .into_objsafe();
            let o = ex.spawn_objsafe(t);
            std::mem::forget(o);
            sender
        })
        .clone()
}

pub fn ax_press(key: KeyboardKey, pressed: bool) {
    let sender = ax_init();
    let mut ex = some_executor::current_executor::current_executor();
    let t = Task::without_notifications(
        "linux ax".to_string(),
        Configuration::new(Hint::IO, Priority::UserInteractive, Instant::now()),
        async move {
            let mut sender = sender;
            sender
                .send(Event::Key(key, pressed))
                .await
                .expect("Failed to send event");
            sender.async_drop().await;
        },
    )
    .into_objsafe();
    let o = ex.spawn_objsafe(t);
    std::mem::forget(o);
}

pub fn ax_mouse() {
    let sender = ax_init();
    let mut ex = some_executor::current_executor::current_executor();
    let t = Task::without_notifications(
        "linux ax".to_string(),
        Configuration::new(Hint::IO, Priority::UserInteractive, Instant::now()),
        async move {
            let mut sender = sender;
            sender
                .send(Event::Mouse())
                .await
                .expect("Failed to send event");
            sender.async_drop().await;
        },
    )
    .into_objsafe();
    let o = ex.spawn_objsafe(t);
    std::mem::forget(o);
}
