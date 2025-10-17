use crate::error::Result;
use jni::JNIEnv;
use jni::objects::JClass;
#[allow(unused_imports)]
use jni::sys::{jboolean, jfloat, jint, jstring};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

const TAG: &str = "libpr";

// 初始化日志记录器的辅助函数
fn init_logger() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Trace)
            .with_tag(TAG), // logcat 中的 TAG
    );
}

// Rust String -> Java String
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_TypeConverter_convertString(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    init_logger();

    // 模拟一个Rust String
    let rust_string = String::from("这是一个来自Rust的字符串");

    info!("Converting Rust String: {}", rust_string);

    // 将Rust String转换为Java String
    match env.new_string(&rust_string) {
        Ok(java_string) => java_string.into_raw(),
        Err(e) => {
            error!("Failed to convert string: {:?}", e);
            env.new_string("").unwrap().into_raw()
        }
    }
}

// Rust i32 -> Java int
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_TypeConverter_convertI32(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    init_logger();

    // 模拟一个Rust i32值
    let rust_i32: i32 = -12345;

    info!("Converting Rust i32: {}", rust_i32);

    // i32直接转换为jint (它们是相同的类型)
    rust_i32 as jint
}

// Rust u32 -> Java int (注意：Java没有无符号整数，需要小心处理)
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_TypeConverter_convertU32(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    init_logger();

    // 模拟一个Rust u32值
    let rust_u32: u32 = 3000000000; // 这个值超过了i32的最大值

    info!("Converting Rust u32: {}", rust_u32);

    // 将u32转换为i32，可能会溢出
    // 在实际应用中，你可能需要检查溢出或使用long类型
    if rust_u32 > i32::MAX as u32 {
        warn!("u32 value {} exceeds i32 max, will overflow", rust_u32);
    }

    rust_u32 as jint
}

// 更安全的u32转换，返回long类型
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_TypeConverter_convertU32ToLong(
    _env: JNIEnv,
    _class: JClass,
) -> jni::sys::jlong {
    init_logger();

    let rust_u32: u32 = 3000000000;

    info!("Converting Rust u32 to Java long: {}", rust_u32);

    rust_u32 as jni::sys::jlong
}

// Rust f32 -> Java float
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_TypeConverter_convertF32(
    _env: JNIEnv,
    _class: JClass,
) -> jfloat {
    init_logger();

    // 模拟一个Rust f32值
    let rust_f32: f32 = 3.14159265;

    info!("Converting Rust f32: {}", rust_f32);

    // f32直接转换为jfloat
    rust_f32 as jfloat
}

// Rust bool -> Java boolean
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_TypeConverter_convertBool(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    init_logger();

    // 模拟一个Rust bool值
    let rust_bool: bool = true;

    info!("Converting Rust bool: {}", rust_bool);

    // bool转换为jboolean (0表示false，非0表示true)
    if rust_bool { 1 } else { 0 }
}

// 复合函数：接受参数并返回转换后的值
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_TypeConverter_processString(
    mut env: JNIEnv,
    _class: JClass,
    input: jstring,
) -> jstring {
    init_logger();

    // 将Java String转换为Rust String
    let java_string = unsafe { jni::objects::JString::from_raw(input) };
    let input_string: String = match env.get_string(&java_string) {
        Ok(java_str) => java_str.into(),
        Err(e) => {
            error!("Failed to get string from Java: {:?}", e);
            return env.new_string("").unwrap().into_raw();
        }
    };

    info!("Processing input string: {}", input_string);

    // 在Rust中处理字符串
    let processed = format!("Processed by Rust: {}", input_string.to_uppercase());

    // 返回处理后的字符串
    match env.new_string(&processed) {
        Ok(java_string) => java_string.into_raw(),
        Err(e) => {
            error!("Failed to create return string: {:?}", e);
            env.new_string("").unwrap().into_raw()
        }
    }
}

fn jstring_to_string(env: &mut JNIEnv, jstring: jstring) -> Result<String> {
    Ok(env
        .get_string(&unsafe { jni::objects::JString::from_raw(jstring) })?
        .into())
}

fn _pdf2wps(
    env: &mut JNIEnv,
    pdf_path: jstring,
    pdf_password: jstring,
    wps_path: jstring,
) -> Result<()> {
    let pdf_path: String = jstring_to_string(env, pdf_path)?;
    let pdf_password: String = jstring_to_string(env, pdf_password)?;
    let wps_path: String = jstring_to_string(env, wps_path)?;
    Ok(())
}

// 复合函数：接受多个参数并返回计算结果
#[unsafe(no_mangle)]
pub extern "system" fn Java_androidx_appcompat_ConvertCore_pdf2wps(
    mut _env: JNIEnv,
    _class: JClass,
    pdf_path: jstring,
    pdf_password: jstring,
    wps_path: jstring,
) -> jint {
    init_logger();

    match _pdf2wps(&mut _env, pdf_path, pdf_password, wps_path) {
        Ok(_) => 0 as jint,
        Err(e) => {
            error!("Failed to convert PDF to WPS: {:?}", e);
            1 as jint
        }
    }
}
