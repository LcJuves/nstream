#[cfg(target_os = "macos")]
pub mod utun;

#[cfg(target_os = "macos")]
pub use utun::UTun;

mod vtun;
pub use vtun::*;

pub fn ifname(fd: core::ffi::c_int) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        extern "C" {
            fn utun_ifname(name: *mut core::ffi::c_char, fd: core::ffi::c_int) -> core::ffi::c_int;
        }

        let mut utunname = unsafe { core::mem::zeroed::<[core::ffi::c_char; libc::IFNAMSIZ]>() };
        seeval!(utunname);

        unsafe {
            if utun_ifname(utunname.as_mut_ptr(), fd) != 0 {
                return None;
            }
        }
        seeval!(utunname);

        let nstr =
            unsafe { std::ffi::CStr::from_ptr(utunname.as_ptr()) }.to_string_lossy().to_string();
        seeval!(nstr);
        return if nstr.is_empty() { None } else { Some(nstr) };
    }

    #[allow(unreachable_code)]
    None
}

#[macro_export(local_inner_macros)]
macro_rules! debug_print {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        std::print!($($arg)*);
    }
}

#[macro_export(local_inner_macros)]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        std::println!($($arg)*);
    }
}

#[macro_export(local_inner_macros)]
macro_rules! seeval {
    ($val:expr) => {
        debug_println!(
            "[{}:{}] {} >>> {:?}",
            core::file!(),
            core::line!(),
            core::stringify!($val),
            $val
        );
    };
}

#[cfg(test)]
mod tests {}
