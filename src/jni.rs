use crate::{error::Result, ipc::MessageSender};
use jni::JNIEnv;
use jni::objects::JClass;
#[allow(unused_imports)]
use jni::sys::{jboolean, jfloat, jint, jstring};
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

const TAG: &str = "libpr";

fn init_logger() {
    #[cfg(target_os = "android")]
    {
        use tracing_subscriber::layer::SubscriberExt;

        let android_layer = tracing_android::layer(TAG).unwrap();
        let subscriber = tracing_subscriber::registry().with(android_layer);
        let _ = tracing::subscriber::set_global_default(subscriber);
    }

    #[cfg(not(target_os = "android"))]
    {
        use tracing_subscriber::EnvFilter;

        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .try_init()
            .ok();
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_TypeConverter_convertString(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    init_logger();

    let rust_string = String::from("这是一个来自Rust的字符串");

    info!("Converting Rust String: {}", rust_string);

    match env.new_string(&rust_string) {
        Ok(java_string) => java_string.into_raw(),
        Err(e) => {
            error!("Failed to convert string: {:?}", e);
            env.new_string("").unwrap().into_raw()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_TypeConverter_convertI32(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    init_logger();

    let rust_i32: i32 = -12345;

    info!("Converting Rust i32: {}", rust_i32);

    rust_i32 as jint
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_TypeConverter_convertU32(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    init_logger();

    let rust_u32: u32 = 3000000000;

    info!("Converting Rust u32: {}", rust_u32);

    if rust_u32 > i32::MAX as u32 {
        warn!("u32 value {} exceeds i32 max, will overflow", rust_u32);
    }

    rust_u32 as jint
}

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

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_TypeConverter_convertF32(
    _env: JNIEnv,
    _class: JClass,
) -> jfloat {
    init_logger();

    let rust_f32: f32 = 3.14159265;

    info!("Converting Rust f32: {}", rust_f32);

    rust_f32 as jfloat
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_TypeConverter_convertBool(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    init_logger();

    let rust_bool: bool = true;

    info!("Converting Rust bool: {}", rust_bool);

    if rust_bool { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_TypeConverter_processString(
    mut env: JNIEnv,
    _class: JClass,
    input: jstring,
) -> jstring {
    init_logger();

    let java_string = unsafe { jni::objects::JString::from_raw(input) };
    let input_string: String = match env.get_string(&java_string) {
        Ok(java_str) => java_str.into(),
        Err(e) => {
            error!("Failed to get string from Java: {:?}", e);
            return env.new_string("").unwrap().into_raw();
        }
    };

    info!("Processing input string: {}", input_string);

    let processed = format!("Processed by Rust: {}", input_string.to_uppercase());

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

fn run_python_pdf2wps(sender: MessageSender, task_id: u64) {}

fn run_python_raw2wps(sender: MessageSender, task_id: u64) {}

fn pdf2wps(
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

#[unsafe(no_mangle)]
pub extern "system" fn Java_androidx_appcompat_ConvertCore_pdf2wps(
    mut _env: JNIEnv,
    _class: JClass,
    pdf_path: jstring,
    pdf_password: jstring,
    wps_path: jstring,
) -> jint {
    init_logger();

    match pdf2wps(&mut _env, pdf_path, pdf_password, wps_path) {
        Ok(_) => 0 as jint,
        Err(e) => {
            error!("Failed to convert PDF to WPS: {:?}", e);
            1 as jint
        }
    }
}
