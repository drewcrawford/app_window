//SPDX-License-Identifier: MPL-2.0

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

@_cdecl("SwiftAppWindow_OnMainThread")
public func OnMainThread(context: UInt64, cfn: @escaping @convention(c) @Sendable (UInt64) -> Void) {
    Task {
        await MainActor.run {
            cfn(context)
        }
    }
}
