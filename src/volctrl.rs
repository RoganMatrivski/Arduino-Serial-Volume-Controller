use color_eyre::Report;
use std::ptr;
use tracing::trace;
use windows::Win32::{
    Media::Audio::{Endpoints::IAudioEndpointVolume, *},
    System::Com::*,
};

#[derive(Clone)]
pub struct VolCtrl {
    pub endpoint: IAudioEndpointVolume,
    guid: *const windows::core::GUID,
}

impl VolCtrl {
    pub fn new() -> Result<Self, Report> {
        unsafe {
            CoInitialize(None)?;

            trace!("Getting device enumerator");
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            trace!("Getting default audio endpoint");
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            trace!("Getting volume controller");
            let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, Some(ptr::null()))?;

            trace!("Generating new GUID");
            let guid = windows::core::GUID::new()?;
            let guid = &guid as *const windows::core::GUID;

            CoUninitialize();

            Ok(VolCtrl { endpoint, guid })
        }
    }

    #[allow(dead_code)]
    pub fn refresh_endpoint(&mut self) -> Result<(), Report> {
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, Some(ptr::null()))?;

            self.endpoint = endpoint
        }

        Ok(())
    }

    pub fn get_volume_level(&self) -> Result<u8, Report> {
        unsafe { Ok((self.endpoint.GetMasterVolumeLevelScalar()? * 100.0) as u8) }
    }

    pub fn set_volume_level(&self, new_level: u8) -> Result<(), Report> {
        unsafe {
            Ok(self
                .endpoint
                .SetMasterVolumeLevelScalar(f32::from(new_level) / 100.0, self.guid)?)
        }
    }

    pub fn get_volume_mute(&self) -> Result<bool, Report> {
        unsafe { Ok((self.endpoint.GetMute()?).into()) }
    }

    pub fn set_volume_mute(&self, mute: bool) -> Result<(), Report> {
        unsafe { Ok(self.endpoint.SetMute(mute, self.guid)?) }
    }
}
