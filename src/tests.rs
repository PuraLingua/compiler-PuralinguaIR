use std::{ffi::c_char, sync::LazyLock};

use compiler_base::BuildOptions;
use compiler_service::{IServiceBuild, OptionalCompilerServicePtr, boxed::BoxedCompilerService};
use pura_lingua::global;

const COMPILER_SERVICE_PATH: &str = include_str!("../COMPILER_SERVICE_PATH");
static COMPILER_SERVICE: LazyLock<libloading::Library> =
    LazyLock::new(|| unsafe { libloading::Library::new(COMPILER_SERVICE_PATH).unwrap() });

fn OpenCompilerServiceA(path: *const c_char) -> OptionalCompilerServicePtr {
    type Native = extern "C" fn(*const c_char) -> OptionalCompilerServicePtr;
    static IMPL: LazyLock<Native> = LazyLock::new(|| unsafe {
        *COMPILER_SERVICE
            .get::<Native>("OpenCompilerServiceA")
            .unwrap()
    });
    IMPL(path)
}

#[test]
fn test_load() -> global::Result<()> {
    let service = unsafe {
        BoxedCompilerService::from_raw(
            OpenCompilerServiceA(c"./MsgboxTest".as_ptr())
                .into_option()
                .unwrap(),
        )
    };

    _ = service;

    Ok(())
}

#[test]
fn test_build() -> global::Result<()> {
    let service = unsafe {
        BoxedCompilerService::from_raw(
            OpenCompilerServiceA(c"./MsgboxTest".as_ptr())
                .into_option()
                .unwrap(),
        )
    };

    service.build(&BuildOptions::builder().build())?;

    Ok(())
}
