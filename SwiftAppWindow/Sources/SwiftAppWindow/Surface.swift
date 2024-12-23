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
    var rawHandle: UnsafeMutableRawPointer {
        Unmanaged.passUnretained(view).toOpaque()
    }
}

@_cdecl("SwiftAppWindow_SurfaceSize") public func SurfaceSize(context: UInt64, surface: UnsafeMutableRawPointer, ret: @convention(c) @Sendable (UInt64, Double, Double) -> ()) {
    let surface = Unmanaged<Surface>.fromOpaque(surface).takeUnretainedValue()
    Task {
        let size = await surface.size()
        ret(context, size.width, size.height)
    }
}

@_cdecl("SwiftAppWindow_SurfaceRawHandle") public func RawHandle(surface: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    let surface = Unmanaged<Surface>.fromOpaque(surface).takeUnretainedValue()
    return surface.rawHandle
}

@_cdecl("SwiftAppWindow_SurfaceFree") public func SurfaceFree(surface: UnsafeMutableRawPointer) {
    Unmanaged<Surface>.fromOpaque(surface).release()
}

