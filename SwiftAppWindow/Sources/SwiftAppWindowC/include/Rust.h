// SPDX-License-Identifier: MPL-2.0
//
//  Rust.h
//  SwiftRawInput
//
//  Created by Drew Crawford on 12/16/24.
//
#include <stdbool.h>
#include <stdint.h>

extern void raw_input_finish_key_event_context(const void* context);
extern void raw_input_finish_mouse_event_context(const void* context);
extern void raw_input_key_notify_func(const void *context, void *window, uint16_t keyCode, bool pressed);
extern void raw_input_mouse_move(const void *context, void *window, double windowPosX, double windowPosY, double windowWidth, double windowHeight);
extern void raw_input_mouse_button(const void *context, void *window, uint8_t button, bool down);
extern void raw_input_mouse_scroll(const void *context, void *window, double deltaX, double deltaY);
