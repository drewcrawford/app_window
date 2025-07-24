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

final class PlatformCoalescedKeyboard:
    /*Rust type implements send/sync
     **/
    Sendable
{
    nonisolated(unsafe) let monitor: Any?
    nonisolated(unsafe) let context: UnsafeMutableRawPointer
    
    init(context: UnsafeMutableRawPointer) {
        MainActor.shared.dispatchMainThreadFromRustContextDetached {
            NSApplication.shared.setActivationPolicy(.regular)
        }
        self.context = context
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
                func notifyModifier(event: NSEvent, flag: NSEvent.ModifierFlags) {
                    if event.modifierFlags.contains(flag) {
                        raw_input_key_notify_func(context, eventWindow, event.keyCode, true)
                    }
                    else {
                        raw_input_key_notify_func(context, eventWindow, event.keyCode, false)
                    }
                }

                switch event.keyCode {
                case 0x3B: //control
                    notifyModifier(event: event, flag: .control)
                case 0x3E: //right control
                    notifyModifier(event: event, flag: .control)
                case 0x3A: //option
                    notifyModifier(event: event, flag: .option)
                case 0x3D://right option
                    notifyModifier(event: event, flag: .option)
                case 0x37://command
                    notifyModifier(event: event, flag: .command)
                case 0x36: //right command
                    notifyModifier(event: event, flag: .command)
                case 0x38: //shift
                    notifyModifier(event: event, flag: .shift)
                case 0x3C: //right shift
                    notifyModifier(event: event, flag: .shift)
                case 0x3F: //function
                    notifyModifier(event: event, flag: .function)
                case 0x39: //caps lock
                    notifyModifier(event: event, flag: .capsLock)
                
                    
                
                default:
                    fatalError("\(event)")
                }
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
