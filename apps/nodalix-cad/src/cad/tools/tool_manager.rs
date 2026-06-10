use super::tool::{CADTool, ToolContext};
use std::collections::HashMap;

pub struct ToolManager {
    tools: HashMap<&'static str, Box<dyn CADTool>>,
    active_id: Option<&'static str>,
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolManager {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            active_id: None,
        }
    }

    pub fn register(&mut self, tool: Box<dyn CADTool>) {
        let id = tool.id();
        self.tools.insert(id, tool);
    }

    pub fn activate(&mut self, id: &'static str, ctx: &mut ToolContext<'_>) -> bool {
        if !self.tools.contains_key(id) {
            return false;
        }
        if let Some(current) = self.active_id {
            if let Some(tool) = self.tools.get_mut(current) {
                tool.deactivate(ctx);
            }
        }
        self.active_id = Some(id);
        if let Some(tool) = self.tools.get_mut(id) {
            tool.activate(ctx);
        }
        true
    }

    pub fn active_id(&self) -> Option<&'static str> {
        self.active_id
    }

    pub fn active_tool_mut(&mut self) -> Option<&mut (dyn CADTool + '_)> {
        let id = self.active_id?;
        if let Some(tool) = self.tools.get_mut(id) {
            Some(tool.as_mut())
        } else {
            None
        }
    }

    pub fn deactivate(&mut self, ctx: &mut ToolContext<'_>) {
        if let Some(id) = self.active_id {
            if let Some(tool) = self.tools.get_mut(id) {
                tool.deactivate(ctx);
            }
        }
        self.active_id = None;
    }
}
