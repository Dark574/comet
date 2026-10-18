use comet::constants;
use serde::Deserialize;
use std::io::{self, Read};

const MAX_STDIN_CREDENTIAL_BYTES: u64 = 16 * 1024;

mod heroic;
#[cfg(target_os = "linux")]
mod lutris;
#[cfg(target_os = "linux")]
mod wyvern;

pub struct ImportedCredentials {
    pub access_token: String,
    pub refresh_token: String,
    pub galaxy_user_id: String,
    pub username: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StdinCredentials {
    access_token: String,
    refresh_token: String,
    user_id: String,
    username: String,
}

pub fn handle_credentials_import(args: &crate::Args) -> Result<ImportedCredentials, &'static str> {
    if args.credentials_stdin {
        return read_credentials_from(io::stdin().lock());
    }

    if args.heroic {
        log::debug!("Loading Heroic credentials");
        let config = heroic::load_tokens();
        let config = config
            .fields
            .get(constants::GALAXY_CLIENT_ID)
            .expect("No Galaxy credentials");

        let access_token = config
            .get("access_token")
            .expect("access_token not present in heroic config")
            .as_str()
            .unwrap()
            .to_owned();
        let refresh_token = config
            .get("refresh_token")
            .expect("refresh_token not present in heroic config")
            .as_str()
            .unwrap()
            .to_owned();
        let galaxy_user_id = config
            .get("user_id")
            .expect("user_id not present in heroic config")
            .as_str()
            .unwrap()
            .to_owned();
        return Ok(ImportedCredentials {
            access_token,
            refresh_token,
            galaxy_user_id,
            username: None,
        });
    }

    #[cfg(target_os = "linux")]
    if args.lutris {
        let config = lutris::load_tokens();
        let access_token = config
            .get("access_token")
            .expect("access_token not present in lutris config")
            .as_str()
            .unwrap()
            .to_owned();
        let refresh_token = config
            .get("refresh_token")
            .expect("refresh_token not present in lutris config")
            .as_str()
            .unwrap()
            .to_owned();
        let galaxy_user_id = config
            .get("user_id")
            .expect("user_id not present in lutris config")
            .as_str()
            .unwrap()
            .to_owned();
        return Ok(ImportedCredentials {
            access_token,
            refresh_token,
            galaxy_user_id,
            username: None,
        });
    }

    #[cfg(target_os = "linux")]
    if args.wyvern {
        let config = wyvern::load_tokens();
        let token = config.token;
        let (access_token, refresh_token, galaxy_user_id) = token.dissolve();

        return Ok(ImportedCredentials {
            access_token,
            refresh_token,
            galaxy_user_id,
            username: None,
        });
    }

    let access_token = args.access_token.clone().expect("Access token is required");
    let refresh_token = args
        .refresh_token
        .clone()
        .expect("Refresh token is required");
    let galaxy_user_id = args.user_id.clone().expect("User id is required");

    Ok(ImportedCredentials {
        access_token,
        refresh_token,
        galaxy_user_id,
        username: None,
    })
}

fn read_credentials_from<R: Read>(reader: R) -> Result<ImportedCredentials, &'static str> {
    let mut data = Vec::new();
    reader
        .take(MAX_STDIN_CREDENTIAL_BYTES + 1)
        .read_to_end(&mut data)
        .map_err(|_| "Could not read credentials from stdin")?;

    if data.len() as u64 > MAX_STDIN_CREDENTIAL_BYTES {
        data.fill(0);
        return Err("Credential input from stdin is too large");
    }

    let parsed = serde_json::from_slice::<StdinCredentials>(&data)
        .map_err(|_| "Credential input from stdin is not valid JSON");
    data.fill(0);
    let parsed = parsed?;

    create_credentials(
        parsed.access_token,
        parsed.refresh_token,
        parsed.user_id,
        Some(parsed.username),
    )
}

fn create_credentials(
    access_token: String,
    refresh_token: String,
    galaxy_user_id: String,
    username: Option<String>,
) -> Result<ImportedCredentials, &'static str> {
    if access_token.is_empty() {
        return Err("Access token must not be empty");
    }
    if refresh_token.is_empty() {
        return Err("Refresh token must not be empty");
    }
    if galaxy_user_id.trim().is_empty() {
        return Err("User id must not be empty");
    }
    if username
        .as_ref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err("User name must not be empty");
    }

    Ok(ImportedCredentials {
        access_token,
        refresh_token,
        galaxy_user_id,
        username,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn reads_credentials_from_stdin_json() {
        let input = br#"{
            "access_token": "access",
            "refresh_token": "refresh",
            "user_id": "12345",
            "username": "player"
        }"#;

        let credentials = read_credentials_from(Cursor::new(input)).unwrap();

        assert_eq!(credentials.access_token, "access");
        assert_eq!(credentials.refresh_token, "refresh");
        assert_eq!(credentials.galaxy_user_id, "12345");
        assert_eq!(credentials.username.as_deref(), Some("player"));
    }

    #[test]
    fn rejects_missing_stdin_credentials() {
        let input = br#"{
            "access_token": "access",
            "user_id": "12345",
            "username": "player"
        }"#;

        let error = read_credentials_from(Cursor::new(input)).err().unwrap();

        assert_eq!(error, "Credential input from stdin is not valid JSON");
    }

    #[test]
    fn rejects_empty_stdin_credentials() {
        let input = br#"{
            "access_token": "",
            "refresh_token": "refresh",
            "user_id": "12345",
            "username": "player"
        }"#;

        let error = read_credentials_from(Cursor::new(input)).err().unwrap();

        assert_eq!(error, "Access token must not be empty");
    }

    #[test]
    fn rejects_oversized_stdin_credentials() {
        let input = vec![b'x'; MAX_STDIN_CREDENTIAL_BYTES as usize + 1];

        let error = read_credentials_from(Cursor::new(input)).err().unwrap();

        assert_eq!(error, "Credential input from stdin is too large");
    }
}
