use std::{
    io::{BufRead, Read, Write},
    sync::mpsc,
    time::Duration,
};

use bytes::Buf;
use event::{Event, Progress};
use model::Model;
use tracing::trace;

use crate::{
    instrument::{self, CmdLanguage, Language, Login, Reset},
    versatest::VERSATEST_FLASH_UTIL_STR,
    ConnectionAddr, InstrumentError,
};

use super::{
    protocol::{Protocol, ReadStb},
    Clear, Flash, Info, InstrumentInfo, Script,
};

pub mod event;
pub mod model;

pub(crate) mod authentication;

#[allow(dead_code)]
pub struct Instrument {
    protocol: Protocol,
    model: Model,
    tx_events: Vec<mpsc::Sender<Event>>,
    fw_flash_in_progress: bool,
}

impl Instrument {
    fn notify_subs(&self, event: &Event) {
        for t in self.tx_events.clone() {
            // This channel isn't that important. Just ignore errors.
            let _ = t.send(event.clone());
        }
    }

    pub fn subscribe_events(&mut self, tx: mpsc::Sender<Event>) {
        self.tx_events.push(tx);
    }

    fn write_all_raw(
        &mut self,
        mut buf: &[u8],
        progress_fn: Option<fn(Progress) -> Event>,
        complete_fn: Option<fn() -> Event>,
    ) -> std::io::Result<()> {
        let mut progress = Progress::new(buf.len());
        if let Some(pf) = progress_fn {
            self.notify_subs(&pf(progress));
        }
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    ));
                }
                Ok(n) => {
                    buf = &buf[n..];
                    progress.add_progress(n);
                    if let Some(pf) = progress_fn {
                        self.notify_subs(&pf(progress));
                    }
                }
                Err(e) => return Err(e),
            }
        }
        if let Some(cf) = complete_fn {
            self.notify_subs(&cf());
        }
        Ok(())
    }

    /// # Errors
    /// Issues with visa connections or raw sockets may return errors
    pub fn new(conn_info: ConnectionAddr) -> crate::error::Result<Self> {
        let mut protocol: super::protocol::Protocol = conn_info.try_into()?;

        let info = protocol.info()?;

        Ok(Self {
            protocol,
            model: info.model.unwrap_or(Model::Other(String::new())),
            tx_events: Vec::new(),
            fw_flash_in_progress: false,
        })
    }
}

impl Write for Instrument {
    #[tracing::instrument(skip(self, buf))]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        trace!("Writing to instrument: {}", String::from_utf8_lossy(buf));
        self.protocol.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.protocol.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.write_all_raw(
            buf,
            Some(Event::WriteProgress),
            Some(|| Event::WriteComplete),
        )
    }
}

impl Read for Instrument {
    /// Try to read from the instrument in a non-blocking way.
    ///
    /// # Errors
    /// If the operation would block (i.e. a message is not yet available from
    /// the instrument), returns `std::io::ErrorKind::WouldBlock`
    ///
    /// Other errors include [`visa_rs::Instrument`] read errors and [`TcpStream`]
    /// io errors.
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.protocol.read(buf)
        //match self.protocol {
        //    Protocol::RawSocket(_) => self.protocol.read(buf),
        //    Protocol::Visa { .. } => self.protocol.read(buf),
        //        match self.protocol.read_stb() {
        //        Ok(x) => {
        //            match x.mav() {
        //                Ok(false) => Err(std::io::Error::new(
        //                    std::io::ErrorKind::WouldBlock,
        //                    "read operation would block",
        //                )),

        //                // Err(_) means x must be `Stb::NotSupported`, therefore we can just read
        //                Ok(true)
        //                | Err(_) => self.protocol.read(buf)
        //            }
        //        }
        //        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        //    },
        //}
        //self.protocol.read(buf)
    }
}

impl ReadStb for Instrument {
    type Error = InstrumentError;

    fn read_stb(&mut self) -> core::result::Result<super::protocol::Stb, Self::Error> {
        self.protocol.read_stb()
    }
}

impl Clear for Instrument {
    type Error = InstrumentError;

    fn clear(&mut self) -> core::result::Result<(), Self::Error> {
        self.protocol.clear()
    }
}

