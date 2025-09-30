use pyo3::ffi::c_str;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::wrap_pyfunction;

#[pyclass]
#[derive(Debug)]
struct MyRustStruct {
    // 你的字段，例如状态
    pub status: String,
    // ... 其他字段
}

#[pymethods]
impl MyRustStruct {
    #[new]
    fn new() -> Self {
        MyRustStruct {
            status: "initial".to_string(),
        }
    }
}

#[pyfunction]
fn progress_callback(done: i32, total: i32, mut obj: PyRefMut<MyRustStruct>) -> PyResult<()> {
    // 在这里更新 Rust 侧的状态
    obj.status = format!("Progress: {}/{}", done, total);
    // ... 其他更新逻辑

    println!("Updated status: {}", obj.status); // 可选：Rust 侧日志

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::*;

    #[test]
    fn test_callback() -> PyResult<()> {
        Python::attach(|py| {
            // 创建 MyRustStruct 的 Python 实例（如果需要传递给 Python）
            let obj = Py::new(py, MyRustStruct::new())?;

            // 创建回调的 Python callable
            let callback = wrap_pyfunction!(progress_callback, py)?;

            // 示例：加载并调用 Python 代码
            // 假设 Python 代码在字符串中，或从文件加载
            let py_code = r#"
def some_python_func(callback, obj):
    total = 100
    for done in range(0, total + 1, 10):
        callback(done, total, obj)
        print(f"Python side: {done}/{total}")
"#;

            let py_module = PyModule::from_code(
                py,
                &CString::new(py_code)?,
                c_str!("example.py"),
                c_str!("example"),
            )?;

            // 调用 Python 函数，传递回调和 obj
            py_module.call_method1("some_python_func", (callback, obj))?;

            Ok(())
        })
    }
}
