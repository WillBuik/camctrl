use std::str::FromStr;

use chrono::{NaiveDateTime, NaiveDate, Utc};
use log::*;
use clap::{Parser, Subcommand};

use device::{Device, DeviceError};
use onvif::schema::onvif::User;
use onvif::schema;
use url::Url;

mod discovery;
mod device;
mod util;

/// ONVIF camera control program.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Camera URI.
    #[clap(long)]
    uri: Option<String>,

    /// Credentials file.
    #[clap(long)]
    creds: Option<String>,

    /// Subcommand.
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Query network for all online ONVIF compatible devices.
    Probe,

    /// Show configuration for an ONVIF camera.
    Info,

    /// Get a list of users from a camera.
    GetUsers,
    /// Set a user's password.
    SetUser {
        username: String,
        password: String,
    },

    /// Reboot a camera.
    Reboot,
}

/// Convert an ONVIF DateTime to chrono::NaiveDateTime.
fn datetime_to_naive(datetime: &schema::onvif::DateTime) -> NaiveDateTime {
    NaiveDate::from_ymd(datetime.date.year, datetime.date.month.try_into().unwrap_or(0), datetime.date.day.try_into().unwrap_or(0))
    .and_hms(datetime.time.hour.try_into().unwrap_or(0), datetime.time.minute.try_into().unwrap_or(0), datetime.time.second.try_into().unwrap_or(0))
}

/// Convert an ONVIF Network Host to a display string.
fn network_host_to_string(network_host: &schema::onvif::NetworkHost) -> String {
    let mut display = String::new();
    if let Some(dns) = &network_host.dn_sname {
        display.push_str(&format!("{} ", dns));
    }
    if let Some(ipv4) = &network_host.i_pv_4_address {
        display.push_str(&format!("{} ", ipv4));
    }
    if let Some(ipv6) = &network_host.i_pv_6_address {
        display.push_str(&format!("{} ", ipv6));
    }
    return display;
}

/*async fn show_device_capabilities(device: &Device) -> Result<(), DeviceError> {
    let devicemgmt = device.get_device_service();
    let caps = schema::devicemgmt::get_capabilities(&devicemgmt, &Default::default()).await?.capabilities;

    println!("Device Capabilities:");
    println!("{:#?}", caps);
    return Ok(());
}*/

