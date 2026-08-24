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
