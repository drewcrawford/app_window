// SPDX-License-Identifier: MPL-2.0
/// A key on the keyboard.
///
/// This enum represents physical keys on a keyboard, mapped from platform-specific
/// scancodes to a unified representation. Each variant corresponds to a specific
/// physical key, independent of the keyboard layout or locale settings.
///
/// # Physical vs Logical Keys
///
/// `KeyboardKey` represents *physical* keys rather than *logical* characters.
/// For example, the `A` variant represents the physical key labeled 'A' on a
/// QWERTY keyboard, regardless of what character it produces when pressed.
/// This makes it ideal for game controls and shortcuts, but not for text input.
///
/// # Platform Mapping
///
/// Keys are mapped from platform-specific scancodes:
/// - **Windows**: Virtual key codes (VK_*)
/// - **macOS**: Hardware key codes from NSEvent
/// - **Linux**: Wayland keycodes
/// - **WebAssembly**: KeyboardEvent.code values
///
/// # Examples
///
/// ```
/// use app_window::input::keyboard::key::KeyboardKey;
///
/// // Keys can be compared
/// assert_eq!(KeyboardKey::A, KeyboardKey::A);
/// assert_ne!(KeyboardKey::A, KeyboardKey::B);
///
/// // Keys are Copy types
/// let key = KeyboardKey::Space;
/// let key_copy = key;
/// assert_eq!(key, key_copy);
///
/// // Keys can be used in match expressions
/// match key {
///     KeyboardKey::Space => println!("Spacebar"),
///     KeyboardKey::Escape => println!("Escape"),
///     _ => println!("Other key"),
/// }
/// ```
///
/// # Special Keys
///
/// ```
/// use app_window::input::keyboard::key::KeyboardKey;
///
/// // Modifier keys
/// let modifiers = [
///     KeyboardKey::Shift,
///     KeyboardKey::Control,
///     KeyboardKey::Option,    // Alt key
///     KeyboardKey::Command,   // Cmd/Windows key
/// ];
///
/// // Function keys
/// let function_keys = [
///     KeyboardKey::F1,
///     KeyboardKey::F2,
///     KeyboardKey::F3,
///     // ... up to F24
/// ];
///
/// // Navigation keys
/// let nav_keys = [
///     KeyboardKey::UpArrow,
///     KeyboardKey::DownArrow,
///     KeyboardKey::LeftArrow,
///     KeyboardKey::RightArrow,
///     KeyboardKey::Home,
///     KeyboardKey::End,
///     KeyboardKey::PageUp,
///     KeyboardKey::PageDown,
/// ];
/// ```
#[repr(usize)]
#[derive(Debug, Hash, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyboardKey {
    /// The 'A' key on the main keyboard area.
    A,
    /// The 'S' key on the main keyboard area.
    S,
    /// The 'D' key on the main keyboard area.
    D,
    /// The 'F' key on the main keyboard area.
    F,
    /// The 'H' key on the main keyboard area.
    H,
    /// The 'G' key on the main keyboard area.
    G,
    /// The 'Z' key on the main keyboard area.
    Z,
    /// The 'X' key on the main keyboard area.
    X,
    /// The 'C' key on the main keyboard area.
    C,
    /// The 'V' key on the main keyboard area.
    V,
    /// The 'B' key on the main keyboard area.
    B,
    /// The 'Q' key on the main keyboard area.
    Q,
    /// The 'W' key on the main keyboard area (common for forward movement in games).
    W,
    /// The 'E' key on the main keyboard area (common for interact/use in games).
    E,
    /// The 'R' key on the main keyboard area (common for reload in games).
    R,
    /// The 'Y' key on the main keyboard area.
    Y,
    /// The 'T' key on the main keyboard area.
    T,
    Num1,
    Num2,
    Num3,
    Num4,
    Num6,
    Num5,
    Equal,
    Num9,
    Num7,
    Minus,
    Num8,
    Num0,
    RightBracket,
    O,
    U,
    LeftBracket,
    I,
    P,
    L,
    J,
    Quote,
    K,
    Semicolon,
    Backslash,
    Comma,
    Slash,
    N,
    M,
    Period,
    Grave,
    KeypadDecimal,
    KeypadMultiply,
    KeypadPlus,
    KeypadClear,
    KeypadDivide,
    KeypadEnter,
    KeypadMinus,
    KeypadEquals,
    Keypad0,
    Keypad1,
    Keypad2,
    Keypad3,
    Keypad4,
    Keypad5,
    Keypad6,
    Keypad7,
    Keypad8,
    Keypad9,
    /// The Return/Enter key.
    Return,
    /// The Tab key (typically used for indentation or field navigation).
    Tab,
    /// The Space bar.
    Space,
    /// The Delete/Backspace key (deletes to the left of cursor).
    Delete,
    /// The Escape key (often used to cancel operations or open menus).
    Escape,
    /// The Command key on macOS, Windows key on PC keyboards.
    Command,
    /// The left Shift key (modifier for uppercase and alternate characters).
    Shift,
    /// The Caps Lock key (toggles uppercase letter input).
    CapsLock,
    /// The Option/Alt key (left side).
    Option,
    /// The left Control key (modifier for shortcuts).
    Control,
    /// The right Command key on macOS, right Windows key on PC keyboards.
    RightCommand,
    /// The right Shift key.
    RightShift,
    /// The right Option/Alt key.
    RightOption,
    /// The right Control key.
    RightControl,
    /// The Function (Fn) key found on many laptop keyboards.
    Function,
    F17,
    VolumeUp,
    VolumeDown,
    Mute,
    F18,
    F19,
    F20,
    F5,
    F6,
    F7,
    F3,
    F8,
    F9,
    F11,
    F13,
    F16,
    F14,
    F10,
    ContextualMenu,
    F12,
    F15,
    Help,
    Home,
    PageUp,
    ForwardDelete,
    F4,
    End,
    F2,
    PageDown,
    F1,
    /// The left arrow key (navigation).
    LeftArrow,
    /// The right arrow key (navigation).
    RightArrow,
    /// The down arrow key (navigation).
    DownArrow,
    /// The up arrow key (navigation).
    UpArrow,
    ISOSection,
    JISYen,
    JISUnderscore,
    JISKeypadComma,
    JISEisu,
    JISKana,
    Pause,
    ScrollLock,
    PrintScreen,
    InternationalBackslash,
    F21,
    F22,
    F23,
    F24,
    Convert,
    NonConvert,
    PreviousTrack,
    NextTrack,
    LaunchApp2,
    Play,
    Stop,
    BrowserHome,
    NumLock,
    Insert,
    ContextMenu,
    Power,
    Eject,
    BrowserSearch,
    BrowserFavorites,
    BrowserRefresh,
    BrowserStop,
    BrowserForward,
    BrowserBack,
    LaunchApp1,
    LaunchMail,
    MediaSelect,
    Again,
    Props,
    Undo,
    Select,
    Copy,
    Open,
    Paste,
    Find,
    Cut,
    WakeUp,
}

