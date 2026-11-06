use std::collections::HashMap;

use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use url::Url;
use zed_extension_api::{self as zed, serde_json, Extension, LanguageServerId, Worktree};

const ZED_CANGJIE_OUTPUT_PREFIX: &'static str = "Cangjie PL extension message:";

/// Install locations of Python 3.11 from the python.org installer, Homebrew on Apple Silicon,
/// and Homebrew on Intel respectively.
const PYTHON_FRAMEWORK_DIRECTORIES_FALLBACK_MACOS: [&str; 3] = [
    "/Library/Frameworks",
    "/opt/homebrew/opt/python@3.11/Frameworks",
    "/usr/local/opt/python@3.11/Frameworks",
];

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

    fn get_dap_binary(
        &mut self,
        adapter_name: String,
        config: zed_extension_api::DebugTaskDefinition,
        user_provided_debug_adapter_path: Option<String>,
        worktree: &Worktree,
    ) -> zed_extension_api::Result<zed_extension_api::DebugAdapterBinary, String>
    {
        let cangjie_home: String = get_cangjie_home(worktree)?;
        let command = user_provided_debug_adapter_path
            .unwrap_or_else(|| format!("{cangjie_home}/third_party/llvm/bin/lldb-vscode"));

        let mut configuration: serde_json::Value = serde_json::from_str(&config.config)
            .map_err(|error| format!("invalid debug configuration: {error}"))?;
        let configuration_object = configuration
            .as_object_mut()
            .ok_or("debug configuration must be a JSON object")?;
        configuration_object
            .entry("cwd")
            .or_insert(worktree.root_path().into());
        // lldb-vscode resolves a relative `program` against its own working directory.
        let cwd = configuration_object
            .get("cwd")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| worktree.root_path());

        let request = self.dap_request_kind(adapter_name, configuration.clone())?;

        let mut envs = vec![library_path_env(&cangjie_home, worktree)?];
        if zed::current_platform().0 == zed::Os::Mac {
            // The SDK's liblldb links /Library/Frameworks/Python.framework/Versions/3.11/Python,
            // and Zed doesn't forward `dap.<adapter>.env` to extension adapters. Nonexistent
            // directories in the fallback are skipped by dyld.
            let framework_path = lsp_settings_env_var(worktree, "DYLD_FRAMEWORK_PATH")
                .unwrap_or_else(|| PYTHON_FRAMEWORK_DIRECTORIES_FALLBACK_MACOS.join(":"));
            envs.push(("DYLD_FRAMEWORK_PATH".to_string(), framework_path));
        }

        Ok(zed::DebugAdapterBinary {
            command: Some(command),
            arguments: vec![],
            envs,
            cwd: Some(cwd),
            connection: None,
            request_args: zed::StartDebuggingRequestArguments {
                configuration: configuration.to_string(),
                request,
            },
        })
    }

    fn dap_request_kind(
        &mut self,
        _adapter_name: String,
        config: serde_json::Value,
    ) -> zed_extension_api::Result<zed_extension_api::StartDebuggingRequestArgumentsRequest, String>
    {
        match config.get("request").and_then(serde_json::Value::as_str) {
            Some("launch") => Ok(zed::StartDebuggingRequestArgumentsRequest::Launch),
            Some("attach") => Ok(zed::StartDebuggingRequestArgumentsRequest::Attach),
            _ => Err(
                "missing or invalid `request` field in config. Expected 'launch' or 'attach'"
                    .into(),
            ),
        }
    }

    fn dap_config_to_scenario(
        &mut self,
        config: zed_extension_api::DebugConfig,
    ) -> zed_extension_api::Result<zed_extension_api::DebugScenario, String> {
        let mut configuration = serde_json::Map::new();
        match config.request {
            zed::DebugRequest::Attach(attach) => {
                configuration.insert("request".into(), "attach".into());
                if let Some(process_id) = attach.process_id {
                    configuration.insert("pid".into(), process_id.into());
                }
            }
            zed::DebugRequest::Launch(launch) => {
                configuration.insert("request".into(), "launch".into());
                configuration.insert("program".into(), launch.program.into());
                if !launch.args.is_empty() {
                    configuration.insert("args".into(), launch.args.into());
                }
                // Unlike CodeLLDB, lldb-vscode expects `env` as a list of "KEY=VALUE" strings.
                if !launch.envs.is_empty() {
                    let env: Vec<String> = launch
                        .envs
                        .into_iter()
                        .map(|(key, value)| format!("{key}={value}"))
                        .collect();
                    configuration.insert("env".into(), env.into());
                }
                if let Some(stop_on_entry) = config.stop_on_entry {
                    configuration.insert("stopOnEntry".into(), stop_on_entry.into());
                }
                if let Some(cwd) = launch.cwd {
                    configuration.insert("cwd".into(), cwd.into());
                }
            }
        }

        Ok(zed::DebugScenario {
            adapter: config.adapter,
            label: config.label,
            config: serde_json::Value::Object(configuration).to_string(),
            tcp_connection: None,
            build: None,
        })
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

/// Looks up `name` in the `lsp.cangjie-lsp.binary.env` settings.
fn lsp_settings_env_var(worktree: &Worktree, name: &str) -> Option<String> {
    zed::settings::LspSettings::for_worktree("cangjie-lsp", worktree)
        .ok()
        .and_then(|lsp_settings| lsp_settings.binary)
        .and_then(|binary_settings| binary_settings.env)
        .and_then(|env| env.get(name).map(|t| t.to_string()))
}

fn get_cangjie_home(worktree: &Worktree) -> zed::Result<String> {
    lsp_settings_env_var(worktree, "CANGJIE_HOME")
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

/// Library search path needed by the debuggee to load the Cangjie runtime,
/// prepended to any value already present in the worktree's shell environment.
fn library_path_env(cangjie_home: &str, worktree: &Worktree) -> zed::Result<(String, String)> {
    let (os, architecture) = zed::current_platform();
    let (variable, os_name) = match os {
        zed::Os::Mac => ("DYLD_LIBRARY_PATH", "darwin"),
        zed::Os::Linux => ("LD_LIBRARY_PATH", "linux"),
        zed::Os::Windows => return Err("debugging Cangjie is not supported on Windows".into()),
    };
    let architecture_name = match architecture {
        zed::Architecture::Aarch64 => "aarch64",
        zed::Architecture::X8664 => "x86_64",
        zed::Architecture::X86 => {
            return Err("debugging Cangjie is not supported on 32-bit x86".into())
        }
    };

    let mut library_path = format!(
        "{cangjie_home}/runtime/lib/{os_name}_{architecture_name}_cjnative:{cangjie_home}/tools/lib"
    );
    if let Some((_, existing)) = worktree
        .shell_env()
        .into_iter()
        .find(|(key, value)| key == variable && !value.is_empty())
    {
        library_path.push(':');
        library_path.push_str(&existing);
    }
    Ok((variable.to_string(), library_path))
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
