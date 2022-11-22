use std::fs::read_to_string;
#[cfg(windows)]
use winres::WindowsResource;

fn main() -> std::io::Result<()> {
    include_env_vars()?;
    attatch_icon()?;
    Ok(())
}

fn include_env_vars() -> std::io::Result<()> {
    let env_file = read_to_string("not_very_secret.env");
    let env_file = env_file.expect("You must have 'not_very_secret.env' file");

    for line in env_file.lines() {
        if let Some((key, value)) = line.split_once("=") {
            println!("cargo:rustc-env={key}={value}");
        }
    }

    Ok(())
}

#[cfg(windows)]
fn attatch_icon() -> std::io::Result<()> {
    WindowsResource::new().set_icon("icon.ico").compile()?;
    Ok(())
}
