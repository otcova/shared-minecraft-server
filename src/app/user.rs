use crate::public_ip;

pub fn id() -> String {
    format!(
        "ip={}",
        public_ip::get().expect("Could not get public ip address")
    )
}

pub fn parse_id<'a, S: AsRef<str>, T: AsRef<str>>(user_id: &'a S, key: T) -> Option<&'a str> {
    for line in user_id.as_ref().split("\n") {
        if let Some((line_key, value)) = line.split_once("=") {
            if line_key == key.as_ref() {
                return Some(value);
            }
        }
    }
    None
}

