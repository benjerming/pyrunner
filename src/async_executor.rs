use crate::executor::Executable;
use crate::statements::{Error, Statements};
use crate::sync_executor::SyncExecutor;
use log::info;

#[allow(dead_code)]
pub struct AsyncExecutor {}

#[allow(dead_code)]
impl AsyncExecutor {
    pub fn new() -> Self {
        Self {}
    }
}

#[allow(dead_code)]
impl Executable for AsyncExecutor {
    fn init(&self) {}

    fn execute(&self, statements: &Statements) -> Result<(), Error> {
        info!("TODO: AsyncExecutor execute");
        Ok(())
    }

    fn cancel(&self) {
        info!("TODO: AsyncExecutor cancel");
    }

    fn finish(&self) {}
}
