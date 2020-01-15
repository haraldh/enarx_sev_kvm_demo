#[no_mangle]
pub extern "C" fn strlen(ptr: *const i8) -> usize {
    let mut i = ptr;
    loop {
        unsafe {
            if i.read() == 0 {
                return i.sub(ptr as _) as usize;
            }
            i = i.add(1);
        }
    }
}
