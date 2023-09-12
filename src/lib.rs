//  Copyright (C) 2023  Ganlv
//
//  This program is free software; you can redistribute it and/or modify
//  it under the terms of the GNU General Public License as published by
//  the Free Software Foundation; either version 2 of the License, or
//  (at your option) any later version.
//
//  This program is distributed in the hope that it will be useful,
//  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//  GNU General Public License for more details.
//
//  You should have received a copy of the GNU General Public License along
//  with this program; if not, write to the Free Software Foundation, Inc.,
//  51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.

mod video_tile_remap;

use std::ptr::null_mut;
use bindings::*;

pub static mut OBS_MODULE_POINTER: *mut obs_module_t = null_mut();

#[no_mangle]
pub unsafe extern "C" fn obs_module_set_pointer(module: *mut obs_module_t) {
    OBS_MODULE_POINTER = module;
}

#[no_mangle]
pub unsafe extern "C" fn obs_current_module() -> *mut obs_module_t {
    OBS_MODULE_POINTER
}

#[no_mangle]
pub unsafe extern "C" fn obs_module_ver() -> u32 {
    LIBOBS_API_MAJOR_VER
}

#[no_mangle]
pub unsafe extern "C" fn obs_module_load() -> bool {
    video_tile_remap::register();
    true
}
