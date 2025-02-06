//SPDX-License-Identifier: MPL-2.0

//
//  Coordinates.swift
//  SwiftAppWindow
//
//  Created by Drew Crawford on 12/22/24.
//

import AppKit
extension NSPoint {
    init(rustX: CGFloat, rustY: CGFloat, outerBounds: CGRect) {
        self.init(x: rustX, y: outerBounds.height - rustY)
    }
}
