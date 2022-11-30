use std::path::{PathBuf, Path};

const PUB_KEY: [u8; 32] = [
    84, 173, 250, 169, 229, 119, 126, 227, 207, 177, 135, 154, 158, 77, 116, 125, 197, 204, 135,
    61, 110, 215, 203, 166, 6, 229, 111, 148, 213, 3, 85, 12,
];

fn signature_path<P: AsRef<Path>>(file: P) -> PathBuf {
    let mut file_extension = file.as_ref().file_name().unwrap_or_default().to_os_string();
    file_extension.push(".ed25519");
    let mut path = PathBuf::from(file.as_ref());
    path.set_file_name(&file_extension);
    path
}

pub fn verify_signature<P: AsRef<Path>>(file: P) -> bool {
    use ring_compat::signature::{
        ed25519::{Signature, VerifyingKey},
        Verifier,
    };

    let Ok(key) = VerifyingKey::new(&PUB_KEY) else {
        return false;
    };

    let Ok(file_raw) = std::fs::read(file.as_ref()) else {
        return false;
    };

    let Ok(file_sign_raw) = &std::fs::read(signature_path(file.as_ref())) else {
        return false;
    };

    let Ok(signature) = Signature::from_bytes(&file_sign_raw) else {
        return false;
    };

    key.verify(&file_raw, &signature).is_ok()
}
