use std::{
    io::{BufRead, Read, Write},
    time::Duration,
};

use crate::{
    instrument::{
        self,
        authenticate::Authentication,
        clear_output_queue,
        info::{get_info, InstrumentInfo},
        language, Info, Login, Reset, Script,
    },
    interface::NonBlock,
    protocol::Protocol,
    Flash, InstrumentError,
};
use bytes::Buf;
use language::Language;
use tracing::trace;

pub struct Instrument {
    info: Option<InstrumentInfo>,
    protocol: Protocol,
    auth: Box<dyn Authentication>,
    fw_flash_in_progress: bool,
}

impl Instrument {
    #[must_use]
    pub fn is(info: &InstrumentInfo) -> bool {
        info.model.as_ref().is_some_and(Self::model_is)
    }

    #[must_use]
    pub fn model_is(model: impl AsRef<str>) -> bool {
        model
            .as_ref()
            .split_ascii_whitespace()
            .last()
            .is_some_and(is_versatest)
    }

    #[must_use]
    pub const fn new(protocol: Protocol, auth: Box<dyn Authentication>) -> Self {
        Self {
            info: None,
            protocol,
            auth,
            fw_flash_in_progress: false,
        }
    }

    pub fn add_info(&mut self, info: InstrumentInfo) -> &Self {
        self.info = Some(info);
        self
    }
}

fn is_versatest(model: impl AsRef<str>) -> bool {
    ["MP5103", "VERSATEST-300", "VERSATEST-600", "TSPop", "TSP"].contains(&model.as_ref())
}

//Implement device_interface::Interface since it is a subset of instrument::Instrument trait.
impl instrument::Instrument for Instrument {}

impl Info for Instrument {
    fn info(&mut self) -> crate::error::Result<InstrumentInfo> {
        if let Some(inst_info) = self.info.clone() {
            return Ok(inst_info);
        }

        get_info(self)
    }
}

impl Language for Instrument {}

impl Login for Instrument {
    fn check_login(&mut self) -> crate::error::Result<instrument::State> {
        self.write_all(b"print('unlocked')\n")?;
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
    }

    fn login(&mut self) -> crate::error::Result<()> {
        let mut inst_login_state = self.check_login()?;
        if instrument::State::NotNeeded == inst_login_state {
            return Ok(());
        } else if instrument::State::LogoutNeeded == inst_login_state {
            return Err(InstrumentError::InterfaceLoginErr);
        }

        println!("Instrument might be locked.\nEnter the password to unlock the instrument:");
        let password = self.auth.read_password()?;
        self.write_all(format!("password {password}\n").as_bytes())?;

        inst_login_state = self.check_login()?;
        if instrument::State::NotNeeded == inst_login_state {
            println!("Login successful.");
        } else if instrument::State::Needed == inst_login_state {
            return Err(InstrumentError::LoginRejected);
        }

        Ok(())
    }
}

impl Script for Instrument {}

impl Read for Instrument {
    #[tracing::instrument(skip(self))]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let b = self.protocol.read(buf)?;
        let ascii = String::from_utf8_lossy(buf);
        let ascii = ascii.trim_end().trim_matches(['\0', '\n', '\r']);
        if !ascii.is_empty() {
            trace!("read from instrument: '{ascii}'");
        }
        Ok(b)
    }
}

impl Write for Instrument {
    #[tracing::instrument(skip(self))]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        trace!("writing to instrument: '{}'", String::from_utf8_lossy(buf));
        self.protocol.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.protocol.flush()
    }
}

impl Flash for Instrument {
    fn flash_firmware(
        &mut self,
        image: &[u8],
        firmware_info: Option<u16>,
    ) -> crate::error::Result<()> {
        self.fw_flash_in_progress = true;
        let mut is_module = false;
        let slot_number: u16 = firmware_info.unwrap_or(0);
        if slot_number > 0 {
            is_module = true;
        }

        self.write_all(b"localnode.prompts=0\n")?;
        let mut image = image.reader();
        self.write_all(b"flash\n")?;

        self.write_all(image.fill_buf().unwrap())?;

        self.write_all(b"endflash\n")?;

        // give it up to 10 minutes.
        // This call will write a timestamp to be printed by the instrument
        // and will then poll (if `self` is non-blocking) to read the timestamp
        // back.
        match clear_output_queue(self, 60 * 10, Duration::from_secs(1)) {
            Ok(()) => {}
            Err(InstrumentError::Other(_)) => return Err(InstrumentError::Other(
                "Writing image took longer than 10 minutes. Check your connection and try again.".to_string(),
            )),
            Err(e) => return Err(e),
        }

        //TODO CHECK ERRORS

        if is_module {
            self.write_all(format!("slot[{slot_number}].firmware.update()\n").as_bytes())?;
        } else {
            //Update Mainframe
            self.write_all(b"firmware.update()\n")?;
        }

        Ok(())
    }
}

