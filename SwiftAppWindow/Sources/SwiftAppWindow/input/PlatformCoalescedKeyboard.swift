// SPDX-License-Identifier: MPL-2.0
//
//  PlatformCoalescedKeyboard.swift
//  SwiftRawInput
//
//  Created by Drew Crawford on 12/13/24.
//

#if canImport(AppKit)
import AppKit
#else
#error("This platform is not yet supported")
#endif
import SwiftAppWindowC

// Holds mutable modifier state for the event monitor closure
private final class ModifierFlagsState: @unchecked Sendable {
    var previousFlags: UInt = 0
}

final class PlatformCoalescedKeyboard:
    /*Rust type implements send/sync
     **/
    Sendable
{
    nonisolated(unsafe) let monitor: Any?
    nonisolated(unsafe) let context: UnsafeMutableRawPointer
    private let flagsState = ModifierFlagsState()

    // Device-specific modifier masks (from IOKit/hidsystem/IOLLEvent.h)
    private static let NX_DEVICELCTLKEYMASK: UInt   = 0x00000001
    private static let NX_DEVICELSHIFTKEYMASK: UInt = 0x00000002
    private static let NX_DEVICERSHIFTKEYMASK: UInt = 0x00000004
    private static let NX_DEVICELCMDKEYMASK: UInt   = 0x00000008
    private static let NX_DEVICERCMDKEYMASK: UInt   = 0x00000010
    private static let NX_DEVICELALTKEYMASK: UInt   = 0x00000020
    private static let NX_DEVICERALTKEYMASK: UInt   = 0x00000040
    private static let NX_DEVICERCTLKEYMASK: UInt   = 0x00002000

    // Device-independent modifier masks
    private static let NX_COMMANDMASK: UInt  = 0x00100000
    private static let NX_SHIFTMASK: UInt    = 0x00020000
    private static let NX_CONTROLMASK: UInt  = 0x00040000
    private static let NX_ALTERNATEMASK: UInt = 0x00080000
    private static let NX_FUNCTIONMASK: UInt = 0x00800000
    private static let NX_CAPSLOCKMASK: UInt = 0x00010000

    init(context: UnsafeMutableRawPointer) {
        MainActor.shared.dispatchMainThreadFromRustContextDetached {
            NSApplication.shared.setActivationPolicy(.regular)
        }
        self.context = context
        let flagsState = self.flagsState
        self.monitor = NSEvent.addLocalMonitorForEvents(matching: [.keyDown, .keyUp, .flagsChanged]) { event in
            let eventWindow: UnsafeMutableRawPointer?
            if let window = event.window {
                eventWindow = Unmanaged.passUnretained(window).toOpaque()
            }
            else {
                eventWindow = nil
            }
            switch event.type {
            case .keyDown:
                raw_input_key_notify_func(context,  eventWindow, event.keyCode, true)
            case .keyUp:
                raw_input_key_notify_func(context, eventWindow, event.keyCode, false)
            case .flagsChanged:
                let curr = UInt(event.modifierFlags.rawValue)
                let prev = flagsState.previousFlags

                // Helper to check modifier state changes
                func checkModifier(deviceMask: UInt, independentMask: UInt, keyCode: UInt16) {
                    // A modifier is "active" when both the device-specific bit AND
                    // the device-independent bit are set
                    let prevActive = (prev & deviceMask) != 0 && (prev & independentMask) != 0
                    let currActive = (curr & deviceMask) != 0 && (curr & independentMask) != 0

                    if prevActive && !currActive {
                        raw_input_key_notify_func(context, eventWindow, keyCode, false)
                    } else if !prevActive && currActive {
                        raw_input_key_notify_func(context, eventWindow, keyCode, true)
                    }
                }

                // Helper for modifiers without left/right distinction
                func checkSingleModifier(mask: UInt, keyCode: UInt16) {
                    let prevActive = (prev & mask) != 0
                    let currActive = (curr & mask) != 0

                    if prevActive && !currActive {
                        raw_input_key_notify_func(context, eventWindow, keyCode, false)
                    } else if !prevActive && currActive {
                        raw_input_key_notify_func(context, eventWindow, keyCode, true)
                    }
                }

                // Check each modifier key
                // Command keys
                checkModifier(deviceMask: Self.NX_DEVICELCMDKEYMASK, independentMask: Self.NX_COMMANDMASK, keyCode: 0x37)
                checkModifier(deviceMask: Self.NX_DEVICERCMDKEYMASK, independentMask: Self.NX_COMMANDMASK, keyCode: 0x36)

                // Shift keys
                checkModifier(deviceMask: Self.NX_DEVICELSHIFTKEYMASK, independentMask: Self.NX_SHIFTMASK, keyCode: 0x38)
                checkModifier(deviceMask: Self.NX_DEVICERSHIFTKEYMASK, independentMask: Self.NX_SHIFTMASK, keyCode: 0x3C)

                // Option keys
                checkModifier(deviceMask: Self.NX_DEVICELALTKEYMASK, independentMask: Self.NX_ALTERNATEMASK, keyCode: 0x3A)
                checkModifier(deviceMask: Self.NX_DEVICERALTKEYMASK, independentMask: Self.NX_ALTERNATEMASK, keyCode: 0x3D)

                // Control keys
                checkModifier(deviceMask: Self.NX_DEVICELCTLKEYMASK, independentMask: Self.NX_CONTROLMASK, keyCode: 0x3B)
                checkModifier(deviceMask: Self.NX_DEVICERCTLKEYMASK, independentMask: Self.NX_CONTROLMASK, keyCode: 0x3E)

                // Function key (no left/right distinction)
                checkSingleModifier(mask: Self.NX_FUNCTIONMASK, keyCode: 0x3F)

                // Caps Lock (no left/right distinction)
                checkSingleModifier(mask: Self.NX_CAPSLOCKMASK, keyCode: 0x39)

                flagsState.previousFlags = curr

            default:
                fatalError("Unknown event type \(event.type)")
            }


            return event
        }

    }
    deinit {
        if let monitor {
            NSEvent.removeMonitor(monitor)
        }
        raw_input_finish_mouse_event_context(self.context)
        
    }
}


@_cdecl("PlatformCoalescedKeyboardNew") public func PlatformCoalescedKeyboardNew( context: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    let p = PlatformCoalescedKeyboard(context: context)
    return Unmanaged.passRetained(p).toOpaque()
}

@_cdecl("PlatformCoalescedKeyboardFree") public func PlatformCoalescedKeyboardFree(_ p: UnsafeMutableRawPointer) {
    Unmanaged<PlatformCoalescedKeyboard>.fromOpaque(p).release()
}
