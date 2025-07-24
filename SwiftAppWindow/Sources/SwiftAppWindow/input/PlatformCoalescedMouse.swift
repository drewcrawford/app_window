// SPDX-License-Identifier: MPL-2.0
//
//  PlatformCoalescedMouse.swift
//  SwiftRawInput
//
//  Created by Drew Crawford on 12/16/24.
//
import AppKit
import SwiftAppWindowC

func convertToRustCoordinates(absolutePoint: NSPoint, minX: Double, maxY: Double) -> (x: Double, y: Double) {
    //flip to upper left coordinate system
    return (x: absolutePoint.x - minX, y: maxY - absolutePoint.y)
}

final class PlatformCoalescedMouse:
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
        
        let sendContext = Int(bitPattern: context)
        
        self.monitor = NSEvent.addLocalMonitorForEvents(matching: [.mouseMoved, .leftMouseDown, .leftMouseUp, .otherMouseDown, .otherMouseUp, .rightMouseDown, .rightMouseUp,.scrollWheel]) { event in
            nonisolated(unsafe) let eventWindow: UnsafeMutableRawPointer?
            if let window = event.window {
                eventWindow = Unmanaged.passUnretained(window).toOpaque()
            }
            else {
                eventWindow = nil
            }
            switch event.type {
            case .mouseMoved:
                let location = event.locationInWindow
                if let window = event.window {
                    MainActor.assumeIsolated {
                        let recvContext = UnsafeMutableRawPointer(bitPattern: sendContext)
                        if let contentView = window.contentView {
                            let contentPoint = contentView.convert(location, from: nil)
                            let contentPointRust = convertToRustCoordinates(absolutePoint: contentPoint, minX: 0, maxY: contentView.frame.size.height)
                            raw_input_mouse_move(recvContext, eventWindow, contentPointRust.x, contentPointRust.y, 0, contentView.frame.size.height)
                        }
                        else {
                            let windowRustCoords = convertToRustCoordinates(absolutePoint: location, minX: 0, maxY: window.frame.size.height)
                            raw_input_mouse_move(recvContext, eventWindow, windowRustCoords.x, windowRustCoords.y, 0, window.frame.size.height)
                        }
                    }
                    
                }
            case .leftMouseDown:
                raw_input_mouse_button(context, eventWindow, 0, true)
            case .leftMouseUp:
                raw_input_mouse_button(context, eventWindow, 0, false)
            case .rightMouseDown:
                raw_input_mouse_button(context, eventWindow, 1, true)
            case .rightMouseUp:
                raw_input_mouse_button(context, eventWindow, 1, false)
            case .otherMouseDown:
                raw_input_mouse_button(context,  eventWindow, UInt8(event.buttonNumber), true)
            case .otherMouseUp:
                raw_input_mouse_button(context,  eventWindow, UInt8(event.buttonNumber), false)
            case .scrollWheel:
                raw_input_mouse_scroll(context,  eventWindow, event.scrollingDeltaX, event.scrollingDeltaY)
            default:
                fatalError("\(event)")
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

public typealias MouseNotifyFunc = @convention(c) (UnsafeMutableRawPointer) -> ()

@_cdecl("PlatformCoalescedMouseNew") public func PlatformCoalescedMouseNew(context: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {

    let p = PlatformCoalescedMouse(context: context)
    return Unmanaged.passRetained(p).toOpaque()
}

@_cdecl("PlatformCoalescedMouseFree") public func PlatformCoalescedMouseFree(_ p: UnsafeMutableRawPointer) {
    Unmanaged<PlatformCoalescedMouse>.fromOpaque(p).release()
}
