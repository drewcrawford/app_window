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
}

