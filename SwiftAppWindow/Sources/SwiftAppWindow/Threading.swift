//
//  Threading.swift
//  SwiftAppWindow
//
//  Created by Drew Crawford on 12/22/24.
//
import Foundation
import AppKit

@_cdecl("SwiftAppWindowIsMainThread")
public func IsMainThread() -> Bool {
    Thread.current.isMainThread
}

@_cdecl("SwiftAppWindowRunMainThread")
public func RunMainThread() {
    MainActor.assumeIsolated {
        NSApplication.shared.run()
    }
}
