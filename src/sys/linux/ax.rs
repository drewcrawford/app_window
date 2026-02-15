// SPDX-License-Identifier: MPL-2.0
use super::{BUTTON_WIDTH, CLOSE_ID, MAXIMIZE_ID, MINIMIZE_ID, TITLEBAR_HEIGHT};
use crate::coordinates::Size;

use crate::sys::window::WindowInternal;
use accesskit::{Action, ActionRequest, NodeId, Rect, Role, TreeId, TreeUpdate};
use std::sync::{Arc, Mutex};

pub fn build_tree_update(title: String, window_size: Size) -> TreeUpdate {
    let mut window = accesskit::Node::new(Role::Window);
    window.set_label(title);
    //accesskit rect is min and max, not origin and height!
    window.set_bounds(Rect::new(
        0.0,
        0.0,
        window_size.width(),
        window_size.height(),
    ));
    let mut title_bar = accesskit::Node::new(Role::TitleBar);
    title_bar.set_label("app_window");
    title_bar.set_bounds(Rect::new(
        0.0,
        0.0,
        window_size.width(),
        TITLEBAR_HEIGHT as f64,
    ));
    let mut close_button = accesskit::Node::new(Role::Button);
    close_button.add_action(Action::Click);
    close_button.add_action(Action::Focus);

    close_button.set_bounds(Rect::new(
        window_size.width() - BUTTON_WIDTH as f64,
        0.0,
        window_size.width(),
        TITLEBAR_HEIGHT as f64,
    ));
    close_button.set_label("Close");

    let mut maximize_button = accesskit::Node::new(Role::Button);
    maximize_button.add_action(Action::Click);
    maximize_button.add_action(Action::Focus);
    maximize_button.set_bounds(Rect::new(
        window_size.width() - BUTTON_WIDTH as f64 * 2.0,
        0.0,
        window_size.width() - BUTTON_WIDTH as f64 * 1.0,
        TITLEBAR_HEIGHT as f64,
    ));
    maximize_button.set_label("Maximize");

    let mut minimize_button = accesskit::Node::new(Role::Button);
    minimize_button.add_action(Action::Click);
    minimize_button.add_action(Action::Focus);
    minimize_button.set_bounds(Rect::new(
        window_size.width() - BUTTON_WIDTH as f64 * 3.0,
        0.0,
        window_size.width() - BUTTON_WIDTH as f64 * 2.0,
        TITLEBAR_HEIGHT as f64,
    ));
    minimize_button.set_label("Minimize");

    //window.set_children(vec![NodeId(2)]);
    //title_bar.set_children(vec![NodeId(3),NodeId(4), NodeId(5)]);
    window.set_children(vec![CLOSE_ID, MINIMIZE_ID, MAXIMIZE_ID]);

    let tree = accesskit::Tree {
        root: NodeId(1),
        toolkit_name: Some("app_window".to_string()),
        toolkit_version: Some("0.1.0".to_string()),
    };

    accesskit::TreeUpdate {
        nodes: vec![
            (NodeId(1), window),
            /*(NodeId(2), title_bar),*/ (CLOSE_ID, close_button),
            (MAXIMIZE_ID, maximize_button),
            (MINIMIZE_ID, minimize_button),
        ],
        tree: Some(tree),
        tree_id: TreeId::ROOT,
        focus: NodeId(1),
    }
}

pub struct Inner {
    window_size: Size,
    title: String,
}

#[derive(Clone)]
pub(super) struct AX {
    inner: Arc<Inner>,
    window_internal: Arc<Mutex<WindowInternal>>,
}

impl AX {
    pub fn new(
        window_size: Size,
        title: String,
        window_internal: Arc<Mutex<WindowInternal>>,
    ) -> Self {
        AX {
            inner: Arc::new(Inner { window_size, title }),
            window_internal,
        }
    }
}

impl accesskit::ActivationHandler for AX {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        Some(build_tree_update(
            self.inner.title.clone(),
            self.inner.window_size,
        ))
    }
}

impl accesskit::ActionHandler for AX {
    fn do_action(&mut self, request: ActionRequest) {
        if request.target_node == CLOSE_ID {
            match request.action {
                Action::Click => {
                    self.window_internal.lock().unwrap().close_window();
                }
                _ => unimplemented!(),
            }
        } else if request.target_node == MAXIMIZE_ID {
            match request.action {
                Action::Click => {
                    self.window_internal.lock().unwrap().maximize();
                }
                _ => unimplemented!(),
            }
        } else if request.target_node == MINIMIZE_ID {
            match request.action {
                Action::Click => {
                    self.window_internal.lock().unwrap().minimize();
                }
                _ => unimplemented!(),
            }
        } else {
            unimplemented!(
                "Unknown action target: {target:?}",
                target = request.target_node
            );
        }
    }
}

impl accesskit::DeactivationHandler for AX {
    fn deactivate_accessibility(&mut self) {
        todo!()
    }
}
