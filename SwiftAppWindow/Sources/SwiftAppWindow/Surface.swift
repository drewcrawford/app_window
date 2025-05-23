//SPDX-License-Identifier: MPL-2.0

//
//  Surface.swift
//  SwiftAppWindow
//
//  Created by Drew Crawford on 12/22/24.
//
import AppKit

final class SurfaceView: NSView {
    var sizeNotify: ((CGFloat, CGFloat) -> ())?
    override func layout() {
        super.layout()
        let scale = window?.backingScaleFactor ?? 1.0
        sizeNotify?(frame.width * scale, frame.height * scale)
    }
}

public final class Surface: Sendable {
    let view: SurfaceView
    
    init(view: SurfaceView) {
        self.view = view
    }
    
    func size() async -> CGSize {
        await MainActor.run {
            let scale = view.window?.backingScaleFactor ?? 1.0
            return CGSize(width: view.frame.width * scale, height: view.frame.height * scale)
        }
    }
    var rawHandle: UnsafeMutableRawPointer {
        Unmanaged.passUnretained(view).toOpaque()
    }
    func sizeUpdate(notify: @escaping @Sendable (CGFloat, CGFloat) -> ()) {
        Task {
            await MainActor.run {
                view.sizeNotify = notify
            }
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

@_cdecl("SwiftAppWindow_SurfaceRawHandle") public func RawHandle(surface: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    let surface = Unmanaged<Surface>.fromOpaque(surface).takeUnretainedValue()
    return surface.rawHandle
}

@_cdecl("SwiftAppWindow_SurfaceFree") public func SurfaceFree(surface: UnsafeMutableRawPointer) {
    Unmanaged<Surface>.fromOpaque(surface).release()
}

@_cdecl("SwiftAppWindow_SurfaceSizeUpdate") public func SurfaceSizeUpdate(ctx: UInt64, surface: UnsafeMutableRawPointer, notify: @Sendable @convention(c) (UInt64, CGFloat, CGFloat) -> ()) {
    Unmanaged<Surface>.fromOpaque(surface).takeUnretainedValue().sizeUpdate(notify: {
        notify(ctx, $0, $1)
    })
}
