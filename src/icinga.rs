pub enum ExitCode {
    Ok = 0,
    Warning = 1,
    Critical = 2,
    Unknown = 3,
}

impl ExitCode {
    pub fn from_i32(value: i32) -> ExitCode {
        match value {
            0 => ExitCode::Ok,
            1 => ExitCode::Warning,
            2 => ExitCode::Critical,
            3 => ExitCode::Unknown,
            _ => ExitCode::Unknown,
        }
    }
}

pub trait IcingaTermination {
    fn exitcode(&self) -> ExitCode;

    fn report(&self);
}

impl IcingaTermination for Box<dyn std::error::Error> {
    fn exitcode(&self) -> ExitCode {
        ExitCode::Unknown
    }

    fn report(&self) {
        println!("{}", self);
        std::process::exit(self.exitcode() as i32);
    }
}

pub fn icinga_exit<T, E>(result: Result<T, E>)
where
    T: IcingaTermination,
    E: IcingaTermination,
{
    match result {
        Ok(termination) => {
            termination.report();
        }
        Err(termination) => {
            termination.report();
        }
    }
}
