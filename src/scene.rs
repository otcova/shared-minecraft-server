#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum Scene {
    Main,
    SomeoneLocked,
    SelfLocked,
    Hosting {
        server_output: String,
        command: String,
    },

    Downloading,
    Uploading,
}
