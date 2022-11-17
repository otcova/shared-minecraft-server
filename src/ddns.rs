use crate::{error::Error, public_ip};
use http_req::{request::Request, uri::Uri};

/// Updates the ddns to point to the public ip of the current machine
pub fn update() -> Result<(), Error> {
    let Some(ip) = public_ip::get() else {
		return Err(Error::from_str("Could not get public ip"))
	};

    let mut writer = Vec::new();
    let uri = Uri::try_from("https://dynupdate.no-ip.com/nic/update?hostname=vsinf.ddns.net")?;
    let response = Request::new(&uri)
        .header("Host", "dynupdate.no-ip.com")
        .header("Authorization", "Basic b3RnZXJjb21hczpzN2FuSGFiUThYZXVp")
        .header(
            "User-Agent",
            "Octova shared_mc_server/0.1.0 otgercomas@gmail.com",
        )
        .send(&mut writer)?;

    let body = String::from_utf8_lossy(&writer);

    if body == format!("good {}\r\n", ip) || body == format!("nochg {}\r\n", ip) {
        Ok(())
    } else {
        Err(Error::from_str(format!(
            "Invalid ddns response: {:?}",
            body
        )))
    }
}