impl NonBlock for Instrument {
    fn set_nonblocking(&mut self, enable: bool) -> crate::error::Result<()> {
        match &mut self.protocol {
            Protocol::Raw(r) => r.set_nonblocking(enable),

            #[cfg(feature = "visa")]
            Protocol::Visa { .. } => Ok(()),
        }
    }
}

impl Drop for Instrument {
    #[tracing::instrument(skip(self))]
    fn drop(&mut self) {
        trace!("calling versatest drop...");
        if self.fw_flash_in_progress {
            trace!("FW flash in progress. Skipping instrument reset.");
            return;
        }
        let _ = self.reset();
    }
}

impl Reset for Instrument {
    fn reset(&mut self) -> crate::error::Result<()> {
        trace!("calling versatest reset...");
        let _ = self.write_all(b"*RST\n");
        std::thread::sleep(Duration::from_millis(100));
        let _ = self.write_all(b"abort\n");
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }
}

#[cfg(test)]
mod unit {
    use crate::protocol;
    use std::{
        assert_matches::assert_matches,
        io::{BufRead, Read, Write},
    };

    use bytes::Buf;
    use mockall::{mock, Sequence};

    use crate::{
        instrument::{self, authenticate::Authentication, info::Info, Login, Script},
        interface::{self, NonBlock}, InstrumentError,
    };

    use super::Instrument;

