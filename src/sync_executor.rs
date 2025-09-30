use crate::executor::Executable;
use crate::statements::{Error, Statements};
use log::info;

#[allow(dead_code)]
pub struct SyncExecutor {}

#[allow(dead_code)]
impl SyncExecutor {
    pub fn new() -> Self {
        Self {}
    }
}

#[allow(dead_code)]
impl Executable for SyncExecutor {
    fn init(&self) {
        info!("TODO: python::initialize")
    }

    fn execute(&self, statements: &Statements) -> Result<(), Error> {
        statements.execute()
    }

    fn cancel(&self) {
        info!("TODO: take GIL and exit from python")
    }

    fn finish(&self) {
        info!("TODO: python::finalize")
    }
}
