use std::fs::{copy, create_dir_all, remove_dir_all, write};

fn main() {
    if profile_is_release() {
        create_dir_all("releases/last").expect("Could not create releases/last folder");
        remove_dir_all("releases/last").expect("Could not remove previous release");
        create_dir_all("releases/last").expect("Could not create releases/last folder");


        write("releases/last/version.txt", fetch_current_version())
            .expect("Could not write current version");

        copy(
            "target/release/shared-minecraft-server.exe",
            "releases/last/Shared Minecraft Server.exe",
        )
        .expect("Could not copy app executable");
    }
}

fn profile_is_release() -> bool {
    env!("CRATE_PROFILE") == "release"
}

fn fetch_current_version() -> String {
    let raw_cargo_toml = std::fs::read_to_string(env!("CRATE_MANIFEST_PATH")).unwrap();
    let cargo_toml = raw_cargo_toml.parse::<toml::Value>().unwrap();
    cargo_toml["package"].as_table().unwrap()["version"].as_str().unwrap().into()
}
