use crate::public_ip;

pub fn id_from_username(username: &String) -> String {
    public_ip::get().expect("Could not get public ip address") + ";" + username
}

pub fn username_from_id(user_id: &String) -> String {
    user_id
        .split_once(";")
        .map(|(_, name)| String::from(name))
        .unwrap_or_else(|| user_id.clone())
}

pub fn user_ip_from_id(user_id: &String) -> String {
    user_id
        .split_once(";")
        .map(|(ip, _)| String::from(ip))
        .unwrap_or_else(|| user_id.clone())
}