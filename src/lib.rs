use zed_extension_api::{self as zed, Extension, LanguageServerId, Worktree};

struct CangjieExtension {}

impl Extension for CangjieExtension {
    fn new() -> Self {
        Self {}
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        _: &Worktree,
    ) -> zed::Result<zed::Command> {
        Err("Unimplemented".into())
    }
}

zed::register_extension!(CangjieExtension);
