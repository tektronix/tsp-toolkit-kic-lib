use std::{
    io::{BufRead, Read, Write},
    time::Duration,
};

use bytes::Buf;
use language::{CmdLanguage, Language};

use crate::{
    instrument::{
        self,
        authenticate::Authentication,
        info::{get_info, InstrumentInfo},
        language, Info, Login, Script,
    },
    interface::Interface,
    interface::NonBlock,
    Flash, InstrumentError,
};

pub struct Instrument {
    info: Option<InstrumentInfo>,
    interface: Box<dyn Interface>,
    auth: Box<dyn Authentication>,
}

impl Instrument {
    #[must_use]
    pub fn is(info: &InstrumentInfo) -> bool {
        info.model.as_ref().map_or(false, is_tti)
    }

    #[must_use]
    pub const fn new(interface: Box<dyn Interface>, auth: Box<dyn Authentication>) -> Self {
        Self {
            info: None,
            interface,
            auth,
        }
    }

    pub fn add_info(&mut self, info: InstrumentInfo) -> &Self {
        self.info = Some(info);
        self
    }
}

fn is_tti(model: impl AsRef<str>) -> bool {
    [
        "2450", "2470", "DMM7510", "2460", "2461", "2461-SYS", "DMM7512", "DMM6500", "DAQ6510",
    ]
    .contains(&model.as_ref())
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

impl Language for Instrument {
    fn get_language(&mut self) -> Result<CmdLanguage, InstrumentError> {
        self.write_all(b"*LANG?\n")?;
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
    }

    fn change_language(&mut self, lang: CmdLanguage) -> Result<(), InstrumentError> {
        self.write_all(format!("*LANG {lang}\n").as_bytes())?;
        Ok(())
    }
}

impl Login for Instrument {
    fn check_login(&mut self) -> crate::error::Result<instrument::State> {
        self.write_all(b"*TST?\n")?;
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
        self.write_all(format!("login {password}\n").as_bytes())?;

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

impl Flash for Instrument {
    fn flash_firmware(&mut self, image: &[u8], _: Option<u16>) -> crate::error::Result<()> {
        let mut image = image.reader();

        self.write_all(b"localnode.prompts=localnode.DISABLE\n")?;
        self.write_all(b"if ki.upgrade ~= nil and ki.upgrade.noacklater ~= nil then ki.upgrade.noacklater() end\n")?;
        self.write_all(b"prevflash\n")?;

        self.write_all(image.fill_buf().unwrap())?;

        self.write_all(b"endflash\n")?;
        Ok(())
    }
}

impl Read for Instrument {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.interface.read(buf)
    }
}

impl Write for Instrument {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.interface.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.interface.flush()
    }
}

impl NonBlock for Instrument {
    fn set_nonblocking(&mut self, enable: bool) -> crate::error::Result<()> {
        self.interface.set_nonblocking(enable)
    }
}

impl Drop for Instrument {
    fn drop(&mut self) {
        let _ = self.write_all(b"logout\n");
    }
}

#[cfg(test)]
mod unit {
    use std::{
        assert_matches::assert_matches,
        io::{BufRead, Read, Write},
    };

    use bytes::Buf;
    use mockall::{mock, Sequence};

    use crate::{
        instrument::{self, authenticate::Authentication, info::Info, Language, Login, Script},
        interface::{self, NonBlock},
        test_util, Flash, InstrumentError,
    };

    use super::Instrument;

    #[test]
    fn login_not_needed() {
        let mut interface = MockInterface::new();
        let auth = MockAuthenticate::new();
        let mut seq = Sequence::new();
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 2)
            .return_once(|buf: &mut [u8]| {
                let msg = b"0\n";
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
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 2)
            .return_once(|buf: &mut [u8]| {
                let msg = b"0\n";
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
            .withf(|buf: &[u8]| buf == b"logout\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(Box::new(interface), Box::new(auth));

        assert_matches!(instrument.check_login(), Ok(instrument::State::NotNeeded));

        assert!(instrument.login().is_ok());
    }

    #[test]
    #[allow(clippy::too_many_lines)] //Allow for now
    fn login_success() {
        let mut interface = MockInterface::new();
        let mut auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
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
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
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
            .withf(|buf: &[u8]| buf == b"login secret_token\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        // login() { second check_login() }
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
                let msg = b"0\n";
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
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
                let msg = b"0\n";
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
            .withf(|buf: &[u8]| buf == b"logout\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(Box::new(interface), Box::new(auth));

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
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
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
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
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
            .withf(|buf: &[u8]| buf == b"login secret_token\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        // login() { second check_login() }
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
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
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
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
            .withf(|buf: &[u8]| buf == b"logout\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        let mut instrument: Instrument = Instrument::new(Box::new(interface), Box::new(auth));

        assert_matches!(instrument.check_login(), Ok(instrument::State::Needed));

        assert_matches!(instrument.login(), Err(InstrumentError::LoginRejected));

        assert_matches!(instrument.check_login(), Ok(instrument::State::Needed));
    }

    #[test]
    fn info() {
        let mut interface = MockInterface::new();
        let auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

        interface
            .expect_set_nonblocking()
            .times(..)
            .returning(|_| Ok(()));

        // check_login()
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*IDN?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 50)
            .return_once(|buf: &mut [u8]| {
                let msg = b"KEITHLEY INSTRUMENTS,MODEL 2450,0123456789,1.2.3d\n";
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
            .withf(|buf: &[u8]| buf == b"logout\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        let mut instrument: Instrument = Instrument::new(Box::new(interface), Box::new(auth));

        let info = instrument
            .info()
            .expect("instrument can get instrument information from MockInterface");

        let exp_vendor = "KEITHLEY INSTRUMENTS".to_string();
        let exp_model = "2450".to_string();
        let exp_serial = "0123456789".to_string();
        let exp_fw = "1.2.3d".to_string();

        assert_eq!(info.vendor.unwrap(), exp_vendor);
        assert_eq!(info.model.unwrap(), exp_model);
        assert_eq!(info.serial_number.unwrap(), exp_serial);
        assert_eq!(info.firmware_rev.unwrap(), exp_fw);
    }

    #[test]
    fn get_language_tsp() {
        let mut interface = MockInterface::new();
        let auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*LANG?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 4)
            .return_once(|buf: &mut [u8]| {
                let msg = b"TSP\n";
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
            .withf(|buf: &[u8]| buf == b"logout\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        let mut instrument: Instrument = Instrument::new(Box::new(interface), Box::new(auth));

        assert_eq!(
            instrument.get_language().unwrap(),
            instrument::CmdLanguage::Tsp
        );
    }

    #[test]
    fn get_language_scpi() {
        let mut interface = MockInterface::new();
        let auth = MockAuthenticate::new();
        let mut seq = Sequence::new();
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*LANG?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 5)
            .return_once(|buf: &mut [u8]| {
                let msg = b"SCPI\n";
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
            .withf(|buf: &[u8]| buf == b"logout\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        let mut instrument: Instrument = Instrument::new(Box::new(interface), Box::new(auth));

        assert_eq!(
            instrument.get_language().unwrap(),
            instrument::CmdLanguage::Scpi
        );
    }

    #[test]
    fn change_language() {
        let mut interface = MockInterface::new();
        let auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*LANG TSP\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*LANG SCPI\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"logout\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(Box::new(interface), Box::new(auth));

        assert!(instrument
            .change_language(instrument::CmdLanguage::Tsp)
            .is_ok());

        assert!(instrument
            .change_language(instrument::CmdLanguage::Scpi)
            .is_ok());
    }

    #[test]
    fn write_script() {
        let optional_writes: Vec<Vec<u8>> = vec![
            (*b"logout\n").into(),
            (*b"abort\n").into(),
            (*b"_orig_prompts = localnode.prompts localnode.prompts = 0\n").into(),
            (*b"localnode.prompts = _orig_prompts _orig_prompts = nil\n").into(),
        ];
        let expected: Vec<Vec<u8>> = vec![
            (*b"test_script=nil\n").into(),
            (*b"loadscript test_script\n").into(),
            (*b"line1\nline2\nline3").into(),
            (*b"\nendscript\n").into(),
        ];
        let mut interface = MockInterface::new();
        let auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

        //Accept any number of flushes
        interface.expect_flush().times(..).returning(|| Ok(()));

        for o in optional_writes {
            interface
                .expect_write()
                .times(..)
                .withf(move |buf: &[u8]| buf == o)
                .returning(|buf: &[u8]| Ok(buf.len()));
        }

        for e in expected {
            interface
                .expect_write()
                .times(1)
                .in_sequence(&mut seq)
                .withf(move |buf: &[u8]| buf == e)
                .returning(|buf: &[u8]| Ok(buf.len()));
        }
        let mut instrument: Instrument = Instrument::new(Box::new(interface), Box::new(auth));

        instrument
            .write_script(b"test_script", &b"line1\nline2\nline3"[..], false, false)
            .expect("instrument should have written script to MockInterface");
    }

    #[test]
    fn write_script_run() {
        let optional_writes: Vec<Vec<u8>> = vec![
            (*b"logout\n").into(),
            (*b"abort\n").into(),
            (*b"_orig_prompts = localnode.prompts localnode.prompts = 0\n").into(),
            (*b"localnode.prompts = _orig_prompts _orig_prompts = nil\n").into(),
        ];
        let expected: Vec<Vec<u8>> = vec![
            (*b"test_script=nil\n").into(),
            (*b"loadscript test_script\n").into(),
            (*b"line1\nline2\nline3").into(),
            (*b"\nendscript\n").into(),
            (*b"test_script.run()\n").into(),
        ];
        let mut interface = MockInterface::new();
        let auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

        //Accept any number of flushes
        interface.expect_flush().times(..).returning(|| Ok(()));

        for o in optional_writes {
            interface
                .expect_write()
                .times(..)
                .withf(move |buf: &[u8]| buf == o)
                .returning(|buf: &[u8]| Ok(buf.len()));
        }

        for e in expected {
            interface
                .expect_write()
                .times(1)
                .in_sequence(&mut seq)
                .withf(move |buf: &[u8]| buf == e)
                .returning(|buf: &[u8]| Ok(buf.len()));
        }
        let mut instrument: Instrument = Instrument::new(Box::new(interface), Box::new(auth));

        instrument
            .write_script(b"test_script", &b"line1\nline2\nline3"[..], false, true)
            .expect("instrument should have written script to MockInterface");
    }

    #[test]
    fn write_script_save() {
        let optional_writes: Vec<Vec<u8>> = vec![
            (*b"logout\n").into(),
            (*b"abort\n").into(),
            (*b"_orig_prompts = localnode.prompts localnode.prompts = 0\n").into(),
            (*b"localnode.prompts = _orig_prompts _orig_prompts = nil\n").into(),
        ];
        let expected: Vec<Vec<u8>> = vec![
            (*b"test_script=nil\n").into(),
            (*b"loadscript test_script\n").into(),
            (*b"line1\nline2\nline3").into(),
            (*b"\nendscript\n").into(),
            (*b"test_script.save()\n").into(),
        ];
        let mut interface = MockInterface::new();
        let auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

        //Accept any number of flushes
        interface.expect_flush().times(..).returning(|| Ok(()));

        for o in optional_writes {
            interface
                .expect_write()
                .times(..)
                .withf(move |buf: &[u8]| buf == o)
                .returning(|buf: &[u8]| Ok(buf.len()));
        }

        for e in expected {
            interface
                .expect_write()
                .times(1)
                .in_sequence(&mut seq)
                .withf(move |buf: &[u8]| buf == e)
                .returning(|buf: &[u8]| Ok(buf.len()));
        }

        let mut instrument: Instrument = Instrument::new(Box::new(interface), Box::new(auth));

        instrument
            .write_script(b"test_script", &b"line1\nline2\nline3"[..], true, false)
            .expect("instrument should have written script to MockInterface");
    }

    #[test]
    fn write_script_save_run() {
        let optional_writes: Vec<Vec<u8>> = vec![
            (*b"logout\n").into(),
            (*b"abort\n").into(),
            (*b"_orig_prompts = localnode.prompts localnode.prompts = 0\n").into(),
            (*b"localnode.prompts = _orig_prompts _orig_prompts = nil\n").into(),
        ];
        let expected: Vec<Vec<u8>> = vec![
            (*b"test_script=nil\n").into(),
            (*b"loadscript test_script\n").into(),
            (*b"line1\nline2\nline3").into(),
            (*b"\nendscript\n").into(),
            (*b"test_script.save()\n").into(),
            (*b"test_script.run()\n").into(),
        ];
        let mut interface = MockInterface::new();
        let auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

        //Accept any number of flushes
        interface.expect_flush().times(..).returning(|| Ok(()));

        for o in optional_writes {
            interface
                .expect_write()
                .times(..)
                .withf(move |buf: &[u8]| buf == o)
                .returning(|buf: &[u8]| Ok(buf.len()));
        }

        for e in expected {
            interface
                .expect_write()
                .times(1)
                .in_sequence(&mut seq)
                .withf(move |buf: &[u8]| buf == e)
                .returning(|buf: &[u8]| Ok(buf.len()));
        }

        let mut instrument: Instrument = Instrument::new(Box::new(interface), Box::new(auth));

        instrument
            .write_script(b"test_script", &b"line1\nline2\nline3"[..], true, true)
            .expect("instrument should have written script to MockInterface");
    }

    #[test]
    fn flash_firmware() {
        let mut interface = MockInterface::new();
        let auth = MockAuthenticate::new();
        let mut seq = Sequence::new();
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| String::from_utf8_lossy(buf).contains("localnode.prompts"))
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"if ki.upgrade ~= nil and ki.upgrade.noacklater ~= nil then ki.upgrade.noacklater() end\n")
            .returning(|buf: &[u8]| Ok(buf.len()) );

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"prevflash\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(move |buf: &[u8]| {
                buf == test_util::SIMPLE_FAKE_BINARY_FW
                    .reader()
                    .fill_buf()
                    .unwrap()
            })
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"endflash\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"logout\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(Box::new(interface), Box::new(auth));

        instrument
            .flash_firmware(test_util::SIMPLE_FAKE_BINARY_FW, Some(0))
            .expect("instrument should have written fw to MockInterface");
    }

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
