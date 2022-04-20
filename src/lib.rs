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

#[allow(dead_code)]
pub type CoreShowArgs = fn();

pub struct Core<'a> {
    pub core_create_func: Symbol<'a, CoreCreate>,
    pub core_destroy_func: Symbol<'a, CoreDestroy>,
    pub core_update_func: Symbol<'a, CoreUpdate>,
    pub core_show_args: Symbol<'a, CoreShowArgs>,
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

        std::fs::create_dir_all(dirs.config_dir())
            .with_context(|| format!("unable to create all needed directories"))?;

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

    pub fn load_core(core_filename: &Option<String>) -> Result<Library> {
        let filename = if let Some(core_filename) = core_filename {
            core_filename
        } else {
            "../retrovert-core/target/debug/librv_core.so"
        };

        let lib = unsafe { Library::new(filename)? };
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
            let core_show_args: Symbol<CoreShowArgs> = lib
                .get(b"core_show_args\0")
                .context("Unable to find \"core_show_args\" function")?;

            Ok(Core {
                core_create_func,
                core_destroy_func,
                core_update_func,
                core_show_args,
            })
        }
    }
}
