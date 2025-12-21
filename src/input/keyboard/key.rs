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
    /// The '1' key on the main keyboard area.
    Num1,
    /// The '2' key on the main keyboard area.
    Num2,
    /// The '3' key on the main keyboard area.
    Num3,
    /// The '4' key on the main keyboard area.
    Num4,
    /// The '6' key on the main keyboard area.
    Num6,
    /// The '5' key on the main keyboard area.
    Num5,
    /// The equals '=' key.
    Equal,
    /// The '9' key on the main keyboard area.
    Num9,
    /// The '7' key on the main keyboard area.
    Num7,
    /// The minus '-' key.
    Minus,
    /// The '8' key on the main keyboard area.
    Num8,
    /// The '0' key on the main keyboard area.
    Num0,
    /// The right bracket ']' key.
    RightBracket,
    /// The 'O' key on the main keyboard area.
    O,
    /// The 'U' key on the main keyboard area.
    U,
    /// The left bracket '[' key.
    LeftBracket,
    /// The 'I' key on the main keyboard area.
    I,
    /// The 'P' key on the main keyboard area.
    P,
    /// The 'L' key on the main keyboard area.
    L,
    /// The 'J' key on the main keyboard area.
    J,
    /// The quote/apostrophe '\'' key.
    Quote,
    /// The 'K' key on the main keyboard area.
    K,
    /// The semicolon ';' key.
    Semicolon,
    /// The backslash '\\' key.
    Backslash,
    /// The comma ',' key.
    Comma,
    /// The forward slash '/' key.
    Slash,
    /// The 'N' key on the main keyboard area.
    N,
    /// The 'M' key on the main keyboard area.
    M,
    /// The period '.' key.
    Period,
    /// The grave/backtick '`' key.
    Grave,
    /// The decimal point '.' key on the numeric keypad.
    KeypadDecimal,
    /// The multiply '*' key on the numeric keypad.
    KeypadMultiply,
    /// The plus '+' key on the numeric keypad.
    KeypadPlus,
    /// The Clear key on the numeric keypad (often Num Lock on PC keyboards).
    KeypadClear,
    /// The divide '/' key on the numeric keypad.
    KeypadDivide,
    /// The Enter key on the numeric keypad.
    KeypadEnter,
    /// The minus '-' key on the numeric keypad.
    KeypadMinus,
    /// The equals '=' key on the numeric keypad.
    KeypadEquals,
    /// The '0' key on the numeric keypad.
    Keypad0,
    /// The '1' key on the numeric keypad.
    Keypad1,
    /// The '2' key on the numeric keypad.
    Keypad2,
    /// The '3' key on the numeric keypad.
    Keypad3,
    /// The '4' key on the numeric keypad.
    Keypad4,
    /// The '5' key on the numeric keypad.
    Keypad5,
    /// The '6' key on the numeric keypad.
    Keypad6,
    /// The '7' key on the numeric keypad.
    Keypad7,
    /// The '8' key on the numeric keypad.
    Keypad8,
    /// The '9' key on the numeric keypad.
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
    /// The F17 function key.
    F17,
    /// The Volume Up key (media control).
    VolumeUp,
    /// The Volume Down key (media control).
    VolumeDown,
    /// The Mute key (media control).
    Mute,
    /// The F18 function key.
    F18,
    /// The F19 function key.
    F19,
    /// The F20 function key.
    F20,
    /// The F5 function key (often used for refresh).
    F5,
    /// The F6 function key.
    F6,
    /// The F7 function key.
    F7,
    /// The F3 function key (often used for search).
    F3,
    /// The F8 function key.
    F8,
    /// The F9 function key.
    F9,
    /// The F11 function key (often used for fullscreen).
    F11,
    /// The F13 function key.
    F13,
    /// The F16 function key.
    F16,
    /// The F14 function key.
    F14,
    /// The F10 function key (often used for menu activation).
    F10,
    /// The contextual menu key (equivalent to right-click).
    ContextualMenu,
    /// The F12 function key (often used for developer tools).
    F12,
    /// The F15 function key.
    F15,
    /// The Help key.
    Help,
    /// The Home key (move to beginning of line/document).
    Home,
    /// The Page Up key (scroll up one page).
    PageUp,
    /// The Forward Delete key (deletes to the right of cursor, often labeled 'Del').
    ForwardDelete,
    /// The F4 function key (often used for address bar in browsers).
    F4,
    /// The End key (move to end of line/document).
    End,
    /// The F2 function key (often used for rename operations).
    F2,
    /// The Page Down key (scroll down one page).
    PageDown,
    /// The F1 function key (often used for help).
    F1,
    /// The left arrow key (navigation).
    LeftArrow,
    /// The right arrow key (navigation).
    RightArrow,
    /// The down arrow key (navigation).
    DownArrow,
    /// The up arrow key (navigation).
    UpArrow,
    /// The ISO Section key (§/±) found on some international keyboards.
    ISOSection,
    /// The Yen (¥) key on Japanese keyboards.
    JISYen,
    /// The underscore key on Japanese keyboards.
    JISUnderscore,
    /// The comma key on the numeric keypad of Japanese keyboards.
    JISKeypadComma,
    /// The Eisu (英数) key for switching to alphanumeric input on Japanese keyboards.
    JISEisu,
    /// The Kana (かな) key for switching to kana input on Japanese keyboards.
    JISKana,
    /// The Pause/Break key.
    Pause,
    /// The Scroll Lock key.
    ScrollLock,
    /// The Print Screen key (often used for screenshots).
    PrintScreen,
    /// The backslash key found on some international keyboard layouts.
    InternationalBackslash,
    /// The F21 function key.
    F21,
    /// The F22 function key.
    F22,
    /// The F23 function key.
    F23,
    /// The F24 function key.
    F24,
    /// The Convert key (変換) on Japanese keyboards.
    Convert,
    /// The Non-Convert key (無変換) on Japanese keyboards.
    NonConvert,
    /// The Previous Track media key.
    PreviousTrack,
    /// The Next Track media key.
    NextTrack,
    /// The Launch Application 2 key.
    LaunchApp2,
    /// The Play/Pause media key.
    Play,
    /// The Stop media key.
    Stop,
    /// The Browser Home key.
    BrowserHome,
    /// The Num Lock key (toggles numeric keypad between numbers and navigation).
    NumLock,
    /// The Insert key (toggles insert/overwrite mode).
    Insert,
    /// The Context Menu key (equivalent to right-click, often has a menu icon).
    ContextMenu,
    /// The Power button key.
    Power,
    /// The Eject key (for removable media).
    Eject,
    /// The Browser Search key.
    BrowserSearch,
    /// The Browser Favorites/Bookmarks key.
    BrowserFavorites,
    /// The Browser Refresh key.
    BrowserRefresh,
    /// The Browser Stop key.
    BrowserStop,
    /// The Browser Forward key.
    BrowserForward,
    /// The Browser Back key.
    BrowserBack,
    /// The Launch Application 1 key.
    LaunchApp1,
    /// The Launch Mail application key.
    LaunchMail,
    /// The Media Select key.
    MediaSelect,
    /// The Again/Redo key.
    Again,
    /// The Props/Properties key.
    Props,
    /// The Undo key.
    Undo,
    /// The Select key.
    Select,
    /// The Copy key.
    Copy,
    /// The Open key.
    Open,
    /// The Paste key.
    Paste,
    /// The Find key.
    Find,
    /// The Cut key.
    Cut,
    /// The Wake Up key.
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
