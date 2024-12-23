//
//  Surface.swift
//  SwiftAppWindow
//
//  Created by Drew Crawford on 12/22/24.
//
import AppKit
public final class Surface: Sendable {
    let view: NSView
    
    init(view: NSView) {
        self.view = view
    }
    
    func size() async -> CGSize {
        await MainActor.run {
            view.frame.size
        }
    }
}

@_cdecl("SwiftAppWindow_SurfaceSize") public func SurfaceSize(context: UInt64, surface: UnsafeMutableRawPointer, ret: @convention(c) @Sendable (UInt64, Double, Double) -> ()) {
    let surface = Unmanaged<Surface>.fromOpaque(surface).takeUnretainedValue()
    Task {
        let size = await surface.size()
        ret(context, size.width, size.height)
    }
}

