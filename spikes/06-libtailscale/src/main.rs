// EXP-0006: call four libtailscale entry points without touching a network.
use std::ffi::{c_char, c_int, CStr, CString};

extern "C" {
    fn tailscale_new() -> c_int;
    fn tailscale_set_dir(sd: c_int, dir: *const c_char) -> c_int;
    fn tailscale_errmsg(sd: c_int, buf: *mut c_char, buflen: usize) -> c_int;
    fn tailscale_close(sd: c_int) -> c_int;
}

fn errmsg(sd: c_int) -> (c_int, String) {
    let mut buf = [0 as c_char; 256];
    let rc = unsafe { tailscale_errmsg(sd, buf.as_mut_ptr(), buf.len()) };
    let msg = unsafe { CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into_owned();
    (rc, msg)
}

fn main() {
    let dir = std::env::temp_dir().join(format!("exp0006-ts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let cdir = CString::new(dir.to_str().unwrap()).unwrap();

    let sd = unsafe { tailscale_new() };
    let set_dir = unsafe { tailscale_set_dir(sd, cdir.as_ptr()) };
    let (errmsg_rc, msg) = errmsg(sd);
    let close = unsafe { tailscale_close(sd) };
    // After close the handle is gone: errmsg must report EBADF.
    let (errmsg_after_close, _) = errmsg(sd);

    let _ = std::fs::remove_dir_all(&dir);

    println!(
        "===RESULT=== {{\"sd\": {sd}, \"set_dir\": {set_dir}, \"errmsg_rc\": {errmsg_rc}, \
         \"errmsg\": {msg:?}, \"close\": {close}, \"errmsg_after_close\": {errmsg_after_close}, \
         \"target\": \"{}\"}}",
        std::env::consts::ARCH.to_string() + "-" + std::env::consts::OS
    );

    let ok = set_dir == 0 && errmsg_rc == 0 && msg.is_empty() && close == 0 && errmsg_after_close != 0;
    std::process::exit(if ok { 0 } else { 1 });
}
