fn main() {
    #[cfg(windows)]
    {
        use embed_manifest::manifest::{ExecutionLevel, SupportedOS};
        use embed_manifest::{embed_manifest, new_manifest};

        // requireAdministrator triggers a UAC prompt whenever the helper exe
        // is launched directly (e.g. via Start-Process -Verb RunAs). The app
        // shell relies on this for elevation rather than running its own
        // shellexecute("runas") logic.
        let manifest = new_manifest("CrossPoint.UnlockerHelper")
            .requested_execution_level(ExecutionLevel::RequireAdministrator)
            .supported_os(SupportedOS::Windows10..);
        embed_manifest(manifest).expect("embed-manifest");
    }
}
