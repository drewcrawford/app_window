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
                ax_loop(receiver),
                Configuration::new(Hint::IO, Priority::UserInteractive, Instant::now()),
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
        async move {
            let mut sender = sender;
            sender
                .send(Event::Key(key, pressed))
                .await
                .expect("Failed to send event");
            sender.async_drop().await;
        },
        Configuration::new(Hint::IO, Priority::UserInteractive, Instant::now()),
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
        async move {
            let mut sender = sender;
            sender
                .send(Event::Mouse())
                .await
                .expect("Failed to send event");
            sender.async_drop().await;
        },
        Configuration::new(Hint::IO, Priority::UserInteractive, Instant::now()),
    )
    .into_objsafe();
    let o = ex.spawn_objsafe(t);
    std::mem::forget(o);
}

//evidently based on /usr/X11/include/keysymdef.h
fn key_to_id(key: KeyboardKey, is_numlock_enabled: bool) -> i32 {
    match key {
        // Alphabet keys
        KeyboardKey::A => 0x0061, // XK_a
        KeyboardKey::B => 0x0062, // XK_b
        KeyboardKey::C => 0x0063, // XK_c
        KeyboardKey::D => 0x0064, // XK_d
        KeyboardKey::E => 0x0065, // XK_e
        KeyboardKey::F => 0x0066, // XK_f
        KeyboardKey::G => 0x0067, // XK_g
        KeyboardKey::H => 0x0068, // XK_h
        KeyboardKey::I => 0x0069, // XK_i
        KeyboardKey::J => 0x006a, // XK_j
        KeyboardKey::K => 0x006b, // XK_k
        KeyboardKey::L => 0x006c, // XK_l
        KeyboardKey::M => 0x006d, // XK_m
        KeyboardKey::N => 0x006e, // XK_n
        KeyboardKey::O => 0x006f, // XK_o
        KeyboardKey::P => 0x0070, // XK_p
        KeyboardKey::Q => 0x0071, // XK_q
        KeyboardKey::R => 0x0072, // XK_r
        KeyboardKey::S => 0x0073, // XK_s
        KeyboardKey::T => 0x0074, // XK_t
        KeyboardKey::U => 0x0075, // XK_u
        KeyboardKey::V => 0x0076, // XK_v
        KeyboardKey::W => 0x0077, // XK_w
        KeyboardKey::X => 0x0078, // XK_x
        KeyboardKey::Y => 0x0079, // XK_y
        KeyboardKey::Z => 0x007a, // XK_z

        // Number keys
        KeyboardKey::Num0 => 0x0030, // XK_0
        KeyboardKey::Num1 => 0x0031, // XK_1
        KeyboardKey::Num2 => 0x0032, // XK_2
        KeyboardKey::Num3 => 0x0033, // XK_3
        KeyboardKey::Num4 => 0x0034, // XK_4
        KeyboardKey::Num5 => 0x0035, // XK_5
        KeyboardKey::Num6 => 0x0036, // XK_6
        KeyboardKey::Num7 => 0x0037, // XK_7
        KeyboardKey::Num8 => 0x0038, // XK_8
        KeyboardKey::Num9 => 0x0039, // XK_9

        // Keypad
        KeyboardKey::Keypad0 => {
            if is_numlock_enabled {
                0xffb0
            } else {
                0xff95
            }
        } // XK_KP_0, XK_KP_HOME
        KeyboardKey::Keypad1 => {
            if is_numlock_enabled {
                0xffb1
            } else {
                0xff9c
            }
        } // XK_KP_1, XK_KP_END
        KeyboardKey::Keypad2 => {
            if is_numlock_enabled {
                0xffb2
            } else {
                0xff99
            }
        } // XK_KP_2, XK_KP_DOWN
        KeyboardKey::Keypad3 => {
            if is_numlock_enabled {
                0xffb3
            } else {
                0xff9b
            }
        } // XK_KP_3, XK_KP_PAGE_DOWN
        KeyboardKey::Keypad4 => {
            if is_numlock_enabled {
                0xffb4
            } else {
                0xff96
            }
        } // XK_KP_4, XK_KP_LEFT
        KeyboardKey::Keypad5 => {
            if is_numlock_enabled {
                0xffb5
            } else {
                0xff9d
            }
        } // XK_KP_5, XK_KP_BEGIN
        KeyboardKey::Keypad6 => {
            if is_numlock_enabled {
                0xffb6
            } else {
                0xff98
            }
        } // XK_KP_6, XK_KP_RIGHT
        KeyboardKey::Keypad7 => {
            if is_numlock_enabled {
                0xffb7
            } else {
                0xff97
            }
        } // XK_KP_7, XK_KP_UP
        KeyboardKey::Keypad8 => {
            if is_numlock_enabled {
                0xffb8
            } else {
                0xff9a
            }
        } // XK_KP_8, XK_KP_PAGE_UP
        KeyboardKey::Keypad9 => {
            if is_numlock_enabled {
                0xffb9
            } else {
                0xff9a
            }
        } // XK_KP_9, XK_KP_PRIOR
        KeyboardKey::KeypadDecimal => {
            if is_numlock_enabled {
                0xffae
            } else {
                0xff9f
            }
        } // XK_KP_Decimal, XK_KP_Delete
        KeyboardKey::KeypadMultiply => 0xffaa, // XK_KP_Multiply
        KeyboardKey::KeypadPlus => 0xffab,     // XK_KP_Add
        KeyboardKey::KeypadClear => 0xff0b,    // XK_Clear
        KeyboardKey::KeypadDivide => 0xffaf,   // XK_KP_Divide
        KeyboardKey::KeypadEnter => 0xff8d,    // XK_KP_Enter
        KeyboardKey::KeypadMinus => 0xffad,    // XK_KP_Subtract
        KeyboardKey::KeypadEquals => 0xffbd,   // XK_KP_Equal

        // Function keys
        KeyboardKey::F1 => 0xffbe,  // XK_F1
        KeyboardKey::F2 => 0xffbf,  // XK_F2
        KeyboardKey::F3 => 0xffc0,  // XK_F3
        KeyboardKey::F4 => 0xffc1,  // XK_F4
        KeyboardKey::F5 => 0xffc2,  // XK_F5
        KeyboardKey::F6 => 0xffc3,  // XK_F6
        KeyboardKey::F7 => 0xffc4,  // XK_F7
        KeyboardKey::F8 => 0xffc5,  // XK_F8
        KeyboardKey::F9 => 0xffc6,  // XK_F9
        KeyboardKey::F10 => 0xffc7, // XK_F10
        KeyboardKey::F11 => 0xffc8, // XK_F11
        KeyboardKey::F12 => 0xffc9, // XK_F12
        KeyboardKey::F13 => 0xffca, // XK_F13
        KeyboardKey::F14 => 0xffcb, // XK_F14
        KeyboardKey::F15 => 0xffcc, // XK_F15
        KeyboardKey::F16 => 0xffcd, // XK_F16
        KeyboardKey::F17 => 0xffce, // XK_F17
        KeyboardKey::F18 => 0xffcf, // XK_F18
        KeyboardKey::F19 => 0xffd0, // XK_F19
        KeyboardKey::F20 => 0xffd1, // XK_F20
        KeyboardKey::F21 => 0xffd2, // XK_F21
        KeyboardKey::F22 => 0xffd3, // XK_F22
        KeyboardKey::F23 => 0xffd4, // XK_F23
        KeyboardKey::F24 => 0xffd5, // XK_F24

        // Special characters
        KeyboardKey::Space => 0x0020,        // XK_space
        KeyboardKey::Minus => 0x002d,        // XK_minus
        KeyboardKey::Equal => 0x003d,        // XK_equal
        KeyboardKey::LeftBracket => 0x005b,  // XK_bracketleft
        KeyboardKey::RightBracket => 0x005d, // XK_bracketright
        KeyboardKey::Backslash => 0x005c,    // XK_backslash
        KeyboardKey::Semicolon => 0x003b,    // XK_semicolon
        KeyboardKey::Quote => 0x0027,        // XK_apostrophe
        KeyboardKey::Grave => 0x0060,        // XK_grave
        KeyboardKey::Comma => 0x002c,        // XK_comma
        KeyboardKey::Period => 0x002e,       // XK_period
        KeyboardKey::Slash => 0x002f,        // XK_slash

        // Control keys
        KeyboardKey::Return => 0xff0d,        // XK_Return
        KeyboardKey::Tab => 0xff09,           // XK_Tab
        KeyboardKey::Delete => 0xff08,        // XK_backspace
        KeyboardKey::ForwardDelete => 0xffff, // XK_Delete
        KeyboardKey::Escape => 0xff1b,        // XK_Escape
        KeyboardKey::Home => 0xff50,          // XK_Home
        KeyboardKey::PageUp => 0xff55,        // XK_Page_Up
        KeyboardKey::PageDown => 0xff56,      // XK_Page_Down
        KeyboardKey::End => 0xff57,           // XK_End
        KeyboardKey::Help => 0xff6a,          // XK_Help
        KeyboardKey::LeftArrow => 0xff51,     // XK_Left
        KeyboardKey::RightArrow => 0xff53,    // XK_Right
        KeyboardKey::DownArrow => 0xff54,     // XK_Down
        KeyboardKey::UpArrow => 0xff52,       // XK_Up

        // Modifier keys
        KeyboardKey::Shift => 0xffe1,        // XK_Shift_L
        KeyboardKey::RightShift => 0xffe2,   // XK_Shift_R
        KeyboardKey::Control => 0xffe3,      // XK_Control_L
        KeyboardKey::RightControl => 0xffe4, // XK_Control_R
        KeyboardKey::Option => 0xffe9,       // XK_Alt_L
        KeyboardKey::RightOption => 0xffea,  // XK_Alt_R
        KeyboardKey::Command => 0xffeb,      // XK_Super_l
        KeyboardKey::RightCommand => 0xffec, // XK_Super_R
        KeyboardKey::Function => 0xfd1e,     // Special function key
        KeyboardKey::CapsLock => 0xffe5,     // XK_Caps_Lock

        // Media keys
        KeyboardKey::VolumeUp => 0x1008ff13, // XF86XK_AudioRaiseVolume
        KeyboardKey::VolumeDown => 0x1008ff11, // XF86XK_AudioLowerVolume
        KeyboardKey::Mute => 0x1008ff12,     // XF86XK_AudioMute
        KeyboardKey::Play => 0x1008ff14,     // XF86XK_AudioPlay
        KeyboardKey::Stop => 0x1008ff15,     // XF86XK_AudioStop
        KeyboardKey::PreviousTrack => 0x1008ff16, // XF86XK_AudioPrev
        KeyboardKey::NextTrack => 0x1008ff17, // XF86XK_AudioNext
        KeyboardKey::Eject => 0x1008ff2c,    // XF86XK_Eject

        // Additional special keys
        KeyboardKey::PrintScreen => 0xff61, // XK_Print
        KeyboardKey::ScrollLock => 0xff14,  // XK_Scroll_Lock
        KeyboardKey::Pause => 0xff13,       // XK_Pause
        KeyboardKey::Insert => 0xff63,      // XK_Insert
        KeyboardKey::NumLock => 0xff7f,     // XK_Num_Lock
        KeyboardKey::ContextMenu => 0xff67, // XK_Menu
        KeyboardKey::Power => 0x1008ff2a,   // XF86XK_PowerOff

        // Browser keys
        KeyboardKey::BrowserBack => 0x1008ff26, // XF86XK_Back
        KeyboardKey::BrowserForward => 0x1008ff27, // XF86XK_Forward
        KeyboardKey::BrowserRefresh => 0x1008ff29, // XF86XK_Refresh
        KeyboardKey::BrowserStop => 0x1008ff28, // XF86XK_Stop
        KeyboardKey::BrowserSearch => 0x1008ff1b, // XF86XK_Search
        KeyboardKey::BrowserFavorites => 0x1008ff30, // XF86XK_Favorites
        KeyboardKey::BrowserHome => 0x1008ff18, // XF86XK_HomePage

        // Application keys
        KeyboardKey::LaunchMail => 0x1008ff19,  // XF86XK_Mail
        KeyboardKey::MediaSelect => 0x1008ff32, // XF86XK_AudioMedia
        KeyboardKey::LaunchApp1 => 0x1008ff1c,  // XF86XK_Launch0
        KeyboardKey::LaunchApp2 => 0x1008ff1d,  // XF86XK_Launch1

        // International keys
        KeyboardKey::JISKana => 0xff2d,              // XK_Kana_Lock
        KeyboardKey::JISEisu => 0xff2f,              // XK_Eisu_Shift
        KeyboardKey::JISYen => 0x0a5,                // XK_yen
        KeyboardKey::JISUnderscore => 0x5f,          // XK_underscore
        KeyboardKey::JISKeypadComma => 0xffac,       // XK_KP_Separator
        KeyboardKey::ISOSection => 0xa7,             // XK_section
        KeyboardKey::InternationalBackslash => 0x5c, // XK_backslash

        // Edit keys
        KeyboardKey::Again => 0xff66,  // XK_Redo
        KeyboardKey::Undo => 0xff65,   // XK_Undo
        KeyboardKey::Cut => 0xff63,    // XK_Cut
        KeyboardKey::Copy => 0xff62,   // XK_Copy
        KeyboardKey::Paste => 0xff63,  // XK_Paste
        KeyboardKey::Find => 0xff68,   // XK_Find
        KeyboardKey::Props => 0xff62,  // XK_Execute
        KeyboardKey::Select => 0xff60, // XK_Select
        KeyboardKey::Open => 0xff62,   // XK_Execute

        // Japanese input conversion
        KeyboardKey::Convert => 0xff21,    // XK_Convert
        KeyboardKey::NonConvert => 0xff22, // XK_NonConvert

        // System
        KeyboardKey::WakeUp => 0x1008ff2b,     // XF86XK_WakeUp
        KeyboardKey::ContextualMenu => 0xff67, // XK_Menu
    }
}

