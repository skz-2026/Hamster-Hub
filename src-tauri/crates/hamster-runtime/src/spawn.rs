//! 启动解析（纯函数）：RuntimeSpec + 首条 prompt + 启动覆盖项 → 实际 spawn 的 (program, args)。
//!
//! 规则（runtime-design.md §3.4/§3.5）：
//! - `PromptInject::Argv` 且 prompt 非空 → 追加为位置参数；`None` → 忽略 prompt
//! - 模型/推理强度覆盖项 → 按 LaunchOption 的 arg_template（含 {v}，按空格拆 token）
//!   展开为官方 CLI 参数（红线①：argv 是官方接口，不注入）
//! - Windows 且 spec.windows_shim（npm 的 .cmd shim）→ `cmd.exe /c <program> <args…>`
//!   （CreateProcess 不直接运行批处理文件；cmd /c 对 .exe 同样可达）

use hamster_core::{LaunchOverrides, PromptInject, RuntimeSpec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnPlan {
    pub program: String,
    pub args: Vec<String>,
}

fn expand_option(args: &mut Vec<String>, template: &str, value: &str) {
    for token in template.split_whitespace() {
        args.push(token.replace("{v}", value));
    }
}

pub fn plan_spawn(
    spec: &RuntimeSpec,
    first_prompt: Option<&str>,
    overrides: &LaunchOverrides,
    is_windows: bool,
) -> SpawnPlan {
    let mut args = spec.args.clone();

    if let (Some(model), Some(value)) = (spec.model.as_ref(), overrides.model.as_deref()) {
        if !value.trim().is_empty() {
            expand_option(&mut args, &model.arg_template, value.trim());
        }
    }
    if let (Some(effort), Some(value)) = (spec.effort.as_ref(), overrides.effort.as_deref()) {
        if !value.trim().is_empty() {
            expand_option(&mut args, &effort.arg_template, value.trim());
        }
    }

    if let (Some(template), Some(key)) =
        (spec.resume_args.as_ref(), overrides.resume_key.as_deref())
    {
        let key = key.trim();
        if !key.is_empty() {
            for token in template.split_whitespace() {
                args.push(token.replace("{key}", key));
            }
        }
    }

    if spec.prompt_inject == PromptInject::Argv {
        if let Some(prompt) = first_prompt {
            let prompt = prompt.trim();
            if !prompt.is_empty() {
                args.push(prompt.to_string());
            }
        }
    }

    if is_windows && spec.windows_shim {
        let mut wrapped = vec!["/c".to_string(), spec.program.clone()];
        wrapped.extend(args);
        SpawnPlan {
            program: "cmd.exe".to_string(),
            args: wrapped,
        }
    } else {
        SpawnPlan {
            program: spec.program.clone(),
            args,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hamster_core::LaunchOption;

    fn spec(shim: bool, inject: PromptInject) -> RuntimeSpec {
        RuntimeSpec {
            program: "claude".to_string(),
            args: vec![],
            windows_shim: shim,
            prompt_inject: inject,
            structured: None,
            model: Some(LaunchOption {
                choices: vec!["opus".into(), "sonnet".into()],
                arg_template: "--model {v}".into(),
                default: None,
                stream_override: false,
            }),
            effort: Some(LaunchOption {
                choices: vec!["low".into(), "high".into()],
                arg_template: "-c effort={v}".into(),
                default: None,
                stream_override: false,
            }),
            resume_args: Some("--resume {key}".into()),
            task_mode: None,
        }
    }

    #[test]
    fn resume_key_expands_template() {
        let ov = LaunchOverrides {
            resume_key: Some("abc-123".into()),
            ..Default::default()
        };
        let plan = plan_spawn(&spec(false, PromptInject::None), None, &ov, false);
        assert_eq!(plan.args, vec!["--resume", "abc-123"]);
    }

    #[test]
    fn argv_prompt_appended() {
        let plan = plan_spawn(
            &spec(true, PromptInject::Argv),
            Some("修登录 bug"),
            &LaunchOverrides::default(),
            true,
        );
        assert_eq!(plan.program, "cmd.exe");
        assert_eq!(plan.args, vec!["/c", "claude", "修登录 bug"]);
    }

    #[test]
    fn prompt_none_ignored() {
        let plan = plan_spawn(
            &spec(true, PromptInject::None),
            Some("hi"),
            &LaunchOverrides::default(),
            true,
        );
        assert_eq!(plan.args, vec!["/c", "claude"]);
    }

    #[test]
    fn overrides_expand_templates_in_order() {
        let ov = LaunchOverrides {
            model: Some("opus".into()),
            effort: Some("high".into()),
            resume_key: None,
        };
        let plan = plan_spawn(&spec(false, PromptInject::Argv), Some("hi"), &ov, false);
        assert_eq!(
            plan.args,
            vec!["--model", "opus", "-c", "effort=high", "hi"]
        );
    }

    #[test]
    fn blank_override_ignored() {
        let ov = LaunchOverrides {
            model: Some("  ".into()),
            effort: None,
            resume_key: None,
        };
        let plan = plan_spawn(&spec(false, PromptInject::None), None, &ov, false);
        assert_eq!(plan.args, Vec::<String>::new());
    }

    #[test]
    fn no_shim_on_unix_like() {
        let plan = plan_spawn(
            &spec(true, PromptInject::Argv),
            Some("hi"),
            &LaunchOverrides::default(),
            false,
        );
        assert_eq!(plan.program, "claude");
        assert_eq!(plan.args, vec!["hi"]);
    }

    #[test]
    fn native_program_not_wrapped_without_shim_flag() {
        let plan = plan_spawn(
            &spec(false, PromptInject::None),
            None,
            &LaunchOverrides::default(),
            true,
        );
        assert_eq!(plan.program, "claude");
    }
}
