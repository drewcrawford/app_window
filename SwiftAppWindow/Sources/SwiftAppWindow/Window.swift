//SPDX-License-Identifier: MPL-2.0

//
//  Window.swift
//  SwiftAppWindow
//
//  Created by Drew Crawford on 12/22/24.
//
import AppKit
import SwiftRs

final class NSWindowCustomize: NSWindow {
    override func keyDown(with event: NSEvent) {
        //don't call super to avoid the noise
    }
    
}

public final class Window: Sendable {
    @MainActor let window: NSWindow

    @MainActor init(x: CGFloat, y: CGFloat, width: CGFloat, height: CGFloat, title: String) {
        NSApplication.shared.setActivationPolicy(.regular)
        NSApplication.shared.activate()
        let window = NSWindowCustomize(contentRect: NSRect(origin: .zero, size: .init(width: width, height: height)) , styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = SurfaceView()
        window.title = title
        let screen = window.screen!
        window.setFrameOrigin(.init(rustX: x, rustY: y, outerBounds: screen.frame))
        window.makeKeyAndOrderFront(nil)
        self.window = window
    }

    @MainActor init(fullscreen: (), title: String) {
        NSApplication.shared.setActivationPolicy(.regular)
        NSApplication.shared.activate()
        let window = NSWindowCustomize(contentRect: .init(origin: .zero, size: NSScreen.main!.frame.size), styleMask: [.borderless], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = SurfaceView()
        window.title = title
        window.collectionBehavior = [.fullScreenPrimary]
        window.setFrame(window.screen!.frame, display: true)
        window.makeKeyAndOrderFront(nil)
        window.toggleFullScreen(nil)
        self.window = window
    }
    deinit {
        let window = self.window
        Task {
            await MainActor.run {
                window.close()
            }
        }
    }
    public func surface() async -> Surface {
        let view = await MainActor.run {
            let view = window.contentView! as! SurfaceView
            return view
        }
        return Surface(view: view)
    }
}

@_cdecl("SwiftAppWindow_WindowNew") public func WindowNew(context: UInt64, x: CGFloat, y: CGFloat, width: CGFloat, height: CGFloat, title: SRString, ret: @convention(c) @Sendable (UInt64, UnsafeMutableRawPointer) -> ()) {
    let title = title.toString()
    Task { @MainActor in
        let window = Window(x: x, y: y, width: width, height: height, title: title)
        ret(context, Unmanaged.passRetained(window).toOpaque())
    }
}

@_cdecl("SwiftAppWindow_WindowNewFullscreen") public func WindowNewFullscreen(context: UInt64, title: SRString, ret: @convention(c) @Sendable (UInt64, UnsafeMutableRawPointer) -> ()) {
    let title = title.toString()
    Task { @MainActor in
        let window = Window(fullscreen: (), title: title)
        ret(context, Unmanaged.passRetained(window).toOpaque())
    }
}

@_cdecl("SwiftAppWindow_WindowFree") public func WindowFree(window: UInt64) {
    let window = UnsafeMutableRawPointer(bitPattern: Int(window))!
    Unmanaged<Window>.fromOpaque(window).release()
}

@_cdecl("SwiftAppWindow_WindowSurface") public func WindowSurface(context: UInt64, window: UnsafeMutableRawPointer, ret: @convention(c) @Sendable (UInt64, UnsafeMutableRawPointer) -> ()) {
    asyncBridge(context: context, input: window, inputType: Window.self, ret: ret) { window in
        await window.surface()
    }
}