/*convert to x11 keycodes
taken from xmodmap -pke
*/
fn key_to_x11(key: KeyboardKey) -> i32 {
    match key {
        // Letters
        KeyboardKey::Q => 24,
        KeyboardKey::W => 25,
        KeyboardKey::E => 26,
        KeyboardKey::R => 27,
        KeyboardKey::T => 28,
        KeyboardKey::Y => 29,
        KeyboardKey::U => 30,
        KeyboardKey::I => 31,
        KeyboardKey::O => 32,
        KeyboardKey::P => 33,
        KeyboardKey::A => 38,
        KeyboardKey::S => 39,
        KeyboardKey::D => 40,
        KeyboardKey::F => 41,
        KeyboardKey::G => 42,
        KeyboardKey::H => 43,
        KeyboardKey::J => 44,
        KeyboardKey::K => 45,
        KeyboardKey::L => 46,
        KeyboardKey::Z => 52,
        KeyboardKey::X => 53,
        KeyboardKey::C => 54,
        KeyboardKey::V => 55,
        KeyboardKey::B => 56,
        KeyboardKey::N => 57,
        KeyboardKey::M => 58,

        // Numbers across the top
        KeyboardKey::Num1 => 10,
        KeyboardKey::Num2 => 11,
        KeyboardKey::Num3 => 12,
        KeyboardKey::Num4 => 13,
        KeyboardKey::Num5 => 14,
        KeyboardKey::Num6 => 15,
        KeyboardKey::Num7 => 16,
        KeyboardKey::Num8 => 17,
        KeyboardKey::Num9 => 18,
        KeyboardKey::Num0 => 19,

        // Function keys
        KeyboardKey::F1 => 67,
        KeyboardKey::F2 => 68,
        KeyboardKey::F3 => 69,
        KeyboardKey::F4 => 70,
        KeyboardKey::F5 => 71,
        KeyboardKey::F6 => 72,
        KeyboardKey::F7 => 73,
        KeyboardKey::F8 => 74,
        KeyboardKey::F9 => 75,
        KeyboardKey::F10 => 76,
        KeyboardKey::F11 => 95,
        KeyboardKey::F12 => 96,
        KeyboardKey::F13 => 191, // Not exact, using alternative mapping
        KeyboardKey::F14 => 192,
        KeyboardKey::F15 => 193,
        KeyboardKey::F16 => 194,
        KeyboardKey::F17 => 195,
        KeyboardKey::F18 => 196,
        KeyboardKey::F19 => 197,
        KeyboardKey::F20 => 198,
        KeyboardKey::F21 => 199,
        KeyboardKey::F22 => 200,
        KeyboardKey::F23 => 201,
        KeyboardKey::F24 => 202,

        // Special characters
        KeyboardKey::Minus => 20,
        KeyboardKey::Equal => 21,
        KeyboardKey::LeftBracket => 34,
        KeyboardKey::RightBracket => 35,
        KeyboardKey::Semicolon => 47,
        KeyboardKey::Quote => 48,
        KeyboardKey::Grave => 49,
        KeyboardKey::Backslash => 51,
        KeyboardKey::Comma => 59,
        KeyboardKey::Period => 60,
        KeyboardKey::Slash => 61,

        // Modifiers
        KeyboardKey::Shift => 50,
        KeyboardKey::RightShift => 62,
        KeyboardKey::Control => 37,
        KeyboardKey::RightControl => 105,
        KeyboardKey::Option => 64,        // Alt_L
        KeyboardKey::RightOption => 108,  // Alt_R
        KeyboardKey::Command => 133,      // Super_L
        KeyboardKey::RightCommand => 134, // Super_R
        KeyboardKey::Function => 135,     // Menu as fallback
        KeyboardKey::CapsLock => 66,

        // Navigation
        KeyboardKey::Return => 36,
        KeyboardKey::Tab => 23,
        KeyboardKey::Space => 65,
        KeyboardKey::Delete => 22,         // BackSpace
        KeyboardKey::ForwardDelete => 119, // Delete
        KeyboardKey::Escape => 9,
        KeyboardKey::Home => 110,
        KeyboardKey::PageUp => 112,
        KeyboardKey::PageDown => 117,
        KeyboardKey::End => 115,
        KeyboardKey::LeftArrow => 113,
        KeyboardKey::RightArrow => 114,
        KeyboardKey::DownArrow => 116,
        KeyboardKey::UpArrow => 111,

        // Keypad
        KeyboardKey::KeypadDecimal => 91,
        KeyboardKey::KeypadMultiply => 63,
        KeyboardKey::KeypadPlus => 86,
        KeyboardKey::KeypadClear => 91, // Using KP_Delete as equivalent
        KeyboardKey::KeypadDivide => 106,
        KeyboardKey::KeypadEnter => 104,
        KeyboardKey::KeypadMinus => 82,
        KeyboardKey::KeypadEquals => 125,
        KeyboardKey::Keypad0 => 90,
        KeyboardKey::Keypad1 => 87,
        KeyboardKey::Keypad2 => 88,
        KeyboardKey::Keypad3 => 89,
        KeyboardKey::Keypad4 => 83,
        KeyboardKey::Keypad5 => 84,
        KeyboardKey::Keypad6 => 85,
        KeyboardKey::Keypad7 => 79,
        KeyboardKey::Keypad8 => 80,
        KeyboardKey::Keypad9 => 81,

        // Lock keys
        KeyboardKey::NumLock => 77,
        KeyboardKey::ScrollLock => 78,

        // Media keys
        KeyboardKey::VolumeUp => 123,
        KeyboardKey::VolumeDown => 122,
        KeyboardKey::Mute => 121,
        KeyboardKey::Play => 172,
        KeyboardKey::Stop => 174,
        KeyboardKey::PreviousTrack => 173,
        KeyboardKey::NextTrack => 171,

        // System keys
        KeyboardKey::PrintScreen => 107,
        KeyboardKey::Pause => 127,
        KeyboardKey::Insert => 118,
        KeyboardKey::Power => 124,
        KeyboardKey::Eject => 169,

        // Browser/App keys
        KeyboardKey::BrowserBack => 166,
        KeyboardKey::BrowserForward => 167,
        KeyboardKey::BrowserRefresh => 181,
        KeyboardKey::BrowserStop => 174, // Using AudioStop as alternative
        KeyboardKey::BrowserSearch => 225,
        KeyboardKey::BrowserFavorites => 164,
        KeyboardKey::BrowserHome => 180,
        KeyboardKey::LaunchMail => 163,
        KeyboardKey::MediaSelect => 234,
        KeyboardKey::LaunchApp1 => 156,
        KeyboardKey::LaunchApp2 => 157,

        // Japanese input
        KeyboardKey::Convert => 100,      // Henkan_Mode
        KeyboardKey::NonConvert => 102,   // Muhenkan
        KeyboardKey::JISKana => 101,      // Using Hiragana_Katakana
        KeyboardKey::JISEisu => 98,       // Using Katakana as alternative
        KeyboardKey::JISYen => 94,        // Using less/greater as alternative
        KeyboardKey::JISUnderscore => 20, // Using minus as alternative
        KeyboardKey::JISKeypadComma => 129,

        // Extra keys
        KeyboardKey::Help => 146,
        KeyboardKey::Again => 137,
        KeyboardKey::Undo => 139,
        KeyboardKey::Cut => 145,
        KeyboardKey::Copy => 141,
        KeyboardKey::Paste => 143,
        KeyboardKey::Find => 144,
        KeyboardKey::Props => 138, // SunProps
        KeyboardKey::Select => 0,  // No direct mapping
        KeyboardKey::Open => 142,
        KeyboardKey::ContextMenu => 135,    // Menu
        KeyboardKey::ContextualMenu => 135, // Menu
        KeyboardKey::WakeUp => 151,

        // International
        KeyboardKey::ISOSection => 94, // Using less/greater as alternative
        KeyboardKey::InternationalBackslash => 94, // Using less/greater
    }
}

