use std::{io, fs};

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Credentials {
    user: String,
    pass: String,
    #[serde(default)]
    serial: Vec<String>,
}

type CredentialsFile = Vec<Credentials>;

/// Load a credential file and return the first matching credentials.
pub fn load_credentials(path: &str, serial: Option<String>) -> io::Result<Option<(String, String)>>{
    let cred_json = fs::read_to_string(path)?;
    let cred_file: CredentialsFile = serde_json::from_str(&cred_json)?;

    for cred in cred_file {
        if serial.is_some() && cred.serial.len() > 0 {
            if cred.serial.contains(serial.as_ref().unwrap()) {
                return Ok(Some((cred.user, cred.pass)));
            }
        } else {
            return Ok(Some((cred.user, cred.pass)));
        }
    }

    return Ok(None);
}
