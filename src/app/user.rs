use crate::public_ip;

const SEPARATOR: &str = "_;_";

pub fn id_from(username: &str, email: &str) -> String {
    let username = username.replace(SEPARATOR, ":O");
    let email = email.replace(SEPARATOR, ":O");
    public_ip::get().expect("Could not get public ip address")
        + SEPARATOR
        + &username
        + SEPARATOR
        + &email
}

pub struct ParsedUserId<'a> {
    pub ip: &'a str,
    pub username: &'a str,
    pub email: &'a str,
}

pub fn parse_id(user_id: &String) -> Option<ParsedUserId> {
    let mut pieces = user_id.split(SEPARATOR);
    Some(ParsedUserId {
        ip: pieces.next()?,
        username: pieces.next()?,
        email: pieces.next()?,
    })
}
