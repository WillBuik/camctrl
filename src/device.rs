use std::time::Duration;

use onvif::{schema::{self, transport, onvif::User}, soap};
use url::Url;
use log::*;

#[derive(Debug)]
pub enum DeviceError {
    /// Device acted in an unexpected way.
    UnexpectedBehavior (String),

    /// ONVIF transport error.
    ///
    /// Serialization or network errors.
    Transport (transport::Error),

    /// Credentials were rejected for an operation.
    Unauthorized(String),

    /// Unknown device error.
    Unknown (Option<String>),
}

impl From<transport::Error> for DeviceError {
    fn from(err: transport::Error) -> Self {
        match err {
            transport::Error::Authorization(err) => DeviceError::Unauthorized(err),
            _ => DeviceError::Transport(err),
        }
    }
}

impl From<String> for DeviceError {
    fn from(err: String) -> Self {
        DeviceError::Unknown(Some(err))
    }
}

impl Default for DeviceError {
    fn default() -> Self {
        DeviceError::Unknown(None)
    }
}

impl std::fmt::Display for DeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceError::UnexpectedBehavior(err) =>
                f.write_str(&format!("Unexpected behavior: {}", err)),
            DeviceError::Transport(err) =>
                f.write_str(&format!("Transport error: {}", err)),
            DeviceError::Unauthorized(err) =>
                f.write_str(&format!("Unauthorized: {}", err)),
            DeviceError::Unknown(err) =>
                match err {
                    Some(err) => f.write_str(&format!("Unknown error: {}", err)),
                    None => f.write_str(&format!("Unknown error")),
                },
        }
    }
}

impl std::error::Error for DeviceError {}

pub struct Device {
    devicemgmt: soap::client::Client,
    event: Option<soap::client::Client>,
    deviceio: Option<soap::client::Client>,
    media: Option<soap::client::Client>,
    media2: Option<soap::client::Client>,
    imaging: Option<soap::client::Client>,
    ptz: Option<soap::client::Client>,
    analytics: Option<soap::client::Client>,
}

impl Device {
    const SERVICE_TIMEOUT: Duration = Duration::from_secs(10);
    
    pub async fn new(devicemgmt_uri: Url, user: Option<String>, pass: Option<String>) -> Result<Self, DeviceError> {
        // Adapted from https://github.com/lumeohq/onvif-rs/blob/main/onvif/examples/camera.rs
        // Copyright (c) 2019 Lumeo, Inc.
        let creds = match (user.as_ref(), pass.as_ref()) {
            (Some(username), Some(password)) => Some(soap::client::Credentials {
                username: username.clone(),
                password: password.clone(),
            }),
            (None, None) => None,
            _ => {
                return Err(DeviceError::Unauthorized("Username and password must be specified together".into()));
            },
        };

        let mut out = Self {
            devicemgmt: soap::client::ClientBuilder::new(&devicemgmt_uri)
                .credentials(creds.clone())
                .timeout(Self::SERVICE_TIMEOUT)
                .build(),
            imaging: None,
            ptz: None,
            event: None,
            deviceio: None,
            media: None,
            media2: None,
            analytics: None,
        };

        let mut base_uri = devicemgmt_uri.clone();
        base_uri.set_path("/");

        let services =
            schema::devicemgmt::get_services(&out.devicemgmt, &Default::default()).await?;
        for s in &services.service {
            let url = Url::parse(&s.x_addr).map_err(|e| e.to_string())?;
            if !url.as_str().starts_with(base_uri.as_str()) {
                return Err(DeviceError::UnexpectedBehavior(format!(
                    "Service URI {} is not within base URI {}",
                    &s.x_addr, &base_uri
                )));
            }
            let svc = Some(
                soap::client::ClientBuilder::new(&url)
                    .credentials(creds.clone())
                    .timeout(Self::SERVICE_TIMEOUT)
                    .build(),
            );
            match s.namespace.as_str() {
                "http://www.onvif.org/ver10/device/wsdl" => {
                    if s.x_addr != devicemgmt_uri.as_str() {
                        return Err(DeviceError::UnexpectedBehavior(format!(
                            "advertised device mgmt uri {} not expected {}",
                            &s.x_addr, &devicemgmt_uri
                        )));
                    }
                }
                "http://www.onvif.org/ver10/events/wsdl" => out.event = svc,
                "http://www.onvif.org/ver10/deviceIO/wsdl" => out.deviceio = svc,
                "http://www.onvif.org/ver10/media/wsdl" => out.media = svc,
                "http://www.onvif.org/ver20/media/wsdl" => out.media2 = svc,
                "http://www.onvif.org/ver20/imaging/wsdl" => out.imaging = svc,
                "http://www.onvif.org/ver20/ptz/wsdl" => out.ptz = svc,
                "http://www.onvif.org/ver20/analytics/wsdl" => out.analytics = svc,
                _ => debug!("unknown service: {:?}", s),
            }
        }
        Ok(out)
    }

    pub fn get_device_service(&self) -> soap::client::Client {
        return self.devicemgmt.clone();
    }

    pub fn get_media_service(&self) -> Option<soap::client::Client> {
        return self.media.clone();
    }

    pub async fn get_users(&self) -> Result<Vec<User>, DeviceError> {
        let user_response = schema::devicemgmt::get_users(&self.devicemgmt, &Default::default()).await?;
        return Ok(user_response.user);
    }

    /*pub async fn create_user(&self, user: User) -> Result<(), DeviceError> {
        let mut create_user_request: schema::devicemgmt::CreateUsers = Default::default();
        create_user_request.user.push(user);
        schema::devicemgmt::create_users(&self.devicemgmt, &create_user_request).await?;
        return Ok(());
    }*/

    pub async fn set_user(&self, user: User) -> Result<(), DeviceError> {
        let mut set_user_request: schema::devicemgmt::SetUser = Default::default();
        set_user_request.user.push(user);
        schema::devicemgmt::set_user(&self.devicemgmt, &set_user_request).await?;
        return Ok(());
    }

    pub async fn system_reboot(&self) -> Result<String, DeviceError> {
        let x = schema::devicemgmt::system_reboot(&self.devicemgmt, &Default::default()).await?;
        return Ok(x.message);
    }
    
}