impl Info for Instrument {
    type Error = InstrumentError;

    fn info(&mut self) -> core::result::Result<InstrumentInfo, Self::Error> {
        self.protocol.info()
    }
}
/// The [`Instrument`] can write a script to be executed.
impl Script for Instrument {
    type Error = InstrumentError;
    /// Write the given script to the instrument with the given name.
    ///
    /// # Parameters
    /// - `name` - is the environment-compatible name of the script
    /// - `script` - is the contents of the script
    /// - `save_script` - `true` if the script should be saved to non-volatile memory
    /// - `run_script` - `true` if the script should be run after load
    ///
    /// # Notes
    /// - The script name will not be validated to ensure that it is compatible with the
    ///     scripting environment.
    /// - The given script content will only be validated by the instrument, but not
    ///     the [`write_script`] function.
    ///
    /// # Errors
    /// Returns an [`InstrumentError`] if any errors occurred.
    fn write_script(
        &mut self,
        name: &[u8],
        script: &[u8],
        save_script: bool,
        run_script: bool,
    ) -> core::result::Result<(), Self::Error> {
        // Truncate name otherwise we risk a Fatal Error (NS-2201)
        let name = if self.model.is_tti() {
            String::from_utf8_lossy(bytes::Buf::take(name, 31).chunk()).to_string()
        } else {
            String::from_utf8_lossy(name).to_string()
        };
        //let mut script = script.reader();
        self.write_all_raw(
            b"_orig_prompts = localnode.prompts localnode.prompts = 0\n",
            None,
            None,
        )?;
        self.flush()?;
        self.write_all_raw(format!("{name}=nil\n").as_bytes(), None, None)?;
        self.flush()?;
        self.write_all_raw(format!("loadscript {name}\n").as_bytes(), None, None)?;

        let mut progress = Progress::new(script.len());
        for line in script.lines() {
            progress.add_progress(self.write(format!("{}\n", line?).as_bytes())?);
            self.notify_subs(&Event::ScriptProgress(progress));
        }
        self.write_all(b"\nendscript\n")?;
        self.flush()?;

        if save_script {
            self.write_all(format!("{name}.save()\n").as_bytes())?;
            self.flush()?;
        }

        if run_script {
            self.write_all(format!("{name}.run()\n").as_bytes())?;
            self.flush()?;
        }

        self.write_all(b"localnode.prompts = _orig_prompts _orig_prompts = nil\n")?;
        self.flush()?;

        self.notify_subs(&Event::ScriptComplete);

        Ok(())
    }
}

impl Flash for Instrument {
    type Error = InstrumentError;

    fn flash_firmware(&mut self, image: &[u8]) -> core::result::Result<(), Self::Error> {
        self.fw_flash_in_progress = true;
        match self.model {
            Model::_2601
            | Model::_2602
            | Model::_2611
            | Model::_2612
            | Model::_2635
            | Model::_2636
            | Model::_2601A
            | Model::_2602A
            | Model::_2611A
            | Model::_2612A
            | Model::_2635A
            | Model::_2636A
            | Model::_2651A
            | Model::_2657A
            | Model::_2601B
            | Model::_2601B_PULSE
            | Model::_2602B
            | Model::_2606B
            | Model::_2611B
            | Model::_2612B
            | Model::_2635B
            | Model::_2636B
            | Model::_2604B
            | Model::_2614B
            | Model::_2634B
            | Model::_2601B_L
            | Model::_2602B_L
            | Model::_2611B_L
            | Model::_2612B_L
            | Model::_2635B_L
            | Model::_2636B_L
            | Model::_2604B_L
            | Model::_2614B_L
            | Model::_2634B_L => {
                let mut image = image.reader();
                self.write_all(b"localnode.prompts = 0\n")?;
                self.write_all(b"flash\n")?;
                self.write_all(image.fill_buf().unwrap())?;
                self.write_all(b"endflash\n")?;
            }
            Model::_3706
            | Model::_3706_SNFP
            | Model::_3706_S
            | Model::_3706_NFP
            | Model::_3706A
            | Model::_3706A_SNFP
            | Model::_3706A_S
            | Model::_3706A_NFP => {
                self.write_all(b"localnode.prompts = 0\n")?;
                self.write_all(b"prevflash\n")?;

                for chunk in image.chunks(4096) {
                    self.write_all(chunk)?;
                    std::thread::sleep(Duration::from_millis(10)); //The position and duration of this delay is intentional
                }

                std::thread::sleep(Duration::from_millis(10)); //The position and duration of this delay is intentional
                self.write_all(b"endflash\n")?;
            }
            Model::_707B | Model::_708B => {}
            Model::_2450
            | Model::_2470
            | Model::_DMM7510
            | Model::_2460
            | Model::_2461
            | Model::_2461_SYS
            | Model::DMM7512
            | Model::DMM6500
            | Model::DAQ6510 => {
                let mut image = image.reader();

                self.write_all(b"localnode.prompts=localnode.DISABLE\n")?;
                self.write_all(b"if ki.upgrade ~= nil and ki.upgrade.noacklater ~= nil then ki.upgrade.noacklater() end\n")?;
                self.write_all(b"prevflash\n")?;

                self.write_all(image.fill_buf().unwrap())?;

                self.write_all(b"endflash\n")?;
            }
            Model::Mp5103(_) => {
                self.write_all(b"localnode.prompts=0\n")?;
                let mut image = image.reader();
                self.write_all(b"flash\n")?;

                let _ = self.write_all(image.fill_buf().unwrap());
                self.write_all(b"endflash\n")?;

                //Update Mainframe
                self.write_all(b"firmware.update()\n")?;
            }
            Model::Other(_) => todo!(),
        }

        Ok(())
    }

