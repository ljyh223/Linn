//! Shared glow loader for GTK `GLArea` renderers.

pub(crate) fn create_glow_context() -> Result<glow::Context, String> {
    unsafe {
        type GetProcAddress =
            unsafe extern "C" fn(*const std::ffi::c_char) -> *mut std::ffi::c_void;
        // GTK uses libepoxy on Linux. Prefer its dispatcher because it selects
        // EGL or GLX for the GLArea's actual backend, then retain EGL as a
        // fallback for minimal Wayland builds where the epoxy symbol is not
        // exported globally.
        let epoxy = libc::dlsym(
            libc::RTLD_DEFAULT,
            b"epoxy_get_proc_address\0".as_ptr() as *const std::ffi::c_char,
        );
        let egl = libc::dlsym(
            libc::RTLD_DEFAULT,
            b"eglGetProcAddress\0".as_ptr() as *const std::ffi::c_char,
        );
        let get_proc_address = if !epoxy.is_null() {
            std::mem::transmute::<*mut std::ffi::c_void, GetProcAddress>(epoxy)
        } else if !egl.is_null() {
            std::mem::transmute::<*mut std::ffi::c_void, GetProcAddress>(egl)
        } else {
            return Err("neither epoxy nor EGL can resolve OpenGL functions".to_string());
        };
        let loader = move |name: &str| -> *const std::ffi::c_void {
            let name = match std::ffi::CString::new(name) {
                Ok(name) => name,
                Err(_) => return std::ptr::null(),
            };
            get_proc_address(name.as_ptr()) as *const std::ffi::c_void
        };
        Ok(glow::Context::from_loader_function(loader))
    }
}
