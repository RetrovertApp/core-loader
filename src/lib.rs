use anyhow::{bail, Context, Result};
use core::ffi::c_void;
use directories::ProjectDirs;
use libloading::{Library, Symbol};
use log::trace;
use log::{LevelFilter, Log, SetLoggerError};
use simplelog::*;
use std::fs::File;
use std::path::Path;

#[allow(dead_code)]
pub type SetupLogger =
    fn(logger: &'static dyn Log, level: LevelFilter) -> Result<(), SetLoggerError>;

#[allow(dead_code)]
pub type CoreCreate = fn() -> *mut c_void;

#[allow(dead_code)]
pub type CoreUpdate = fn(core: *mut c_void);

#[allow(dead_code)]
pub type CoreDestroy = fn(core: *mut c_void, prepare_reflesh: bool);

pub struct Core<'a> {
    pub core_create_func: Symbol<'a, CoreCreate>,
    pub core_destroy_func: Symbol<'a, CoreDestroy>,
    pub core_update_func: Symbol<'a, CoreUpdate>,
}

/// Finds the data directory relative to the executable.
/// This is because it's possible to have data next to the exe, but also running
/// the applications as targeht/path/exe and the location is in the root then
fn find_data_directory() -> Result<()> {
    let current_path = std::env::current_dir().with_context(|| "Unable to get current dir!")?;
    if current_path.join("data").exists() {
        return Ok(());
    }

    let mut path = current_path
        .parent()
        .with_context(|| format!("Unable to get parent dir"))?;

    loop {
        trace!("seaching for data in {:?}", path);

        if path.join("data").exists() {
            std::env::set_current_dir(path)?;
            return Ok(());
        }

        path = path.parent().with_context(|| "Unable to get parent dir")?;
    }
}

impl<'a> Core<'a> {
    pub fn init_logging() -> Result<()> {
        let dirs = match ProjectDirs::from("app", "tbl", "retrovert") {
            Some(dirs) => dirs,
            None => bail!("Unable to get a user directory for config and log output. Please report this problem with a description of your system."),
        };

        std::fs::create_dir_all(dirs.config_dir()).with_context(|| {
            format!("Unable to create the directory \"{:?}\" Make sure the application are allowed to write here. If you think this location is bad please report it.",
                dirs.config_dir())
        })?;

        std::fs::create_dir_all(dirs.config_dir()).with_context(|| format!("test"))?;

        let log_file_path = Path::new(dirs.config_dir()).join("retrovert.log");

        let log_file = File::create(&log_file_path).with_context(|| {
            format!("Unable to create file \"{:?}\" Make sure the application has access to this location or report this problem if you think the location is bad",
                log_file_path)
        })?;

        CombinedLogger::init(vec![
            TermLogger::new(
                LevelFilter::Warn,
                Config::default(),
                TerminalMode::Mixed,
                ColorChoice::Auto,
            ),
            WriteLogger::new(LevelFilter::Info, Config::default(), log_file),
        ])?;

        Ok(())
    }

    pub fn init_data_directory() -> Result<()> {
        let current_exe = std::env::current_exe()?;
        std::env::set_current_dir(
            current_exe
                .parent()
                .with_context(|| "Unable to get parent directory")?,
        )?;

        find_data_directory().with_context(|| "Unable to find data directory")?;

        // TODO: We should do better error handling here
        // This to enforce we load relative to the current exe
        let current_exe = std::env::current_exe()?;
        std::env::set_current_dir(
            current_exe
                .parent()
                .with_context(|| "Unable to get parent directory")?,
        )?;

        Ok(())
    }

    pub fn load_core() -> Result<Library> {
        let core_filename = "../../../retrovert-core/target/debug/librv_core.so";
        let lib = unsafe { Library::new(core_filename)? };
        Ok(lib)
    }

    pub fn new(lib: &'a Library) -> Result<Core<'a>> {
        unsafe {
            let ret = lib.get::<SetupLogger>(b"core_setup_logger");
            if let Ok(setup_logger) = ret {
                setup_logger(log::logger(), log::max_level()).unwrap();
            }

            let core_create_func: Symbol<CoreCreate> = lib
                .get(b"core_create\0")
                .context("Unable to find \"core_create\" function")?;
            let core_destroy_func: Symbol<CoreDestroy> = lib
                .get(b"core_destroy\0")
                .context("Unable to find \"core_destroy\" function")?;
            let core_update_func: Symbol<CoreUpdate> = lib
                .get(b"core_update\0")
                .context("Unable to find \"core_update\" function")?;

            Ok(Core {
                core_create_func,
                core_destroy_func,
                core_update_func,
            })
        }
    }
}
