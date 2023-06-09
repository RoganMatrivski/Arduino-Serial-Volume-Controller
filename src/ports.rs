use color_eyre::{eyre::bail, Report};
use serialport::SerialPort;
use tracing::{debug, trace};

const DEVICE_SIGNATURE: [u8; 8] = [0x03, 0x55, 0x72, 0xf5, 0x1e, 0xd8, 0x06, 0x33];

use crate::DATA_LENGTH;

#[allow(dead_code)]
pub fn check_if_available(port_name: &str) -> Result<bool, Report> {
    let ports = serialport::available_ports()?;

    let is_available = ports.iter().any(|x| x.port_name.as_str() == port_name);

    Ok(is_available)
}

#[tracing::instrument]
pub fn find_device() -> Result<Box<dyn SerialPort>, Report> {
    debug!("Getting all serial devices");
    let mut ports = serialport::available_ports()?;
    ports.sort_by(|a, b| b.port_name.cmp(&a.port_name));

    for port in ports {
        debug!(port_name = port.port_name, "Testing port");
        let mut serial: Box<dyn SerialPort> = serialport::new(port.clone().port_name, 115_200)
            .timeout(std::time::Duration::from_millis(1000))
            .open()?;

        trace!(port_name = port.port_name, "Writing to port");
        let written = serial.write(&DEVICE_SIGNATURE)?;

        if written < DEVICE_SIGNATURE.len() {
            debug!(port_name = port.port_name, "Can't write to port");
            continue;
        }

        let mut readbuf: Vec<u8> = vec![0; DATA_LENGTH];
        let read = serial.read(&mut readbuf)?;

        if read < 8 {
            debug!(
                port_name = port.port_name,
                read_size = read,
                "Mismatch read amount"
            );
            continue;
        }

        let recv_signature = &readbuf[1..8];

        if !DEVICE_SIGNATURE[1..].eq(recv_signature) {
            debug!(
                port_name = port.port_name,
                "Invalid signature. Received: {recv_signature:?}. Expected {DEVICE_SIGNATURE:?}"
            );
            continue;
        }

        debug!(
            "Found Arduino Volume Controller at port {}!",
            port.port_name
        );

        return Ok(serial);
    }

    bail!("Failed to find volume control device!");
}
