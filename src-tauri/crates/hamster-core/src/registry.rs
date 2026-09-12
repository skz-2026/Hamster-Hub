//! Adapter 注册表。core 面向 trait 编程，具体 Adapter 由 adapters crate 注入。

use crate::adapter::AgentAdapter;
use crate::error::{HamsterError, Result};

pub struct Registry {
    adapters: Vec<Box<dyn AgentAdapter>>,
}

impl Registry {
    pub fn new(adapters: Vec<Box<dyn AgentAdapter>>) -> Self {
        Self { adapters }
    }

    pub fn get(&self, id: &str) -> Result<&dyn AgentAdapter> {
        self.adapters
            .iter()
            .map(|a| a.as_ref())
            .find(|a| a.id() == id)
            .ok_or_else(|| HamsterError::UnknownAdapter(id.to_string()))
    }

    pub fn list(&self) -> &[Box<dyn AgentAdapter>] {
        &self.adapters
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AgentAdapter, Caps, CommandSpec};
    use crate::model::agent::InstallInfo;
    use crate::model::mcp::{McpServer, Scope};
    use std::path::{Path, PathBuf};

    struct FakeAdapter;
    impl AgentAdapter for FakeAdapter {
        fn id(&self) -> &'static str {
            "fake"
        }
        fn name(&self) -> &'static str {
            "Fake"
        }
        fn detect(&self) -> Option<InstallInfo> {
            None
        }
        fn capabilities(&self) -> Caps {
            Caps::default()
        }
        fn mcp_path(&self, _scope: &Scope) -> Option<PathBuf> {
            None
        }
        fn read_mcp(&self, _raw: &str) -> crate::error::Result<Vec<McpServer>> {
            Ok(vec![])
        }
        fn render_mcp(&self, _current: Option<&str>, _servers: &[McpServer]) -> Result<String> {
            Ok(String::new())
        }
        fn launch_cmd(&self, project_dir: &Path) -> CommandSpec {
            CommandSpec {
                program: "fake".into(),
                args: vec![],
                working_dir: project_dir.to_path_buf(),
            }
        }
    }

    #[test]
    fn get_unknown_returns_error() {
        let reg = Registry::new(vec![Box::new(FakeAdapter)]);
        assert!(matches!(
            reg.get("nope"),
            Err(HamsterError::UnknownAdapter(_))
        ));
        assert!(reg.get("fake").is_ok());
    }
}
