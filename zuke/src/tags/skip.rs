//! Fixture to implement "skip" tags

use async_trait::async_trait;
use lazy_static::lazy_static;
use regex::Regex;
use zuke::{Context, Fixture, Scope};

/// A fixture that implements `@skip` tags
pub struct Skip;

macro_rules! push_cfg_pattern {
    ($dst:ident, $($x:ident,)*) => {
        $(
            if cfg!($x) {
                $dst.push_str(concat!(r#"|^skip-if-"#, stringify!($x), "$"));
            } else {
                $dst.push_str(concat!(r#"|^skip-if-not-"#, stringify!($x), "$"));
            }
        )*
    };
    ($dst:ident, $($k:ident = $v:literal,)*) => {
        $(
            if cfg!($k = $v) {
                $dst.push_str(concat!(r#"|^skip-if-"#, stringify!($k), "-", $v, "$"));
            } else {
                $dst.push_str(concat!(r#"|^skip-if-not-"#, stringify!($k), "-", $v, "$"));
            }
        )*
    };
}

lazy_static! {
    static ref SKIP_REGEX: Regex = {
        let mut pattern = String::from(r#"^skip$"#);
        // definitely not a complete list. Add items as we need to
        push_cfg_pattern!(
            pattern,
            unix,
            windows,
            test,
            debug_assertions,
            proc_macro,
        );
        push_cfg_pattern!(
            pattern,
            target_arch = "x86",
            target_arch = "x86_64",
            target_arch = "mips",
            target_arch = "powerpc",
            target_arch = "powerpc64",
            target_arch = "arm",
            target_arch = "aarch64",
            target_arch = "riscv64",
            target_arch = "riscv32",
            target_arch = "s390x",
            target_os = "windows",
            target_os = "macos",
            target_os = "linux",
            target_os = "ios",
            target_os = "android",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "vxworks",
            target_os = "uefi",
            target_family = "unix",
            target_family = "windows",
            target_family = "wasm",
            target_env = "",
            target_env = "gnu",
            target_env = "msvc",
            target_env = "musl",
            target_env = "sgx",
            target_endian = "little",
            target_endian = "big",
            target_pointer_width = "16",
            target_pointer_width = "32",
            target_pointer_width = "64",
            target_vendor = "apple",
            target_vendor = "pc",
            target_vendor = "unknown",
        );
        Regex::new(&pattern).unwrap()
    };
}

#[async_trait]
impl Fixture for Skip {
    const SCOPE: Scope = Scope::Global;

    async fn setup(_context: &mut Context) -> anyhow::Result<Self> {
        Ok(Self)
    }

    async fn before(&self, context: &mut Context) -> anyhow::Result<()> {
        let component = context.component();
        if component.step().is_some() {
            return Ok(());
        }

        if context.tags().any(|t| SKIP_REGEX.is_match(t)) {
            zuke::skip!();
        }

        Ok(())
    }
}