    fn flash_module(&mut self, module: u16, image: &[u8]) -> core::result::Result<(), Self::Error> {
        self.fw_flash_in_progress = true;
        if let Model::Mp5103(_) = self.model {
            //TODO This is temporary: Only use while not defined in FW
            self.write_script(b"FlashUtil", VERSATEST_FLASH_UTIL_STR, false, true)?;

            self.write_all_raw(b"localnode.prompts=0\n", None, None)?;
            //let mut image = image.reader();
            self.write_all_raw(b"flash\n", None, None)?;

            self.write_all_raw(image, Some(Event::FwProgress), Some(|| Event::FwComplete))?;
            self.write_all_raw(b"endflash\n", None, None)?;

            //TODO This is temporary: Only use while not defined in FW
            self.write_all_raw(b"FlashUtil()\n", None, None)?;
            self.write_all_raw(format!("updateSlot({module})\n").as_bytes(), None, None)?;

            let flash_util_global_functions = [b"flashupdate", b"flashverify", b"flashencode"];

            for func in flash_util_global_functions {
                //wait before deleting functions
                std::thread::sleep(Duration::from_millis(100));
                let _ = self.write_all_raw(
                    format!("{} = nil\n", String::from_utf8_lossy(func)).as_bytes(),
                    None,
                    None,
                );
            }

            let script_name = "FlashUtil";
            self.write_all_raw(format!("{script_name} = nil\n").as_bytes(), None, None)?;
            //TODO use this when the FW team has implemented it:
            // self.write(format!("slot[{slot_number}].firmware.update()\n").as_bytes());
        } else {
            return Err(InstrumentError::Other(
                "instrument does not support module upgrade".to_string(),
            ));
        }
        Ok(())
    }
}

impl TryFrom<ConnectionAddr> for Instrument {
    type Error = InstrumentError;

