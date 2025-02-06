//SPDX-License-Identifier: MPL-2.0

//
//  AsyncBridge.swift
//  SwiftAppWindow
//
//  Created by Drew Crawford on 12/22/24.
//

func asyncBridge<I: AnyObject & Sendable,R: AnyObject>(context: UInt64, input: UnsafeMutableRawPointer, inputType:I.Type,  ret: @convention(c) @Sendable (UInt64, UnsafeMutableRawPointer) -> (), operation: @Sendable @escaping (I) async -> R) {
    let input = Unmanaged<I>.fromOpaque(input).takeUnretainedValue()
    Task {
        let result = await operation(input)
        let unmanaged = Unmanaged.passRetained(result).toOpaque()
        ret(context, unmanaged)
    }
}
