use std::fs::read_to_string;

fn main() {
    let env_file = read_to_string("not_very_secret.env");
    let env_file = env_file.expect("You must have 'not_very_secret.env' file");

    for line in env_file.lines() {
        if let Some((key, value)) = line.split_once("=") {
            println!("cargo:rustc-env={key}={value}");
        }
    }
}
