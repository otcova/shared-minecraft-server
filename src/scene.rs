
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u8)]
pub enum Scene {
    Main,
    SomeoneLocked,
    SelfLocked,
    Hosting,

    Downloading,
    Uploading,
}
