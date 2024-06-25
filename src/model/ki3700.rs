use std::{
    io::{Read, Write},
    time::Duration,
};

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
        info.model.as_ref().map_or(false, |model| {
            model.split_ascii_whitespace().last().map_or(false, is_3700)
        })
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

//Implement device_interface::Interface since it is a subset of instrument::Instrument trait.
impl instrument::Instrument for Instrument {}

fn is_3700(model: impl AsRef<str>) -> bool {
    [
        "3706",
        "3706-SNFP",
        "3706-S",
        "3706-NFP",
        "3706A",
        "3706A-SNFP",
        "3706A-S",
        "3706A-NFP",
        "707B",
        "708B",
    ]
    .contains(&model.as_ref())
}

impl Info for Instrument {
    fn info(&mut self) -> crate::error::Result<InstrumentInfo> {
        if let Some(inst_info) = self.info.clone() {
            return Ok(inst_info);
        }

        get_info(self)
    }
}

impl language::Language for Instrument {}

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

impl Flash for Instrument {
    /*
    Note: The packet size and delay was experimentally obtianed here.
    Anything over 4096 chunk size and under 10ms delay causes the unit
    to hang up on the unpdate around 6800k.
     */
    fn flash_firmware(&mut self, image: &[u8], _: Option<u16>) -> crate::error::Result<()> {
        self.write_all(b"localnode.prompts = 0\n")?;
        self.write_all(b"prevflash\n")?;

        for chunk in image.chunks(4096) {
            self.write_all(chunk)?;
            std::thread::sleep(Duration::from_millis(10)); //The position and duration of this delay is intentional
        }

        std::thread::sleep(Duration::from_millis(10)); //The position and duration of this delay is intentional
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
        let _ = self.interface.write_all(b"password\n");
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
        instrument::{self, authenticate::Authentication, info::Info, Login, Script},
        interface::NonBlock,
        interface::{self},
        test_util, Flash, InstrumentError,
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
            .withf(|buf: &[u8]| buf == b"password\n")
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
    #[allow(clippy::too_many_lines)] //Allow for now.
    fn login_success() {
        let mut _interface = MockInterface::new();
        let mut interface = MockInterface::new();
        let mut auth = MockAuthenticate::new();

        let mut seq = Sequence::new();

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
            .withf(|buf: &[u8]| buf == b"password\n")
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
        let mut _interface = MockInterface::new();
        let mut interface = MockInterface::new();
        let mut auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

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
            .withf(|buf: &[u8]| buf == b"password\n")
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
        let mut _interface = MockInterface::new();
        let mut interface = MockInterface::new();
        let auth = MockAuthenticate::new();
        let mut seq = Sequence::new();

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
                let msg = b"KEITHLEY INSTRUMENTS INC.,MODEL 3706A,0123456789,1.2.3d\n";
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
            .withf(|buf: &[u8]| buf == b"password\n")
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

        let exp_vendor = "KEITHLEY INSTRUMENTS INC.".to_string();
        let exp_model = "3706A".to_string();
        let exp_serial = "0123456789".to_string();
        let exp_fw = "1.2.3d".to_string();

        assert_eq!(info.vendor.unwrap(), exp_vendor);
        assert_eq!(info.model.unwrap(), exp_model);
        assert_eq!(info.serial_number.unwrap(), exp_serial);
        assert_eq!(info.firmware_rev.unwrap(), exp_fw);
    }

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
            .withf(|buf: &[u8]| buf == b"password\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(Box::new(interface), Box::new(auth));

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
            .withf(|buf: &[u8]| buf == b"password\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(Box::new(interface), Box::new(auth));

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
            .withf(|buf: &[u8]| buf == b"password\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(Box::new(interface), Box::new(auth));

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
            .withf(|buf: &[u8]| buf == b"password\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
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
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"localnode.prompts = 0\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

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
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"password\n")
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
