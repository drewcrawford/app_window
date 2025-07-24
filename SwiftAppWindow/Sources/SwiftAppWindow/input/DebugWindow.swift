// SPDX-License-Identifier: MPL-2.0
//
//  DebugWindow.swift
//  SwiftRawInput
//
//  Created by Drew Crawford on 12/13/24.
//

import AppKit


@MainActor var window: NSWindow? = nil

private final class DebugWindow: NSWindow {
    override func keyDown(with event: NSEvent) {
        //don't call super to avoid 'error' sound
    }
}

@_cdecl("SwiftRawInputDebugWindowShow") public func DebugWindowShow() {
    MainActor.assumeIsolated {
        window = DebugWindow()
        window?.makeKeyAndOrderFront(nil)
        
        window?.contentView = NSView(frame: .init(origin: .zero, size: .init(width: 500, height: 500)))
        NSApplication.shared.run()
    }
}

@_cdecl("SwiftRawInputDebugWindowHide") public func DebugWindowHide(_ p: UnsafeMutableRawPointer) {
    MainActor.assumeIsolated {
        window?.close()
        window = nil
    }
}