    #[test]
    fn login_not_needed() {
        let mut interface = MockInterface::new();
        let auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

        // A successful login attempt on a TTI instrument is as follows:
        // 1. Instrument connects to interface
        // 2. Instrument sends "*STB?\n"
        // 3. Instrument reads from interface and receives status byte
        // 4. Instrument returns `instrument::State::NotNeeded`

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 2)
            .return_once(|buf: &mut [u8]| {
                let msg = b"unlocked\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInterface should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 2)
            .return_once(|buf: &mut [u8]| {
                let msg = b"unlocked\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInterface should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"*RST\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        let mut instrument: Instrument =
            Instrument::new(protocol::Protocol::Raw(Box::new(interface)), Box::new(auth));

        assert_matches!(instrument.check_login(), Ok(instrument::State::NotNeeded));

        assert!(instrument.login().is_ok());
    }

    #[test]
    #[allow(clippy::too_many_lines)] //Allow for now.
    fn login_success() {
        let mut interface = MockInterface::new();
        let mut auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(5)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .returning(|buf: &mut [u8]| {
                let msg = b"FAILURE\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        // login() { first check_login() }
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(5)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .returning(|buf: &mut [u8]| {
                let msg = b"FAILURE\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        auth.expect_read_password()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok("secret_token".to_string()));

        // login() {write(b"login {token}")}
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"password secret_token\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        // login() { second check_login() }
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
                let msg = b"unlocked\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        // check_login()
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
                let msg = b"unlocked\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"*RST\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument =
            Instrument::new(protocol::Protocol::Raw(Box::new(interface)), Box::new(auth));

        assert_matches!(instrument.check_login(), Ok(instrument::State::Needed));

        assert_matches!(instrument.login(), Ok(()));

        assert_matches!(instrument.check_login(), Ok(instrument::State::NotNeeded));
    }

    #[test]
    #[allow(clippy::too_many_lines)] //Allow for now
    fn login_failure() {
        let mut interface = MockInterface::new();
        let mut auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(5)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .returning(|buf: &mut [u8]| {
                let msg = b"FAILURE\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        // login() { first check_login() }
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(5)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .returning(|buf: &mut [u8]| {
                let msg = b"FAILURE\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        auth.expect_read_password()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok("secret_token".to_string()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"password secret_token\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        // login() { second check_login() }
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(5)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .returning(|buf: &mut [u8]| {
                let msg = b"FAILURE\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        // check_login()
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(5)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .returning(|buf: &mut [u8]| {
                let msg = b"FAILURE\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"*RST\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument =
            Instrument::new(protocol::Protocol::Raw(Box::new(interface)), Box::new(auth));

        assert_matches!(instrument.check_login(), Ok(instrument::State::Needed));

        assert_matches!(instrument.login(), Err(InstrumentError::LoginRejected));

        assert_matches!(instrument.check_login(), Ok(instrument::State::Needed));
    }

    //#[test]
    //fn info() {
    //    let mut interface = MockInterface::new();
    //    let auth = MockAuthenticate::new();
    //    let mut seq = Sequence::new();

    //    interface
    //        .expect_write()
    //        .times(1)
    //        .in_sequence(&mut seq)
    //        .withf(|buf: &[u8]| buf == b"*IDN?\n")
    //        .returning(|buf: &[u8]| Ok(buf.len()));

    //    interface
    //        .expect_read()
    //        .times(1)
    //        .in_sequence(&mut seq)
    //        .withf(|buf: &[u8]| buf.len() >= 50)
    //        .return_once(|buf: &mut [u8]| {
    //            let msg = b"KEITHLEY INSTRUMENTS,MODEL 2450,0123456789,1.2.3d\n";
    //            if buf.len() >= msg.len() {
    //                let bytes = msg[..]
    //                    .reader()
    //                    .read(buf)
    //                    .expect("MockInterface should write to buffer");
    //                assert_eq!(bytes, msg.len());
    //            }
    //            Ok(msg.len())
    //        });
    //    interface
    //        .expect_write()
    //        .times(..)
    //        .withf(|buf: &[u8]| buf == b"*RST\n")
    //        .returning(|buf: &[u8]| Ok(buf.len()));
    //    interface
    //        .expect_write()
    //        .times(..)
    //        .withf(|buf: &[u8]| buf == b"abort\n")
    //        .returning(|buf: &[u8]| Ok(buf.len()));

    //    let mut instrument: Instrument =
    //        Instrument::new(protocol::Protocol::Raw(Box::new(interface)), Box::new(auth));

    //    let info = instrument
    //        .info()
    //        .expect("instrument can get instrument information from MockInterface");

    //    let exp_vendor = "KEITHLEY INSTRUMENTS".to_string();
    //    let exp_model = "2450".to_string();
    //    let exp_serial = "0123456789".to_string();
    //    let exp_fw = "1.2.3d".to_string();

    //    assert_eq!(info.vendor.unwrap(), exp_vendor);
    //    assert_eq!(info.model.unwrap(), exp_model);
    //    assert_eq!(info.serial_number.unwrap(), exp_serial);
    //    assert_eq!(info.firmware_rev.unwrap(), exp_fw);
    //}

    #[test]
    fn write_script() {
        let mut interface = MockInterface::new();
        let auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"_orig_prompts = localnode.prompts localnode.prompts = 0\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"localnode.prompts = _orig_prompts _orig_prompts = nil\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        //Accept any number of flushes
        interface.expect_flush().times(..).returning(|| Ok(()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"test_script=nil\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"loadscript test_script\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(move |buf: &[u8]| buf == b"line1\nline2\nline3"[..].reader().fill_buf().unwrap())
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"\nendscript\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"*RST\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument =
            Instrument::new(protocol::Protocol::Raw(Box::new(interface)), Box::new(auth));

        instrument
            .write_script(b"test_script", &b"line1\nline2\nline3"[..], false, false)
            .expect("instrument should have written script to MockInterface");
    }

    #[test]
    fn write_script_run() {
        let mut interface = MockInterface::new();
        let auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"_orig_prompts = localnode.prompts localnode.prompts = 0\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"localnode.prompts = _orig_prompts _orig_prompts = nil\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        //Accept any number of flushes
        interface.expect_flush().times(..).returning(|| Ok(()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"test_script=nil\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"loadscript test_script\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(move |buf: &[u8]| buf == b"line1\nline2\nline3"[..].reader().fill_buf().unwrap())
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"\nendscript\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"test_script.run()\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"*RST\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument =
            Instrument::new(protocol::Protocol::Raw(Box::new(interface)), Box::new(auth));

        instrument
            .write_script(b"test_script", &b"line1\nline2\nline3"[..], false, true)
            .expect("instrument should have written script to MockInterface");
    }

    #[test]
    fn write_script_save() {
        let mut interface = MockInterface::new();
        let auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"_orig_prompts = localnode.prompts localnode.prompts = 0\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"localnode.prompts = _orig_prompts _orig_prompts = nil\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        //Accept any number of flushes
        interface.expect_flush().times(..).returning(|| Ok(()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"test_script=nil\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"loadscript test_script\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(move |buf: &[u8]| buf == b"line1\nline2\nline3"[..].reader().fill_buf().unwrap())
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"\nendscript\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"test_script.save()\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"*RST\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument =
            Instrument::new(protocol::Protocol::Raw(Box::new(interface)), Box::new(auth));

        instrument
            .write_script(b"test_script", &b"line1\nline2\nline3"[..], true, false)
            .expect("instrument should have written script to MockInterface");
    }

    #[test]
    fn write_script_save_run() {
        let mut interface = MockInterface::new();
        let auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"_orig_prompts = localnode.prompts localnode.prompts = 0\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"localnode.prompts = _orig_prompts _orig_prompts = nil\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        //Accept any number of flushes
        interface.expect_flush().times(..).returning(|| Ok(()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"test_script=nil\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"loadscript test_script\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(move |buf: &[u8]| buf == b"line1\nline2\nline3"[..].reader().fill_buf().unwrap())
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"\nendscript\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"test_script.save()\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"test_script.run()\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"*RST\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        let mut instrument: Instrument =
            Instrument::new(protocol::Protocol::Raw(Box::new(interface)), Box::new(auth));

        instrument
            .write_script(b"test_script", &b"line1\nline2\nline3"[..], true, true)
            .expect("instrument should have written script to MockInterface");
    }

    //#[test] // requires timestamp to function, isn't worth it.
    //fn flash_firmware() {
    //    let mut interface = MockInterface::new();
    //    let auth = MockAuthenticate::new();
    //    let mut seq = Sequence::new();

    //    interface
    //        .expect_write()
    //        .times(1)
    //        .in_sequence(&mut seq)
    //        .withf(|buf: &[u8]| buf == b"localnode.prompts=0\n")
    //        .returning(|buf: &[u8]| Ok(buf.len()));

    //    interface
    //        .expect_write()
    //        .times(1)
    //        .in_sequence(&mut seq)
    //        .withf(|buf: &[u8]| buf == b"flash\n")
    //        .returning(|buf: &[u8]| Ok(buf.len()));

    //    interface
    //        .expect_write()
    //        .times(1)
    //        .in_sequence(&mut seq)
    //        .withf(move |buf: &[u8]| {
    //            buf == test_util::SIMPLE_FAKE_TEXTUAL_FW
    //                .reader()
    //                .fill_buf()
    //                .unwrap()
    //        })
    //        .returning(|buf: &[u8]| Ok(buf.len()));

    //    interface
    //        .expect_write()
    //        .times(1)
    //        .in_sequence(&mut seq)
    //        .withf(|buf: &[u8]| buf == b"endflash\n")
    //        .returning(|buf: &[u8]| Ok(buf.len()));


    //    interface
    //        .expect_write()
    //        .times(1)
    //        .in_sequence(&mut seq)
    //        .withf(|buf: &[u8]| buf == b"firmware.update()\n")
    //        .returning(|buf: &[u8]| Ok(buf.len()));
    //    interface
    //        .expect_write()
    //        .times(..)
    //        .withf(|buf: &[u8]| buf == b"*RST\n")
    //        .returning(|buf: &[u8]| Ok(buf.len()));
    //    interface
    //        .expect_write()
    //        .times(..)
    //        .withf(|buf: &[u8]| buf == b"abort\n")
    //        .returning(|buf: &[u8]| Ok(buf.len()));

    //    let mut instrument: Instrument =
    //        Instrument::new(protocol::Protocol::Raw(Box::new(interface)), Box::new(auth));

    //    instrument
    //        .flash_firmware(test_util::SIMPLE_FAKE_TEXTUAL_FW, Some(0))
    //        .expect("instrument should have written fw to MockInterface");
    //}

    // Define a mock interface to be used in the tests above.
    mock! {
        Interface {}

        impl interface::Interface for Interface {}


        impl Read for Interface {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
        }

        impl Write for Interface {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;

            fn flush(&mut self) -> std::io::Result<()>;
        }
        impl NonBlock for Interface {
            fn set_nonblocking(&mut self, enable: bool) -> crate::error::Result<()>;
        }
        impl Info for Interface {}
    }

    // Define a mock Authenticate to be used in the tests above.
    mock! {

        Authenticate {}
        impl Authentication for Authenticate {
            fn read_password(&self) -> std::io::Result<String>;
        }

    }
}
