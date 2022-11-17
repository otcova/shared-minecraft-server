use std::fs::{copy, create_dir_all, remove_dir_all, write};

fn main() {
    if profile_is_release() {
        create_dir_all("releases/last").expect("Could not create releases/last folder");
        remove_dir_all("releases/last").expect("Could not remove previous release");
        create_dir_all("releases/last").expect("Could not create releases/last folder");

        write("releases/last/version.txt", env!("CARGO_PKG_VERSION"))
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
