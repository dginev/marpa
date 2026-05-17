#![cfg(test)]

use crate::{MARPA_ERR_NONE, Marpa_Config, marpa_c_error, marpa_c_init, marpa_g_new};

use std::ptr;

#[test]
fn create_grammar() {
    unsafe {
        let mut a: Marpa_Config = std::mem::zeroed();
        let b = marpa_c_init(&mut a);
        let _ = marpa_g_new(&mut a);
        assert!(b as u32 == MARPA_ERR_NONE);
        assert!(marpa_c_error(&mut a, ptr::null_mut()) as u32 == MARPA_ERR_NONE);
    }
}
