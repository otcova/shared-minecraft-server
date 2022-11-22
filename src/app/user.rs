use crate::public_ip;

pub fn id<S: AsRef<str>>(username: Option<S>) -> String {
    let ip = public_ip::get().expect("Could not get public ip address");
    if let Some(username) = username {
        format!("ip={}\nusername={}", ip, username.as_ref())
    } else {
        format!("ip={}", ip)
    }
}

pub fn parse_id<'a, S: AsRef<str>, T: AsRef<str>>(user_id: &'a S, key: T) -> Option<&'a str> {
    for line in user_id.as_ref().lines() {
        if let Some((line_key, value)) = line.split_once("=") {
            if line_key == key.as_ref() {
                return Some(value);
            }
        }
    }
    None
}

pub fn display<S: AsRef<str>>(user_id: S) -> String {
    match parse_id(&user_id, "username") {
        Some(username) => username.into(),
        None => match parse_id(&user_id, "ip") {
            Some(ip) => ip.into(),
            None => user_id.as_ref().into(),
        },
    }
}