async fn show_device_info(device: &Device) -> Result<i32, DeviceError> {
    let devicemgmt = device.get_device_service();

    // Device Info
    let info = schema::devicemgmt::get_device_information(&devicemgmt, &Default::default()).await?;
    println!(
        "Device Info:\n  Serial\t{}\n  Make\t\t{}\n  Model\t\t{}\n  Firmware\t{}\n  Hardware ID\t{}",
        info.serial_number, info.manufacturer, info.model, info.firmware_version, info.hardware_id);

    // Device Time
    let time = schema::devicemgmt::get_system_date_and_time(&devicemgmt, &Default::default()).await?.system_date_and_time;
    let ntp = schema::devicemgmt::get_ntp(&devicemgmt, &Default::default()).await?.ntp_information;
    println!("Device Time:");
    println!("  Source\t{:?}", time.date_time_type);
    println!("  DST\t\t{}", time.daylight_savings);
    if let Some(tz) = time.time_zone {
        println!("  TimeZone\t{}", tz.tz);
    } else {
        println!("  TimeZone\tNot Set");
    }
    if let Some(utc) = &time.utc_date_time {
        print!("  UTC\t\t{}", datetime_to_naive(utc));
        if datetime_to_naive(utc).signed_duration_since(Utc::now().naive_utc()).num_seconds().abs() > 15 {
            println!(" *DOES NOT MATCH SYSTEM*");
        } else {
            println!("");
        }
    } else {
        println!("  UTC\t\tNot Set");
    }
    if let Some(local) = &time.local_date_time {
        println!("  Local\t\t{}", datetime_to_naive(local));
    } else {
        println!("  Local\t\tNot Set");
    }

    if ntp.from_dhcp {
        print!("  DHCP NTP\t");
        ntp.ntp_from_dhcp.into_iter().for_each(|ntp| print!("{}", network_host_to_string(&ntp)));
        println!("");
    }
    print!("  NTP\t\t");
    ntp.ntp_manual.into_iter().for_each(|ntp| print!("{}", network_host_to_string(&ntp)));
    println!("");
    
    // Device Capabilities
    //show_device_capabilities(&device).await?;

    // Network Configuration
    let network = schema::devicemgmt::get_network_interfaces(&devicemgmt, &Default::default()).await?.network_interfaces;
    for iface in &network {
        let mut iface_name = iface.token.0.clone();
        if let Some(iface_info) = &iface.info {
            if let Some(name) = iface_info.name.clone() {
                iface_name = name;
            }
        }
        println!("Interface {} enabled={}", iface_name, iface.enabled);

        if let Some(iface_info) = &iface.info {
            println!("  HW Addr\t{}", iface_info.hw_address);
            if let Some(mtu) = iface_info.mtu {
                println!("  MTU\t\t{}", mtu);
            }
        }

        for ipv4_iface in &iface.i_pv_4 {
            if ipv4_iface.config.dhcp {
                for dhcp_ipv4 in &ipv4_iface.config.from_dhcp {
                    println!("  DHCP IP\t{}", dhcp_ipv4.address)
                }
            }
            for manual_ipv4 in &ipv4_iface.config.manual {
                println!("  IP\t\t{}", manual_ipv4.address)
            }
        }

        if let Some(extensions) = &iface.extension {
            for dot11 in &extensions.dot_11 {
                println!("  SSID\t\t{}", dot11.ssid)
            }
        }
    }

    // RTSP Stream URLs
    if let Some(media) = device.get_media_service() {
        let _profiles = schema::media::get_profiles(&media, &Default::default()).await?.profiles;
        //println!("{:#?}", profiles);
    }

    // User Configuration
    let users = device.get_users().await?;
    println!("Users:");
    for user in users {
        println!("  User\t\t{} ({:?})", user.username, user.user_level);
    }

    return Ok(0);
}

async fn run_command(cli: &Cli) -> Result<i32, DeviceError> {
    let uri = match &cli.uri {
        Some(uri) => {
            Url::from_str(&uri).map_err(|_| String::from("Could not parse URI"))?
        },
        None => return Err(DeviceError::from(String::from("No URI specified"))),
    };

    let (user, pass) = match &cli.creds {
        Some(cred_file_path) => {
            let creds = util::load_credentials(cred_file_path, None)
            .map_err(|err| format!("Could not load credential file: {}", err))?;

            creds.map(|(user, pass)| (Some(user), Some(pass))).unwrap_or((None, None))
        },
        None => (None, None),
    };

    let device = Device::new(uri, user, pass).await?;

    match &cli.command {
        Commands::Probe => unreachable!(),

        Commands::GetUsers => {
            let users = device.get_users().await?;
            for user in users {
                println!("Users:");
                println!("    {}\t{:?}\t{:?}", user.username, user.user_level, user.extension)
            }
        },

        Commands::SetUser { username, password } => {
            let users = device.get_users().await?;
            let existing_user = users.into_iter().filter(|u| { &u.username == username }).next();
            match existing_user {
                Some(existing_user) => {
                    let update_user = User {
                        username: username.clone(), 
                        password: Some(password.clone()),
                        user_level: existing_user.user_level, 
                        extension: existing_user.extension,
                    };
                    device.set_user(update_user).await?;
                },
                None => {
                    println!("User {} not found.", username);
                    return Ok(-1);
                },
            }
        },

        Commands::Reboot => {
            let message = device.system_reboot().await?;
            println!("Reboot: {}", message);
        },

        Commands::Info => return Ok(show_device_info(&device).await?),
    }

    return Ok(0);
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let cli = Cli::parse();

    match &cli.command {
        Commands::Probe => {
            println!("Discovering cameras");
            let result = discovery::discover().await;
            match result {
                Ok(_) => {},
                Err(err) => println!("Error: {}", err),
            }
        },

        _ => {
            match run_command(&cli).await {
                Ok(status) => std::process::exit(status),
                Err(err) => {
                    error!("Error: {}", err);
                    std::process::exit(-2);
                }
            }
        },
    }
}
