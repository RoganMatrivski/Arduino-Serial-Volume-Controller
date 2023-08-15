use color_eyre::{eyre::bail, Report};
use serialport::SerialPort;
use tracing::{debug, trace};

use tokio::task::JoinSet;

const DEVICE_SIGNATURE: [u8; 8] = [0x03, 0x55, 0x72, 0xf5, 0x1e, 0xd8, 0x06, 0x33];

use crate::DATA_LENGTH;

#[allow(dead_code)]
pub fn check_if_available(port_name: &str) -> Result<bool, Report> {
    let ports = serialport::available_ports()?;

    let is_available = ports.iter().any(|x| x.port_name.as_str() == port_name);

    Ok(is_available)
}

#[tracing::instrument]
fn check_serial(port: &str) -> Result<Box<dyn SerialPort>, Report> {
    debug!(port_name = port, "Testing port");
    let mut serial: Box<dyn SerialPort> = serialport::new(port, 115_200)
        .timeout(std::time::Duration::from_millis(1000))
        .open()?;

    trace!(port_name = port, "Writing to port");
    let written = serial.write(&DEVICE_SIGNATURE)?;

    if written < DEVICE_SIGNATURE.len() {
        debug!(port_name = port, "Can't write to port");
        bail!("");
    }

    let mut readbuf: Vec<u8> = vec![0; DATA_LENGTH];
    let read = serial.read(&mut readbuf)?;
    if read < 8 {
        debug!(port_name = port, read_size = read, "Mismatch read amount");
        bail!("");
    }
    let recv_signature = &readbuf[1..8];
    if !DEVICE_SIGNATURE[1..].eq(recv_signature) {
        debug!(
            port_name = port,
            "Invalid signature. Received: {recv_signature:?}. Expected {DEVICE_SIGNATURE:?}"
        );
        bail!("");
    }

    debug!("Found Arduino Volume Controller at port {}!", port);

    Ok(serial)
}

fn get_ports_str() -> Result<Vec<String>, Report> {
    debug!("Getting all serial devices");
    let port_list = serialport::available_ports()?;

    Ok(port_list
        .iter()
        .map(|x| x.port_name.to_owned())
        .collect::<Vec<String>>())
}

#[tracing::instrument]
pub async fn find_device_async() -> Result<Box<dyn SerialPort>, Report> {
    let ports = get_ports_str()?;

    let mut set = JoinSet::new();

    for port in ports {
        set.spawn(async move { check_serial(&port) });
    }

    while let Some(res) = set.join_next().await {
        let scan_res = res?;

        if let Ok(serial) = scan_res {
            set.shutdown().await;
            return Ok(serial);
        };
    }

    bail!("Failed to find volume control device!");
}

#[tracing::instrument]
pub fn find_device_sync() -> Result<Box<dyn SerialPort>, Report> {
    let mut ports = get_ports_str()?;
    ports.sort_by(|a, b| b.cmp(a));

    for port in ports {
        if let Ok(serial) = check_serial(&port) {
            return Ok(serial);
        }
    }

    bail!("Failed to find volume control device!");
}
