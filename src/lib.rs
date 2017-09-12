// Copyright (c) 2017, Contributors (see CONTRIBUTORS file).
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

#![feature(untagged_unions)] // for clib
#![cfg_attr(test, feature(try_from))]
#![cfg_attr(test, feature(test))]

#[cfg(test)]
extern crate test;

use std::ptr::null_mut;

#[allow(dead_code,non_camel_case_types,non_snake_case)]
mod clib;

pub mod nvme;

use self::clib::*;
use std::marker::PhantomData;

#[derive(Debug)]
pub struct DMA<'a>(*mut ::std::os::raw::c_void, usize, PhantomData<&'a ()>);

impl<'a> DMA<'a> {
    pub fn alloc(size: usize, align: usize) -> Self {
        DMA(unsafe { spdk_dma_malloc(size, align, null_mut()) }, size, PhantomData)
    }

    pub fn alloc_zeroed(size: usize, align: usize) -> Self {
        DMA(unsafe { spdk_dma_zmalloc(size, align, null_mut()) }, size, PhantomData)
    }

    pub fn as_slice(&self) -> &'a [u8] {
        unsafe { ::std::slice::from_raw_parts(self.0 as *mut _ as *const u8, self.1) }
    }

    pub fn as_slice_mut(&self) -> &'a mut [u8] {
        unsafe { ::std::slice::from_raw_parts_mut(self.0 as *mut _ as *mut u8, self.1) }
    }

}

impl<'a> Drop for DMA<'a> {
    fn drop(&mut self) {
        unsafe { spdk_dma_free(self.0) }
    }
}

pub struct EnvOpts(spdk_env_opts);

impl EnvOpts {
    pub fn new() -> Self {
        let mut opts: spdk_env_opts = Default::default();
        unsafe {
            spdk_env_opts_init(&mut opts as *mut spdk_env_opts);
        }
        EnvOpts(opts)
    }
}

pub fn init_env(opts: &EnvOpts) {
    unsafe {
        spdk_env_init(&opts.0 as *const spdk_env_opts);
    }
}
