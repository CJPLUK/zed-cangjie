use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use url::Url;
use zed_extension_api::{self as zed, serde_json, Extension, LanguageServerId, Worktree};

struct CangjieExtension {}

impl Extension for CangjieExtension {
    fn new() -> Self {
        Self {}
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed::Result<zed::Command> {
        let cangjie_home: String = get_cangjie_home(worktree)?;
        let zed_command = zed::Command {
            command: cangjie_home + "/tools/bin/LSPServer",
            args: vec![
                "src".into(),
                "--disableAutoImport".into(),
                "--enable-log=true".into(),
            ],
            env: vec![],
        };
        Ok(zed_command)
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed_extension_api::Result<Option<serde_json::Value>> {
        let modules_home_option: String = get_cangjie_home(worktree)?;
        let multi_module_option: Vec<_> = vec![module_option(worktree, "")?];
        Ok(Some(
            serde_json::to_value(InitializationOptions {
                modules_home_option,
                multi_module_option,
            })
            .unwrap_or_default(),
        ))
    }

    fn dap_locator_create_scenario(
        &mut self,
        locator_name: String,
        build_task: zed_extension_api::TaskTemplate,
        resolved_label: String,
        debug_adapter_name: String,
    ) -> Option<zed_extension_api::DebugScenario> {
        if build_task.command != "cjpm" {
            return None;
        }

        if build_task.args == vec!["run"] {
            return Some(zed::DebugScenario {
                adapter: debug_adapter_name,
                label: resolved_label.clone(),
                config: "{}".into(),
                tcp_connection: None,
                build: Some(zed::BuildTaskDefinition::Template(
                    zed::BuildTaskDefinitionTemplatePayload {
                        template: zed::BuildTaskTemplate {
                            label: "cjpm build".into(),
                            command: "cjpm".into(),
                            args: vec!["build".into(), "-g".into()],
                            env: build_task.env,
                            cwd: build_task.cwd,
                        },
                        locator_name: Some(locator_name),
                    },
                )),
            });
        }

        if build_task.args[0] == "test" {
            let config = serde_json::Value::Null;
            let Ok(config) = serde_json::to_string(&config) else {
                return None;
            };
            return Some(zed::DebugScenario {
                adapter: debug_adapter_name,
                label: resolved_label.clone(),
                config,
                tcp_connection: None,
                build: Some(zed::BuildTaskDefinition::Template(
                    zed::BuildTaskDefinitionTemplatePayload {
                        template: zed::BuildTaskTemplate {
                            label: "cjpm test --no-run -g".into(),
                            command: "cjpm".into(),
                            args: build_task
                                .args
                                .into_iter()
                                .chain(["--no-run".to_string(), "-g".to_string()].into_iter())
                                .collect(),
                            env: build_task.env,
                            cwd: build_task.cwd,
                        },
                        locator_name: Some(locator_name),
                    },
                )),
            });
        }

        None
    }

    fn run_dap_locator(
        &mut self,
        _locator_name: String,
        build_task: zed_extension_api::TaskTemplate,
    ) -> zed_extension_api::Result<zed_extension_api::DebugRequest, String> {
        if build_task.command != "cjpm" {
            return Err("Only supports cjpm tasks".into());
        }

        if build_task.args == vec!["build", "-g"] {
            return Ok(zed::DebugRequest::Launch(zed::LaunchRequest {
                program: "target/debug/bin/main".into(),
                cwd: build_task.cwd,
                args: vec![],
                envs: build_task.env.into_iter().collect(),
            }));
        }

        if build_task.args[0] == "test" {
            if !build_task.args.iter().any(|arg| arg == "-g") {
                return Err("cjpm test tasks only supported with '-g' flag".into());
            }
            let target_dir = build_task
                .args
                .iter()
                .filter_map(|arg| {
                    if arg.starts_with("--target-dir=") && arg.len() > 13 {
                        Some(&arg[13..])
                    } else {
                        None
                    }
                })
                .last()
                .unwrap_or("target");
            return Ok(zed::DebugRequest::Launch(zed::LaunchRequest {
                program: format!("{target_dir}/debug/unittest_bin/$ZED_CUSTOM_pkgname"),
                cwd: build_task.cwd,
                // only use argument which we know that `cjpm test` passes onto the executable (best effort):
                args: build_task
                    .args
                    .into_iter()
                    .filter(|arg| {
                        arg.starts_with("--filter=")
                            || arg.starts_with("--include-tags=")
                            || arg.starts_with("--exclude-tags=")
                            || arg.starts_with("--random-seed=")
                            || arg == "--capture-output"
                            || arg == "--no-capture-output"
                            || arg == "--show-tags"
                            || arg == "--no-color"
                            || arg == "--no-progress"
                            || arg == "--progress-brief"
                            || arg == "--progress-entries-limit"
                            || arg == "--verbose"
                    })
                    .collect(),
                envs: build_task.env,
            }));
        }

        Err("cjpm task not recognised".into())
    }
}

fn module_option(worktree: &Worktree, cjpm_rel_path: &str) -> zed::Result<(String, ModuleConfig)> {
    let worktree_path = worktree.root_path();
    let file_uri = Url::from_file_path(worktree_path.clone() + "/" + cjpm_rel_path + "/cjpm.toml")
        .map_err(|_| "cjpm.toml not found".to_string())?
        .to_string();
    let cjpm_contents =
        //worktree.read_text_file(&("./".to_string() + cjpm_rel_path + "/cjpm.toml"))?;
        worktree.read_text_file("cjpm.toml")?;
    #[derive(Deserialize)]
    struct CjpmFile {
        package: Package,
    }

    #[derive(Deserialize)]
    struct Package {
        name: String,
        #[serde(default = "default_src_path", rename = "src-dir")]
        src_path: String,
    }

    let cjpm_file: CjpmFile = toml::from_str(&cjpm_contents)
        .map_err(|error| format!("failed to parse {cjpm_rel_path}: {error}"))?;

    Ok((
        file_uri,
        ModuleConfig {
            name: cjpm_file.package.name,
            src_path: Url::from_file_path(
                worktree_path + cjpm_rel_path + &cjpm_file.package.src_path,
            )
            .map_err(|_| "error with src directory")?
            .to_string(),
        },
    ))
}

fn default_src_path() -> String {
    "src".to_string()
}

fn get_cangjie_home(worktree: &Worktree) -> zed::Result<String> {
    zed::settings::LspSettings::for_worktree("cangjie-lsp", worktree)
        .ok()
        .and_then(|lsp_settings| lsp_settings.binary)
        .and_then(|binary_settings| binary_settings.env)
        .and_then(|env| env.get("CANGJIE_HOME").map(|t| t.to_string()))
        .or_else(|| {
            // fallback to shellenv
            worktree
                .shell_env()
                .iter()
                .filter(|pair| pair.0 == "CANGJIE_HOME")
                .next()
                .map(|pair| pair.1.to_owned())
        })
        .ok_or("get_cangjie_home: CANGJIE_HOME must be defined in environment".to_string())
}

#[derive(Serialize, Deserialize)]
struct ModuleConfig {
    name: String,
    src_path: String,
}

fn serialize_pairs_as_map<S, T>(pairs: &Vec<(String, T)>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    let mut map = serializer.serialize_map(Some(pairs.len()))?;
    for (key, value) in pairs {
        map.serialize_entry(key, value)?;
    }
    map.end()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InitializationOptions {
    modules_home_option: String,
    // std_lib_path_option
    #[serde(serialize_with = "serialize_pairs_as_map")]
    multi_module_option: Vec<(String, ModuleConfig)>,
}

zed::register_extension!(CangjieExtension);
