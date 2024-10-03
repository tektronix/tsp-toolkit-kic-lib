pub struct Authentication;

impl Authentication {
    pub(crate) fn read_password() -> std::io::Result<String> {
        println!(
        "Instrument might be locked.\nEnter the password to unlock the instrument (no output will be shown):"
    );
        rpassword::read_password()
    }
}