impl KeyboardKey {
    /// Returns all keys supported by the library.
    ///
    /// This method returns a vector containing every variant of the `KeyboardKey` enum.
    /// It's useful for iterating over all possible keys, for example when creating
    /// a key binding configuration UI or when debugging keyboard input.
    ///
    /// # Performance Note
    ///
    /// This method allocates a new `Vec` on each call. If you need to iterate over
    /// all keys frequently, consider caching the result.
    ///
    /// # Examples
    ///
    /// ```
    /// use app_window::input::keyboard::key::KeyboardKey;
    ///
    /// let all_keys = KeyboardKey::all_keys();
    ///
    /// // Check that we have a reasonable number of keys
    /// assert!(all_keys.len() > 50);
    /// assert!(all_keys.len() < 200);
    ///
    /// // Verify specific keys are included
    /// assert!(all_keys.contains(&KeyboardKey::A));
    /// assert!(all_keys.contains(&KeyboardKey::Space));
    /// assert!(all_keys.contains(&KeyboardKey::Escape));
    /// ```
    ///
    /// ## Iterating Over All Keys
    ///
    /// ```
    /// use app_window::input::keyboard::key::KeyboardKey;
    ///
    /// // Count specific types of keys
    /// let all_keys = KeyboardKey::all_keys();
    ///
    /// let function_keys = all_keys.iter()
    ///     .filter(|k| matches!(k,
    ///         KeyboardKey::F1 | KeyboardKey::F2 | KeyboardKey::F3 |
    ///         KeyboardKey::F4 | KeyboardKey::F5 | KeyboardKey::F6 |
    ///         KeyboardKey::F7 | KeyboardKey::F8 | KeyboardKey::F9 |
    ///         KeyboardKey::F10 | KeyboardKey::F11 | KeyboardKey::F12
    ///     ))
    ///     .count();
    ///
    /// assert!(function_keys >= 12);
    ///
    /// let letter_keys = all_keys.iter()
    ///     .filter(|k| matches!(k,
    ///         KeyboardKey::A | KeyboardKey::B | KeyboardKey::C |
    ///         KeyboardKey::D | KeyboardKey::E | KeyboardKey::F |
    ///         KeyboardKey::G | KeyboardKey::H | KeyboardKey::I
    ///         // ... and so on
    ///     ))
    ///     .count();
    ///
    /// assert!(letter_keys > 0);
    /// ```
    pub fn all_keys() -> Vec<KeyboardKey> {
        vec![
            KeyboardKey::A,
            KeyboardKey::S,
            KeyboardKey::D,
            KeyboardKey::F,
            KeyboardKey::H,
            KeyboardKey::G,
            KeyboardKey::Z,
            KeyboardKey::X,
            KeyboardKey::C,
            KeyboardKey::V,
            KeyboardKey::B,
            KeyboardKey::Q,
            KeyboardKey::W,
            KeyboardKey::E,
            KeyboardKey::R,
            KeyboardKey::Y,
            KeyboardKey::T,
            KeyboardKey::Num1,
            KeyboardKey::Num2,
            KeyboardKey::Num3,
            KeyboardKey::Num4,
            KeyboardKey::Num6,
            KeyboardKey::Num5,
            KeyboardKey::Equal,
            KeyboardKey::Num9,
            KeyboardKey::Num7,
            KeyboardKey::Minus,
            KeyboardKey::Num8,
            KeyboardKey::Num0,
            KeyboardKey::RightBracket,
            KeyboardKey::O,
            KeyboardKey::U,
            KeyboardKey::LeftBracket,
            KeyboardKey::I,
            KeyboardKey::P,
            KeyboardKey::L,
            KeyboardKey::J,
            KeyboardKey::Quote,
            KeyboardKey::K,
            KeyboardKey::Semicolon,
            KeyboardKey::Backslash,
            KeyboardKey::Comma,
            KeyboardKey::Slash,
            KeyboardKey::N,
            KeyboardKey::M,
            KeyboardKey::Period,
            KeyboardKey::Grave,
            KeyboardKey::KeypadDecimal,
            KeyboardKey::KeypadMultiply,
            KeyboardKey::KeypadPlus,
            KeyboardKey::KeypadClear,
            KeyboardKey::KeypadDivide,
            KeyboardKey::KeypadEnter,
            KeyboardKey::KeypadMinus,
            KeyboardKey::KeypadEquals,
            KeyboardKey::Keypad0,
            KeyboardKey::Keypad1,
            KeyboardKey::Keypad2,
            KeyboardKey::Keypad3,
            KeyboardKey::Keypad4,
            KeyboardKey::Keypad5,
            KeyboardKey::Keypad6,
            KeyboardKey::Keypad7,
            KeyboardKey::Keypad8,
            KeyboardKey::Keypad9,
            KeyboardKey::Return,
            KeyboardKey::Tab,
            KeyboardKey::Space,
            KeyboardKey::Delete,
            KeyboardKey::Escape,
            KeyboardKey::Command,
            KeyboardKey::Shift,
            KeyboardKey::CapsLock,
            KeyboardKey::Option,
            KeyboardKey::Control,
            KeyboardKey::RightCommand,
            KeyboardKey::RightShift,
            KeyboardKey::RightOption,
            KeyboardKey::RightControl,
            KeyboardKey::Function,
            KeyboardKey::F17,
            KeyboardKey::VolumeUp,
            KeyboardKey::VolumeDown,
            KeyboardKey::Mute,
            KeyboardKey::F18,
            KeyboardKey::F19,
            KeyboardKey::F20,
            KeyboardKey::F5,
            KeyboardKey::F6,
            KeyboardKey::F7,
            KeyboardKey::F3,
            KeyboardKey::F8,
            KeyboardKey::F9,
            KeyboardKey::F11,
            KeyboardKey::F13,
            KeyboardKey::F16,
            KeyboardKey::F14,
            KeyboardKey::F10,
            KeyboardKey::ContextualMenu,
            KeyboardKey::F12,
            KeyboardKey::F15,
            KeyboardKey::Help,
            KeyboardKey::Home,
            KeyboardKey::PageUp,
            KeyboardKey::ForwardDelete,
            KeyboardKey::F4,
            KeyboardKey::End,
            KeyboardKey::F2,
            KeyboardKey::PageDown,
            KeyboardKey::F1,
            KeyboardKey::LeftArrow,
            KeyboardKey::RightArrow,
            KeyboardKey::DownArrow,
            KeyboardKey::UpArrow,
            KeyboardKey::ISOSection,
            KeyboardKey::JISYen,
            KeyboardKey::JISUnderscore,
            KeyboardKey::JISKeypadComma,
            KeyboardKey::JISEisu,
            KeyboardKey::JISKana,
            KeyboardKey::Pause,
            KeyboardKey::ScrollLock,
            KeyboardKey::PrintScreen,
            KeyboardKey::InternationalBackslash,
            KeyboardKey::F21,
            KeyboardKey::F22,
            KeyboardKey::F23,
            KeyboardKey::F24,
            KeyboardKey::Convert,
            KeyboardKey::NonConvert,
            KeyboardKey::PreviousTrack,
            KeyboardKey::NextTrack,
            KeyboardKey::LaunchApp2,
            KeyboardKey::Play,
            KeyboardKey::Stop,
            KeyboardKey::BrowserHome,
            KeyboardKey::NumLock,
            KeyboardKey::Insert,
            KeyboardKey::ContextMenu,
            KeyboardKey::Power,
            KeyboardKey::Eject,
            KeyboardKey::BrowserSearch,
            KeyboardKey::BrowserFavorites,
            KeyboardKey::BrowserRefresh,
            KeyboardKey::BrowserStop,
            KeyboardKey::BrowserForward,
            KeyboardKey::BrowserBack,
            KeyboardKey::LaunchApp1,
            KeyboardKey::LaunchMail,
            KeyboardKey::MediaSelect,
            KeyboardKey::Again,
            KeyboardKey::Props,
            KeyboardKey::Undo,
            KeyboardKey::Select,
            KeyboardKey::Copy,
            KeyboardKey::Open,
            KeyboardKey::Paste,
            KeyboardKey::Find,
            KeyboardKey::Cut,
            KeyboardKey::WakeUp,
        ]
    }
}
