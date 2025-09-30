use crate::statements::*;

#[allow(dead_code)]
pub trait Executable {
    fn init(&self);
    fn execute(&self, statements: &Statements) -> Result<(), Error>;
    fn cancel(&self);
    fn finish(&self);
}
