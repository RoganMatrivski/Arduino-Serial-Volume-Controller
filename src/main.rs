use color_eyre::{eyre::bail, Report};
use tracing::{debug, trace};

mod init;
mod ports;
mod volctrl;
// mod windows;

const DATA_LENGTH: usize = 16;

macro_rules! u8_bool {
    ($byte:expr, $true:expr, $false:expr) => {
        if $byte == $true {
            true
        } else if $byte == $false {
            false
        } else {
            false
        }
    };
}

fn write_serial(serial: &mut Box<dyn serialport::SerialPort>, data: &[u8]) -> Result<(), Report> {
    let write_amt = serial.write(data)?;
    if write_amt != data.len() {
        bail!("Unexpected write amount!")
    }

    Ok(())
}

fn read_serial(
    serial: &mut Box<dyn serialport::SerialPort>,
    length: usize,
) -> Result<Vec<u8>, Report> {
    let mut readbuf: Vec<u8> = vec![0; length];
    let read_amt = serial.read(&mut readbuf)?;
    if read_amt != readbuf.len() {
        bail!("Unexpected write amount!")
    }

    Ok(readbuf)
}

#[cfg(target_os = "windows")]
#[tokio::main]
async fn main() -> Result<(), Report> {
    let args = init::initialize()?;

    use std::{thread::sleep, time::Duration};

    const SLEEP_DURATION_MS: u64 = 100;
    const SEND_RECV_RATIO: f32 = 2.0;

    let serial = if !args.sequence_scan {
        ports::find_device_async().await?
    } else {
        ports::find_device_sync()?
    };

    let serial_id = {
        let mut serial = serial.try_clone()?;
        let volctrl = volctrl::VolCtrl::new()?;
        let volume_state = volctrl.get_volume_level()? + 0b1000_0000;
        let mute_state: u8 = if volctrl.get_volume_mute()? { 255 } else { 128 };
        write_serial(
            &mut serial,
            &[0x01, volume_state, mute_state, 0, 0, 0, 0, 0, 0],
        )?;
        read_serial(&mut serial, DATA_LENGTH)?
    };

    use tokio::{task, task::JoinHandle};
    let serial_id_clone = serial_id.clone();
    let mut serial_clone = serial.try_clone()?;
    let sender_join: JoinHandle<Result<(), Report>> = task::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        loop {
            trace!("Sending new volume request: {serial_id_clone:?}");
            write_serial(&mut serial_clone, &serial_id_clone[..8])?;

            tokio::time::sleep(Duration::from_millis(SLEEP_DURATION_MS)).await;
        }
    });

    let receiver_join: JoinHandle<Result<(), Report>> = task::spawn(async move {
        trace!("Initializing Volume Controller");
        let volctrl = match volctrl::VolCtrl::new() {
            Ok(ctrl) => ctrl,
            Err(err) => bail!("Failed to initialize controller! {err:?}"),
        };
        trace!("Volume Controller Initialized");

        let mut prev_volume = volctrl.get_volume_level()?;
        let mut prev_mute = volctrl.get_volume_mute()?;

        loop {
            trace!("Cloning serial communicator");
            let mut serial = serial.try_clone()?;

            trace!("Getting bytes to be read");
            let to_read = serial.bytes_to_read()?;
            trace!("New bytes = {to_read}");

            if to_read < DATA_LENGTH as u32 {
                sleep(Duration::from_millis(
                    (SLEEP_DURATION_MS as f32 / SEND_RECV_RATIO) as u64,
                ));
                continue;
            }

            trace!("Reading new volume");
            let read_res = read_serial(&mut serial, DATA_LENGTH)?;
            trace!("First 4 bytes {:?}", &read_res[..4]);
            let non_full_byte: Vec<&u8> = read_res.iter().filter(|&&x| x != 255).collect();
            trace!("Non 255 bytes {non_full_byte:?}");

            trace!("Parsing {:?}", &read_res[..2]);
            let (volume_level, is_mute) =
                (read_res[1] & 0b0111_1111, u8_bool!(read_res[0], 255, 128));

            if prev_volume != volume_level {
                debug!(vol = volume_level, "Setting new volume");
                volctrl.set_volume_level(volume_level)?;

                prev_volume = volume_level;
            }

            if prev_mute != is_mute {
                debug!(mute = is_mute, "Setting new mute");
                volctrl.set_volume_mute(is_mute)?;

                prev_mute = is_mute;
            }

            sleep(Duration::from_millis(
                (SLEEP_DURATION_MS as f32 / SEND_RECV_RATIO) as u64,
            ));

            // Really wanna use this. But this task will probably be sent to other thread
            // And VolCtrl can't be sent safely across.
            // tokio::time::sleep(SLEEP_DURATION).await;
        }
    });

    // There was an attempt.
    // tokio::select! {
    //     _ = tokio::signal::ctrl_c() => {},
    //     _ = sender_join => {},
    //     _ = receiver_join => {},
    // }

    let (sender_res, recv_res) = tokio::join!(sender_join, receiver_join);
    let _ = sender_res?;
    let _ = recv_res?;

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn main() {
    panic!("This software isn't supported on OS other than Windows for now");
}