    fn try_from(value: ConnectionAddr) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Language for Instrument {
    fn get_language(&mut self) -> Result<CmdLanguage, InstrumentError> {
        if self.model.is_tti() {
            self.write_all_raw(b"*LANG?\n", None, None)?;
            for _i in 0..5 {
                std::thread::sleep(Duration::from_millis(100));
                let mut lang: Vec<u8> = vec![0; 256];
                let _read = self.read(&mut lang)?;
                let lang = std::str::from_utf8(lang.as_slice())
                    .unwrap_or("")
                    .trim_matches(char::from(0))
                    .trim();

                if lang.contains("TSP") {
                    return Ok(CmdLanguage::Tsp);
                } else if lang.contains("SCPI") {
                    return Ok(CmdLanguage::Scpi);
                }
            }
            Err(InstrumentError::InformationRetrievalError {
                details: ("could not read language of the instrument").to_string(),
            })
        } else {
            Ok(CmdLanguage::Tsp)
        }
    }

    fn change_language(&mut self, lang: CmdLanguage) -> Result<(), InstrumentError> {
        if self.model.is_tti() {
            self.write_all_raw(format!("*LANG {lang}\n").as_bytes(), None, None)?;
        }
        Ok(())
    }
}

impl Login for Instrument {
    fn check_login(&mut self) -> crate::error::Result<instrument::State> {
        if self.model.is_tti() {
            self.write_all_raw(b"*TST?\n", None, None)?;
            for _i in 0..5 {
                std::thread::sleep(Duration::from_millis(100));
                let mut resp: Vec<u8> = vec![0; 256];
                let _read_bytes = self.read(&mut resp)?;
                let resp = std::str::from_utf8(resp.as_slice())
                    .unwrap_or("")
                    .trim_matches(char::from(0))
                    .trim();

                if resp.contains("SUCCESS: Logged in") || resp.contains('0') {
                    return Ok(instrument::State::NotNeeded);
                }

                if resp.contains("FAILURE") {
                    if resp.contains("LOGOUT") {
                        return Ok(instrument::State::LogoutNeeded);
                    }
                    return Ok(instrument::State::Needed);
                }
            }
            Ok(instrument::State::Needed)
        } else if self.model.is_2600() || self.model.is_3700_70x() || self.model.is_mp() {
            self.write_all_raw(b"print('unlocked')\n", None, None)?;
            for _i in 0..5 {
                std::thread::sleep(Duration::from_millis(100));
                let mut resp: Vec<u8> = vec![0; 256];
                let _read = self.read(&mut resp)?;
                let resp = std::str::from_utf8(resp.as_slice())
                    .unwrap_or("")
                    .trim_matches(char::from(0))
                    .trim();

                if resp.contains("unlocked") {
                    return Ok(instrument::State::NotNeeded);
                }
                if resp.contains("Port in use") {
                    return Ok(instrument::State::LogoutNeeded);
                }
            }

            Ok(instrument::State::Needed)
        } else {
            Ok(instrument::State::NotNeeded)
        }
    }

    fn login(&mut self) -> crate::error::Result<()> {
        if self.model.is_other() {
            return Ok(());
        }
        let mut inst_login_state = self.check_login()?;
        if instrument::State::NotNeeded == inst_login_state {
            return Ok(());
        } else if instrument::State::LogoutNeeded == inst_login_state {
            return Err(InstrumentError::InterfaceLoginErr);
        }

        let password = self::authentication::Authentication::read_password()?;

        let login_cmd = if self.model.is_tti() {
            format!("login {password}\n")
        } else if self.model.is_2600() || self.model.is_3700_70x() || self.model.is_mp() {
            format!("password {password}\n")
        } else {
            return Ok(());
        };

        self.write_all_raw(login_cmd.as_bytes(), None, None)?;

        inst_login_state = self.check_login()?;
        if instrument::State::NotNeeded == inst_login_state {
            println!("Login successful.");
        } else if instrument::State::Needed == inst_login_state {
            return Err(InstrumentError::LoginRejected);
        }

        Ok(())
    }
}

impl Reset for Instrument {
    fn reset(&mut self) -> crate::error::Result<()> {
        trace!("calling instrument reset...");

        if self.model.is_tti() {
            let _ = self.write_all_raw(b"abort\n", None, None);
            std::thread::sleep(Duration::from_millis(100));
        }

        let _ = self.write_all_raw(b"*RST\n", None, None);
        std::thread::sleep(Duration::from_millis(100));

        if self.model.is_2600() || self.model.is_3700_70x() || self.model.is_mp() {
            let _ = self.write_all_raw(b"abort\n", None, None);
            std::thread::sleep(Duration::from_millis(100));
        }

        Ok(())
    }
}

impl Drop for Instrument {
    fn drop(&mut self) {
        if self.fw_flash_in_progress {
            return;
        }

        let _ = self.reset();

        if self.model.is_tti() {
            let _ = self.write_all_raw(b"logout\n", None, None);
        }

        if self.model.is_2600() {
            let _ = self.write_all_raw(b"password\n", None, None);
        }
    }
}