fn key_to_name(key: KeyboardKey, is_numlock_enabled: bool) -> &'static str {
    match key {
        // Letters - map to lowercase versions
        KeyboardKey::A => "a",
        KeyboardKey::B => "b",
        KeyboardKey::C => "c",
        KeyboardKey::D => "d",
        KeyboardKey::E => "e",
        KeyboardKey::F => "f",
        KeyboardKey::G => "g",
        KeyboardKey::H => "h",
        KeyboardKey::I => "i",
        KeyboardKey::J => "j",
        KeyboardKey::K => "k",
        KeyboardKey::L => "l",
        KeyboardKey::M => "m",
        KeyboardKey::N => "n",
        KeyboardKey::O => "o",
        KeyboardKey::P => "p",
        KeyboardKey::Q => "q",
        KeyboardKey::R => "r",
        KeyboardKey::S => "s",
        KeyboardKey::T => "t",
        KeyboardKey::U => "u",
        KeyboardKey::V => "v",
        KeyboardKey::W => "w",
        KeyboardKey::X => "x",
        KeyboardKey::Y => "y",
        KeyboardKey::Z => "z",

        // Numbers
        KeyboardKey::Num0 => "0",
        KeyboardKey::Num1 => "1",
        KeyboardKey::Num2 => "2",
        KeyboardKey::Num3 => "3",
        KeyboardKey::Num4 => "4",
        KeyboardKey::Num5 => "5",
        KeyboardKey::Num6 => "6",
        KeyboardKey::Num7 => "7",
        KeyboardKey::Num8 => "8",
        KeyboardKey::Num9 => "9",

        // Special characters
        KeyboardKey::Space => "space",
        KeyboardKey::Minus => "-",
        KeyboardKey::Equal => "=",
        KeyboardKey::LeftBracket => "[",
        KeyboardKey::RightBracket => "]",
        KeyboardKey::Backslash => "\\",
        KeyboardKey::Semicolon => ";",
        KeyboardKey::Quote => "'",
        KeyboardKey::Grave => "`",
        KeyboardKey::Comma => ",",
        KeyboardKey::Period => ".",
        KeyboardKey::Slash => "/",

        // Modifiers
        KeyboardKey::Shift => "Shift_L",
        KeyboardKey::RightShift => "Shift_R",
        KeyboardKey::Control => "Control_L",
        KeyboardKey::RightControl => "Control_R",
        KeyboardKey::Option => "Alt_L",
        KeyboardKey::RightOption => "Alt_R",
        KeyboardKey::Command => "Meta_L",
        KeyboardKey::RightCommand => "Meta_R",
        KeyboardKey::Function => "VoidSymbol", // No direct mapping
        KeyboardKey::CapsLock => "Caps_Lock",

        // Function keys
        KeyboardKey::F1 => "F1",
        KeyboardKey::F2 => "F2",
        KeyboardKey::F3 => "F3",
        KeyboardKey::F4 => "F4",
        KeyboardKey::F5 => "F5",
        KeyboardKey::F6 => "F6",
        KeyboardKey::F7 => "F7",
        KeyboardKey::F8 => "F8",
        KeyboardKey::F9 => "F9",
        KeyboardKey::F10 => "F10",
        KeyboardKey::F11 => "F11",
        KeyboardKey::F12 => "F12",
        KeyboardKey::F13 => "F13",
        KeyboardKey::F14 => "F14",
        KeyboardKey::F15 => "F15",
        KeyboardKey::F16 => "F16",
        KeyboardKey::F17 => "F17",
        KeyboardKey::F18 => "F18",
        KeyboardKey::F19 => "F19",
        KeyboardKey::F20 => "F20",
        KeyboardKey::F21 => "F21",
        KeyboardKey::F22 => "F22",
        KeyboardKey::F23 => "F23",
        KeyboardKey::F24 => "F24",

        // Navigation keys
        KeyboardKey::Return => "Return",
        KeyboardKey::Tab => "Tab",
        KeyboardKey::Delete => "BackSpace",
        KeyboardKey::ForwardDelete => "Delete",
        KeyboardKey::Escape => "Escape",
        KeyboardKey::Home => "Home",
        KeyboardKey::PageUp => "Page_Up",
        KeyboardKey::PageDown => "Page_Down",
        KeyboardKey::End => "End",
        KeyboardKey::LeftArrow => "Left",
        KeyboardKey::RightArrow => "Right",
        KeyboardKey::DownArrow => "Down",
        KeyboardKey::UpArrow => "Up",
        KeyboardKey::Help => "Help",

        // Keypad
        KeyboardKey::KeypadDecimal => {
            if is_numlock_enabled {
                "."
            } else {
                "KP_Delete"
            }
        }
        KeyboardKey::KeypadMultiply => "*",
        KeyboardKey::KeypadPlus => "+",
        KeyboardKey::KeypadClear => "Clear",
        KeyboardKey::KeypadDivide => "/",
        KeyboardKey::KeypadEnter => "KP_Enter",
        KeyboardKey::KeypadMinus => "-",
        KeyboardKey::KeypadEquals => "=",
        KeyboardKey::Keypad0 => {
            if is_numlock_enabled {
                "0"
            } else {
                "KP_Insert"
            }
        }
        KeyboardKey::Keypad1 => {
            if is_numlock_enabled {
                "1"
            } else {
                "KP_End"
            }
        }
        KeyboardKey::Keypad2 => {
            if is_numlock_enabled {
                "2"
            } else {
                "KP_Down"
            }
        }
        KeyboardKey::Keypad3 => {
            if is_numlock_enabled {
                "3"
            } else {
                "KP_Next"
            }
        } //Next is somewhat suspicious to me but matches Gnome Help
        KeyboardKey::Keypad4 => {
            if is_numlock_enabled {
                "4"
            } else {
                "KP_Left"
            }
        }
        KeyboardKey::Keypad5 => {
            if is_numlock_enabled {
                "5"
            } else {
                "KP_Begin"
            }
        }
        KeyboardKey::Keypad6 => {
            if is_numlock_enabled {
                "6"
            } else {
                "KP_Right"
            }
        }
        KeyboardKey::Keypad7 => {
            if is_numlock_enabled {
                "7"
            } else {
                "KP_Home"
            }
        }
        KeyboardKey::Keypad8 => {
            if is_numlock_enabled {
                "8"
            } else {
                "KP_Up"
            }
        }
        KeyboardKey::Keypad9 => {
            if is_numlock_enabled {
                "9"
            } else {
                "KP_Page_Up"
            }
        }

        // Lock keys
        KeyboardKey::NumLock => "Num_Lock",
        KeyboardKey::ScrollLock => "Scroll_Lock",

        // Misc keys
        KeyboardKey::PrintScreen => "Print",
        KeyboardKey::Pause => "Pause",
        KeyboardKey::Insert => "Insert",
        KeyboardKey::ContextMenu => "Menu",
        KeyboardKey::Power => "PowerOff",

        // Media keys
        KeyboardKey::VolumeUp => "AudioRaiseVolume",
        KeyboardKey::VolumeDown => "AudioLowerVolume",
        KeyboardKey::Mute => "AudioMute",
        KeyboardKey::Play => "AudioPlay",
        KeyboardKey::Stop => "AudioStop",
        KeyboardKey::PreviousTrack => "AudioPrev",
        KeyboardKey::NextTrack => "AudioNext",

        // Browser keys
        KeyboardKey::BrowserBack => "Back",
        KeyboardKey::BrowserForward => "Forward",
        KeyboardKey::BrowserRefresh => "Refresh",
        KeyboardKey::BrowserStop => "Stop",
        KeyboardKey::BrowserSearch => "Search",
        KeyboardKey::BrowserFavorites => "Favorites",
        KeyboardKey::BrowserHome => "HomePage",

        // App keys
        KeyboardKey::LaunchMail => "Mail",
        KeyboardKey::MediaSelect => "AudioMedia",
        KeyboardKey::LaunchApp1 => "Launch0",
        KeyboardKey::LaunchApp2 => "Launch1",

        // Japanese input keys
        KeyboardKey::Convert => "Henkan",
        KeyboardKey::NonConvert => "Muhenkan",
        KeyboardKey::JISKana => "Kana_Lock",
        KeyboardKey::JISEisu => "Eisu_toggle",
        KeyboardKey::JISYen => "yen",
        KeyboardKey::JISUnderscore => "underscore",
        KeyboardKey::JISKeypadComma => "KP_Separator",

        // The ISO section maps to section
        KeyboardKey::ISOSection => "section",

        // International
        KeyboardKey::InternationalBackslash => "backslash",

        // Standard editing keys
        KeyboardKey::Again => "Redo",
        KeyboardKey::Undo => "Undo",
        KeyboardKey::Cut => "Cut",
        KeyboardKey::Copy => "Copy",
        KeyboardKey::Paste => "Paste",
        KeyboardKey::Find => "Find",
        KeyboardKey::Props => "Execute",
        KeyboardKey::Select => "Select",
        KeyboardKey::Open => "Open",

        // Power/system
        KeyboardKey::WakeUp => "WakeUp",
        KeyboardKey::Eject => "Eject",
        KeyboardKey::ContextualMenu => "Menu",
    }
}

