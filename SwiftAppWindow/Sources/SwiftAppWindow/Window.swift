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
    @MainActor var window: NSWindow?
    
    init(x: CGFloat, y: CGFloat, width: CGFloat, height: CGFloat, title: String) {
        Task {
            await MainActor.run {
                NSApplication.shared.setActivationPolicy(.regular)
                NSApplication.shared.activate()
                let _window = NSWindowCustomize(contentRect: NSRect(origin: .zero, size: .init(width: width, height: height)) , styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
                _window.isReleasedWhenClosed = false
                self.window = _window
                
                _window.title = title
                let screen = _window.screen!
                _window.setFrameOrigin(.init(rustX: x, rustY: y, outerBounds: screen.frame))
                self.window!.makeKeyAndOrderFront(nil)
            }
        }
    }
    
    init(fullscreen: (), title: String) {
        
        Task {
            await MainActor.run {
                NSApplication.shared.setActivationPolicy(.regular)
                NSApplication.shared.activate()
                let _window = NSWindowCustomize(contentRect: .init(origin: .zero, size: NSScreen.main!.frame.size), styleMask: [.borderless], backing: .buffered, defer: false)
                _window.isReleasedWhenClosed = false

                self.window = _window
                
                _window.title = title
                _window.collectionBehavior = [.fullScreenPrimary]
                _window.setFrame(_window.screen!.frame, display: true)
                _window.makeKeyAndOrderFront(nil)
                _window.toggleFullScreen(nil)
                
            }
        }
    }
    deinit {
        //I'm not really sure why but there's some ARC issue here
        if let window {
            Task {
                await MainActor.run {
                    print("Close the moved window?")
                    print("description \(window)")
                    window.close()
                }
            }
        }
    }
    public func surface() async -> Surface {
        let view = await MainActor.run {
            let window = self.window!
            let view = window.contentView!
            return view
        }
        return Surface(view: view)
    }
}

@_cdecl("SwiftAppWindow_WindowNew") public func WindowNew(x: CGFloat, y: CGFloat, width: CGFloat, height: CGFloat, title: SRString) -> UnsafeMutableRawPointer {
    let w = Window(x: x, y: y, width: width, height: height, title: title.toString())
    let unmanaged = Unmanaged.passRetained(w).toOpaque()
    return unmanaged
}

@_cdecl("SwiftAppWindow_WindowNewFullscreen") public func WindowNew(title: SRString) -> UnsafeMutableRawPointer {
    let w = Window(fullscreen: (), title: title.toString())
    let unmanaged = Unmanaged.passRetained(w).toOpaque()
    return unmanaged
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


