pub mod gui;
pub mod mainview;

use crossbeam::channel;

use handlegraph::handle::NodeId;

use crate::geometry::*;
use crate::input::MousePos;
use crate::view::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMsg {
    SelectNode(Option<NodeId>),
    HoverNode(Option<NodeId>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppConfigMsg {
    ToggleSelectionEdgeDetect,
    ToggleSelectionEdgeBlur,
    ToggleSelectionOutline,
}

pub struct App {
    mouse_pos: MousePos,
    screen_dims: ScreenDims,

    hover_node: Option<NodeId>,
    selected_node: Option<NodeId>,

    pub selection_edge_detect: bool,
    pub selection_edge_blur: bool,
    pub selection_edge: bool,
}

impl App {
    pub fn new<Dims: Into<ScreenDims>>(
        mouse_pos: MousePos,
        screen_dims: Dims,
    ) -> Self {
        Self {
            mouse_pos,
            screen_dims: screen_dims.into(),
            hover_node: None,
            selected_node: None,

            selection_edge_detect: true,
            selection_edge_blur: true,
            selection_edge: true,
        }
    }

    pub fn hover_node(&self) -> Option<NodeId> {
        self.hover_node
    }

    pub fn selected_node(&self) -> Option<NodeId> {
        self.selected_node
    }

    pub fn dims(&self) -> ScreenDims {
        self.screen_dims
    }

    pub fn mouse_pos(&self) -> Point {
        self.mouse_pos.read()
    }

    pub fn update_dims<Dims: Into<ScreenDims>>(&mut self, screen_dims: Dims) {
        self.screen_dims = screen_dims.into();
    }

    pub fn apply_app_msg(&mut self, msg: &AppMsg) {
        match msg {
            AppMsg::SelectNode(id) => self.selected_node = *id,
            AppMsg::HoverNode(id) => self.hover_node = *id,
        }
    }

    pub fn apply_app_config_msg(&mut self, msg: &AppConfigMsg) {
        match msg {
            AppConfigMsg::ToggleSelectionEdgeDetect => {
                self.selection_edge_detect = !self.selection_edge_detect
            }
            AppConfigMsg::ToggleSelectionEdgeBlur => {
                self.selection_edge_blur = !self.selection_edge_blur
            }
            AppConfigMsg::ToggleSelectionOutline => {
                self.selection_edge = !self.selection_edge
            }
        }
    }
}