//https://gnome.pages.gitlab.gnome.org/at-spi2-core/libatspi/enum.ModifierType.html
fn key_to_modifier(key: KeyboardKey) -> i32 {
    match key {
        KeyboardKey::Shift | KeyboardKey::RightShift => 1 << 0,
        KeyboardKey::CapsLock => 1 << 1,
        KeyboardKey::Control | KeyboardKey::RightControl => 1 << 2,
        KeyboardKey::Option | KeyboardKey::RightOption => 1 << 3,
        //this value discovered by snooping
        KeyboardKey::Command | KeyboardKey::RightCommand => 1 << 6,
        KeyboardKey::NumLock => 1 << 14,
        _ => 0,
    }
}

fn key_is_text_input(key: KeyboardKey) -> bool {
    match key {
        // Letters
        KeyboardKey::A
        | KeyboardKey::B
        | KeyboardKey::C
        | KeyboardKey::D
        | KeyboardKey::E
        | KeyboardKey::F
        | KeyboardKey::G
        | KeyboardKey::H
        | KeyboardKey::I
        | KeyboardKey::J
        | KeyboardKey::K
        | KeyboardKey::L
        | KeyboardKey::M
        | KeyboardKey::N
        | KeyboardKey::O
        | KeyboardKey::P
        | KeyboardKey::Q
        | KeyboardKey::R
        | KeyboardKey::S
        | KeyboardKey::T
        | KeyboardKey::U
        | KeyboardKey::V
        | KeyboardKey::W
        | KeyboardKey::X
        | KeyboardKey::Y
        | KeyboardKey::Z => true,

        // Numbers
        KeyboardKey::Num0
        | KeyboardKey::Num1
        | KeyboardKey::Num2
        | KeyboardKey::Num3
        | KeyboardKey::Num4
        | KeyboardKey::Num5
        | KeyboardKey::Num6
        | KeyboardKey::Num7
        | KeyboardKey::Num8
        | KeyboardKey::Num9 => true,

        // Special characters
        KeyboardKey::Space
        | KeyboardKey::Minus
        | KeyboardKey::Equal
        | KeyboardKey::LeftBracket
        | KeyboardKey::RightBracket
        | KeyboardKey::Backslash
        | KeyboardKey::Semicolon
        | KeyboardKey::Quote
        | KeyboardKey::Grave
        | KeyboardKey::Comma
        | KeyboardKey::Period
        | KeyboardKey::Slash => true,

        // Keypad numbers and symbols (when NumLock is on)
        KeyboardKey::Keypad0
        | KeyboardKey::Keypad1
        | KeyboardKey::Keypad2
        | KeyboardKey::Keypad3
        | KeyboardKey::Keypad4
        | KeyboardKey::Keypad5
        | KeyboardKey::Keypad6
        | KeyboardKey::Keypad7
        | KeyboardKey::Keypad8
        | KeyboardKey::Keypad9
        | KeyboardKey::KeypadDecimal
        | KeyboardKey::KeypadMultiply
        | KeyboardKey::KeypadPlus
        | KeyboardKey::KeypadDivide
        | KeyboardKey::KeypadMinus
        | KeyboardKey::KeypadEquals => true,

        // Japanese input characters
        KeyboardKey::JISYen | KeyboardKey::JISUnderscore | KeyboardKey::JISKeypadComma => true,

        // Everything else doesn't generate text input
        _ => false,
    }
}
