use std::collections::HashMap;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
#[allow(unused_imports)]
use pyo3::ffi::c_str;
#[allow(unused_imports)]
use pyo3::prelude::*;
#[allow(unused_imports)]
use pyo3::types::IntoPyDict;
#[allow(unused_imports)]
use pyo3::types::PyDict;
use thiserror;

#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Variable not found: {0}")]
    VariableNotFound(String),
    #[error(transparent)]
    PyError(#[from] PyErr),
}

#[allow(dead_code)]
enum Step {
    // Import(result_var_name, module_name)
    Import(String, String),
    // GetAttr(result_var_name, object_var_name, attr_name)
    GetAttr(String, String, String),
    // Run(code_text)
    Run(String),
    // Call(result_var_name, function_var_name, *args, **kwargs)
    Call(String, String, Vec<String>, HashMap<String, String>),
}

#[allow(dead_code)]
pub struct Statements {
    steps: Vec<Step>,
}

#[allow(dead_code)]
impl Statements {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn import(&mut self, _result_var_name: &str, _name: &str) {
        self.steps.push(Step::Import(
            _result_var_name.to_string(),
            _name.to_string(),
        ));
    }

    pub fn getattr(&mut self, _result_var_name: &str, _object_var_name: &str, _attr_name: &str) {
        self.steps.push(Step::GetAttr(
            _result_var_name.to_string(),
            _object_var_name.to_string(),
            _attr_name.to_string(),
        ));
    }

    pub fn run(&mut self, _code_text: &str) {
        self.steps.push(Step::Run(_code_text.to_string()));
    }

    pub fn call(
        &mut self,
        _result_var_name: &str,
        _function_var_name: &str,
        _args: Vec<String>,
        _kwargs: HashMap<String, String>,
    ) {
        self.steps.push(Step::Call(
            _result_var_name.to_string(),
            _function_var_name.to_string(),
            _args,
            _kwargs,
        ));
    }

    pub fn execute(&self) -> Result<(), Error> {
        Python::attach(|_py| {
            info!("TODO: execute statements");
            //         let mut variables = PyDict::new(_py);
            //         for task in self.steps.iter() {
            //             match task {
            //                 Step::Import(result_var_name, name) => {
            //                     let module = _py.import(name)?;
            //                     variables.set_item(result_var_name.to_string(), module)?;
            //                 }
            //                 Step::GetAttr(result_var_name, object_var_name, attr_name) => {
            //                     let object = variables
            //                         .get_item(object_var_name)?
            //                         .ok_or(Error::VariableNotFound(object_var_name.to_string()))?;
            //                     let attr = object.getattr(attr_name)?;
            //                     variables.set_item(result_var_name.to_string(), attr)?;
            //                 }
            //                 Step::Run(code_text) => {}
            //                 Step::Call(result_var_name, function_var_name, args, kwargs) => {}
            //             }
            //         }
            Ok(())
        })
    }
}

// fn _call_pdfconvert_convert() -> PyResult<()> {
//     Python::attach(|py| {
//         let pdfconvert = py.import("pdfconvert")?;
//         let convert = pdfconvert.getattr("convert")?;
//         let args = ();
//         let kwargs = HashMap::<String, String>::new().into_py_dict(py)?;
//         let result: (i32, i32, i32, i32) = convert.call(args, Some(&kwargs))?.extract()?;
//         let (_ret, _errcode, _pages, _words) = result;
//         Ok(())
//     })
// }

#[cfg(test)]
mod tests {
    use super::*;

    fn _python_get_user_with() -> PyResult<String> {
        Python::attach(|py| {
            let os = py.import("os")?;
            let locals = [("os", os)].into_py_dict(py)?;
            let code_text = c_str!("os.getenv('USER') or os.getenv('USERNAME')");
            py.eval(code_text, None, Some(&locals))?.extract()
        })
    }

    fn _rust_get_user_with() -> Result<String, std::env::VarError> {
        std::env::var("USER").or_else(|_| std::env::var("USERNAME"))
    }

    #[test]
    fn test_call_python() {
        let user_from_python = _python_get_user_with().unwrap();
        let user_from_rust = _rust_get_user_with().unwrap();
        assert_eq!(user_from_python, user_from_rust);
    }
}
