use std::collections::HashMap;

use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use url::Url;
use zed_extension_api::{self as zed, serde_json, Extension, LanguageServerId, Worktree};

const ZED_CANGJIE_OUTPUT_PREFIX: &'static str = "Cangjie PL extension message:";

fn log(message: &str) {
    println!("{ZED_CANGJIE_OUTPUT_PREFIX} {message}")
}

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

fn default_src_path() -> String {
    "src".to_string()
}

struct CJPMProject {
    cjpm_dir: String,
    src_dir: String,
    cjpm_data: CjpmFile,
}

impl CJPMProject {
    /// `cjpm_rel_path` is the relative path to the subdirectory of the workspace
    /// which contains a `cjpm.toml` file.
    /// Starts with `./` and ends with `/`
    fn parse(worktree: &Worktree, cjpm_rel_path: &str) -> zed::Result<Self> {
        let worktree_path = worktree.root_path();
        let cjpm_dir: String = worktree_path.clone() + cjpm_rel_path;
        let cjpm_contents = worktree.read_text_file(&(cjpm_rel_path.to_string() + "cjpm.toml"))?;
        let cjpm_data: CjpmFile = toml::from_str(&cjpm_contents)
            .map_err(|error| format!("failed to parse {cjpm_rel_path}cjpm.toml: {error}"))?;
        let src_dir: String = worktree_path + "/" + cjpm_rel_path + &cjpm_data.package.src_path;

        Ok(CJPMProject {
            cjpm_dir,
            src_dir,
            cjpm_data,
        })
    }
}

struct CangjieExtension {
    cjpm_projects_catalog: HashMap<u64, Vec<CJPMProject>>,
}

impl CangjieExtension {
    fn worktree_projects(&mut self, worktree: &Worktree) -> &Vec<CJPMProject> {
        // absolute path to root of workspace, with trailing `/`
        let root_path = worktree.root_path();
        // relative paths (from workspace root) of directories in workspace
        // containing a `cjpm.toml` file (starting with `./` and ends with `/`)
        let rel_paths = zed::Command::new("find")
            .args([&root_path, "-name", "cjpm.toml"])
            .output()
            .map_err(|err| {
                log(&format!("could not execute `find` -- {err}"));
                err
            })
            // Reaching following requires appropriate capability declaration
            // in extension manifest and explicit approval in user settings.
            .and_then(|output| {
                String::from_utf8(output.stdout).map_err(|_| {
                    log("error decoding `find` output to string");
                    "utf8 decoding error of `find` command output".to_string()
                })
            })
            .map(|str| {
                str.split("\n")
                    .filter_map(|line| {
                        Some(format!(
                            ".{}",
                            line.strip_prefix(&root_path)?.strip_suffix("cjpm.toml")?,
                        ))
                    })
                    .collect::<Vec<_>>()
            })
            // Without approved capability, fall back to assuming that the workspace
            // is at the root of a Cangjie/CJPM project
            .unwrap_or(vec!["./".to_string()]);
        for p in &rel_paths {
            log(&format!("cjpm.toml found relative to workspace at: {p}"));
        }

        let out = self
            .cjpm_projects_catalog
            .entry(worktree.id())
            .insert_entry(
                rel_paths
                    .iter()
                    .filter_map(|rel_path| CJPMProject::parse(worktree, rel_path).ok())
                    .collect(),
            )
            .into_mut();
        out
    }
}

fn module_option(cjpm_proj: &CJPMProject) -> zed::Result<(String, ModuleConfig)> {
    let file_uri = Url::from_file_path(cjpm_proj.cjpm_dir.clone() + "/cjpm.toml")
        .map_err(|_| "cjpm.toml not found".to_string())?
        .to_string();
    let src_path = Url::from_file_path(&cjpm_proj.src_dir)
        .map_err(|_| "error with src directory".to_string())?
        .to_string();

    Ok((
        file_uri,
        ModuleConfig {
            name: cjpm_proj.cjpm_data.package.name.clone(),
            src_path,
        },
    ))
}

impl Extension for CangjieExtension {
    fn new() -> Self {
        Self {
            cjpm_projects_catalog: Default::default(),
        }
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
        let multi_module_option: Vec<_> = self
            .worktree_projects(worktree)
            .iter()
            .filter_map(|cjpm_proj| module_option(cjpm_proj).ok())
            .collect();

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
