//! 已装 Agent 扫描：遍历 Adapter registry → detect() → 清单。

use crate::model::agent::AgentInfo;
use crate::registry::Registry;

pub fn scan(registry: &Registry) -> Vec<AgentInfo> {
    use crate::model::runtime::LaunchOptions;
    registry
        .list()
        .iter()
        .map(|adapter| {
            let runtime = adapter.runtime();
            let (chat, prompt_inject, launch_options, streaming) = match runtime {
                Some(spec) => (
                    true,
                    Some(spec.prompt_inject),
                    Some(LaunchOptions {
                        model: spec.model,
                        effort: spec.effort,
                    }),
                    spec.structured.is_some(),
                ),
                None => (false, None, None, false),
            };
            let program = adapter.program_hint();
            match adapter.detect() {
                Some(info) => AgentInfo {
                    id: info.id,
                    name: info.name,
                    installed: true,
                    version: info.version,
                    config_root: Some(info.config_root),
                    program,
                    capabilities: adapter.capabilities(),
                    chat,
                    prompt_inject,
                    launch_options,
                    streaming,
                },
                None => AgentInfo {
                    id: adapter.id().to_string(),
                    name: adapter.name().to_string(),
                    installed: false,
                    version: None,
                    config_root: None,
                    program,
                    capabilities: adapter.capabilities(),
                    chat,
                    prompt_inject,
                    launch_options,
                    streaming,
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AgentAdapter, Caps, CommandSpec};
    use crate::error::Result;
    use crate::model::agent::InstallInfo;
    use crate::model::mcp::{McpServer, Scope};
    use std::path::{Path, PathBuf};

    struct InstalledAdapter;
    impl AgentAdapter for InstalledAdapter {
        fn id(&self) -> &'static str {
            "installed"
        }
        fn name(&self) -> &'static str {
            "Installed"
        }
        fn detect(&self) -> Option<InstallInfo> {
            Some(InstallInfo {
                id: "installed".into(),
                name: "Installed".into(),
                version: Some("1.0".into()),
                config_root: "/tmp/x".into(),
            })
        }
        fn capabilities(&self) -> Caps {
            Caps {
                mcp: true,
                ..Default::default()
            }
        }
        fn mcp_path(&self, _scope: &Scope) -> Option<PathBuf> {
            None
        }
        fn read_mcp(&self, _raw: &str) -> Result<Vec<McpServer>> {
            Ok(vec![])
        }
        fn render_mcp(&self, _current: Option<&str>, _servers: &[McpServer]) -> Result<String> {
            Ok(String::new())
        }
        fn launch_cmd(&self, project_dir: &Path) -> CommandSpec {
            CommandSpec {
                program: "x".into(),
                args: vec![],
                working_dir: project_dir.to_path_buf(),
            }
        }
    }

    struct MissingAdapter;
    impl AgentAdapter for MissingAdapter {
        fn id(&self) -> &'static str {
            "missing"
        }
        fn name(&self) -> &'static str {
            "Missing"
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
        fn read_mcp(&self, _raw: &str) -> Result<Vec<McpServer>> {
            Ok(vec![])
        }
        fn render_mcp(&self, _current: Option<&str>, _servers: &[McpServer]) -> Result<String> {
            Ok(String::new())
        }
        fn launch_cmd(&self, project_dir: &Path) -> CommandSpec {
            CommandSpec {
                program: "x".into(),
                args: vec![],
                working_dir: project_dir.to_path_buf(),
            }
        }
    }

    #[test]
    fn scan_reports_installed_and_missing() {
        let reg = Registry::new(vec![Box::new(InstalledAdapter), Box::new(MissingAdapter)]);
        let infos = scan(&reg);
        assert_eq!(infos.len(), 2);
        assert!(infos[0].installed);
        assert_eq!(infos[0].config_root.as_deref(), Some("/tmp/x"));
        assert!(!infos[1].installed);
        assert!(infos[1].config_root.is_none());
    }
}
