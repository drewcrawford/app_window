//
//  Window.swift
//  SwiftAppWindow
//
//  Created by Drew Crawford on 12/22/24.
//
import AppKit
import SwiftRs

public final class Window: Sendable {
    @MainActor var window: NSWindow?
    
    init(x: CGFloat, y: CGFloat, width: CGFloat, height: CGFloat, title: String) {
        Task {
            await MainActor.run {
                NSApplication.shared.setActivationPolicy(.regular)
                NSApplication.shared.activate()
                self.window = NSWindow(contentRect: .init(x: x, y: y, width: width, height: height), styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
                self.window!.title = title
                self.window!.makeKeyAndOrderFront(nil)
                
            }
        }
    }
}

@_cdecl("SwiftAppWindow_WindowNew") public func WindowNew(x: CGFloat, y: CGFloat, width: CGFloat, height: CGFloat, title: SRString) -> UnsafeMutableRawPointer {
    let w = Window(x: x, y: y, width: width, height: height, title: title.toString())
    let unmanaged = Unmanaged.passRetained(w).toOpaque()
    return unmanaged
}

@_cdecl("SwiftAppWindow_WindowFree") public func WindowFree(window: UInt64) {
    let window = UnsafeMutableRawPointer(bitPattern: Int(window))!
    Unmanaged<Window>.fromOpaque(window).release()
}


