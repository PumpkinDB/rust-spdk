// Copyright (c) 2017, Contributors (see CONTRIBUTORS file).
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

use std;

use std::ffi::{CStr, CString};
use std::ptr::null;

use super::clib::*;

pub const IO_FLAG_PRCHK_REF_TAG: u32 = 1u32 << 26;
pub const IO_FLAG_PRCHK_APP_TAG: u32 = 1u32 << 27;
pub const IO_FLAG_PRCHK_GUARD: u32 = 1u32 << 28;
pub const IO_FLAG_PRACT: u32 = 1u32 << 29;
pub const IO_FLAG_FORCE_UNIT_ACCESS: u32 = 1u32 << 30;
pub const IO_FLAG_LIMITED_RETRY: u32 = 1u32 << 31;

#[derive(Debug)]
pub struct CompletionQueueEntry(*const spdk_nvme_cpl);

pub trait CommandCallback {
    fn callback(&mut self, cpl: CompletionQueueEntry);
}

pub use super::clib::spdk_nvme_transport_type as TransportType;

#[derive(Debug)]
pub struct TransportIdentifier(*const spdk_nvme_transport_id);

pub struct OwnedTransportIdentifier(spdk_nvme_transport_id);

impl OwnedTransportIdentifier {
    pub fn from_str(str: &str) -> Result<Self, ()> {
        let mut tid : spdk_nvme_transport_id = Default::default();
        let cstr = CString::new(str).unwrap();
        unsafe {
            if spdk_nvme_transport_id_parse(&mut tid as *mut spdk_nvme_transport_id, cstr.as_ptr()) == 0 {
                Ok(OwnedTransportIdentifier(tid))
            } else {
                Err(())
            }
        }
    }
}

impl TransportIdentifier {
    pub fn transport_type(&self) -> TransportType {
        unsafe { (*self.0).trtype }
    }
    pub fn address(&self) -> &str {
        unsafe { CStr::from_ptr(&(*self.0).traddr as *const i8).to_str().unwrap() }
    }
}

pub trait UnderlyingTransportIdentifier {
    fn transport_id(&self) -> *const spdk_nvme_transport_id;
}

impl<'a> UnderlyingTransportIdentifier for &'a OwnedTransportIdentifier {
    fn transport_id(&self) -> *const spdk_nvme_transport_id {
        &self.0 as *const spdk_nvme_transport_id
    }
}

impl UnderlyingTransportIdentifier for TransportIdentifier {
    fn transport_id(&self) -> *const spdk_nvme_transport_id {
        self.0
    }
}

impl UnderlyingTransportIdentifier for () {
    fn transport_id(&self) -> *const spdk_nvme_transport_id {
        null()
    }
}

#[derive(Debug)]
pub struct ControllerOptions(*const spdk_nvme_ctrlr_opts);
#[derive(Debug)]
pub struct ControllerMutableOptions(*mut spdk_nvme_ctrlr_opts);

#[derive(Debug)]
pub struct QueuePair(*mut spdk_nvme_qpair);

impl QueuePair {
    #[inline]
    pub fn process_completions(&self, max: u32) -> i32 {
        unsafe {
            spdk_nvme_qpair_process_completions(self.0, max)
        }
    }
}

impl Drop for QueuePair {
    fn drop(&mut self) {
        unsafe { spdk_nvme_ctrlr_free_io_qpair(self.0); }
    }
}


#[derive(Debug)]
pub struct Namespace(*mut spdk_nvme_ns);

macro_rules! ns_data {
    ($name: ident, $field: ident) => {
      pub fn $name(&self) -> u16 {
        unsafe {
            let data = spdk_nvme_ns_get_data(self.0);
            (*data).$field
        }
      }
    };
}

impl Namespace {
    pub fn is_active(&self) -> bool {
        unsafe { spdk_nvme_ns_is_active(self.0) }
    }

    pub fn id(&self) -> u32 {
        unsafe { spdk_nvme_ns_get_id(self.0) }
    }

    pub fn size(&self) -> u64 {
        unsafe { spdk_nvme_ns_get_size(self.0) }
    }

    pub fn sector_size(&self) -> u32 {
        unsafe { spdk_nvme_ns_get_sector_size(self.0) }
    }

    ns_data!(atomic_write_unit_normal, nawun);
    ns_data!(atomic_write_unit_power_failure, nawupf);
    ns_data!(atomic_boundary_size_normal, nabsn);

    unsafe extern "C" fn cmd_cb<P: CommandCallback>(cb_ctx: *mut ::std::os::raw::c_void,
                                                    cpl: *const spdk_nvme_cpl) {
        let cb = cb_ctx as *mut _ as *mut P;
        (&mut *cb).callback(CompletionQueueEntry(cpl));
        drop(cb);
    }

    pub fn write<C : CommandCallback>(&self, qpair: &QueuePair, data: &[u8],
                                      lba: u64, lba_count: u32, callback: C,
                                      flags: u32) -> Result<(), ()> {
        let cb = Box::new(callback);
        let code =
        unsafe { spdk_nvme_ns_cmd_write(self.0, qpair.0, data as *const _ as *mut ::std::os::raw::c_void,
                                        lba, lba_count,
                                        Some(Namespace::cmd_cb::<C>),
                                        &*cb as *const _ as *mut ::std::os::raw::c_void,
                                        flags) };
        // crossing the FFI boundary
        std::mem::forget(cb);
        if code == 0 {
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn read<C : CommandCallback>(&self, qpair: &QueuePair, data: &mut [u8],
                                     lba: u64, lba_count: u32, callback: C,
                                     flags: u32) {
        let cb = Box::new(callback);
        unsafe { spdk_nvme_ns_cmd_read(self.0, qpair.0, data as *const _ as *mut ::std::os::raw::c_void,
                                       lba, lba_count,
                                       Some(Namespace::cmd_cb::<C>),
                                       &*cb as *const _ as *mut ::std::os::raw::c_void,
                                       flags); }
        // crossing the FFI boundary
        std::mem::forget(cb);
    }
}

#[derive(Debug)]
pub struct Controller(*mut spdk_nvme_ctrlr);

pub use super::clib::spdk_nvme_qprio as QueuePriority;

macro_rules! ctrlr_data {
    ($name: ident, $field: ident) => {
      pub fn $name(&self) -> u16 {
        unsafe {
            let data = spdk_nvme_ctrlr_get_data(self.0);
            (*data).$field
        }
      }
    };
}

impl Controller {
    pub fn alloc_io_queue_pair(&self, qprio: QueuePriority) -> Result<QueuePair, ()> {
        let qpair = unsafe { spdk_nvme_ctrlr_alloc_io_qpair(self.0, qprio) };
        if qpair.is_null() {
            Err(())
        } else {
            Ok(QueuePair(qpair))
        }
    }

    pub fn namespaces(&self) -> Vec<Namespace> {
        let num = unsafe { spdk_nvme_ctrlr_get_num_ns(self.0) };
        let mut result = Vec::with_capacity(num as usize);
        for i in 0..num {
            result.push(Namespace(unsafe { spdk_nvme_ctrlr_get_ns(self.0, i + 1) }));
        }
        result
    }

    pub fn detach(self) {
        unsafe { spdk_nvme_detach(self.0); }
    }


    ctrlr_data!(pci_vendor_id, vid);
    ctrlr_data!(atomic_write_unit_normal, awun);
}

#[allow(unused_variables)]
pub trait ProbeCallback {
    fn probe(&mut self, transport_id: TransportIdentifier, opts: ControllerMutableOptions) -> bool;
    fn attach(&mut self, transport_id: TransportIdentifier, ctrlr: Controller, opts: ControllerOptions) {}
}


pub fn probe<P: ProbeCallback, T: UnderlyingTransportIdentifier>(transport_id: T, probe: P) -> Result<(), ()> {

    unsafe extern "C" fn probe_cb<P: ProbeCallback>(cb_ctx: *mut ::std::os::raw::c_void,
                                                    trid: *const spdk_nvme_transport_id,
                                                    opts: *mut spdk_nvme_ctrlr_opts)
                                                    -> bool {
        let cb = cb_ctx as *mut _ as *mut P;
        (&mut *cb).probe(TransportIdentifier(trid), ControllerMutableOptions(opts))
    }

    unsafe extern "C" fn attach_cb<P: ProbeCallback>(cb_ctx: *mut ::std::os::raw::c_void,
                                                     trid: *const spdk_nvme_transport_id,
                                                     ctrlr: *mut spdk_nvme_ctrlr,
                                                     opts: *const spdk_nvme_ctrlr_opts) {
        let cb = cb_ctx as *mut _ as *mut P;
        (&mut *cb).attach(TransportIdentifier(trid), Controller(ctrlr), ControllerOptions(opts))
    }

    unsafe {
        if spdk_nvme_probe(transport_id.transport_id(),
                           &probe as *const _ as *mut ::std::os::raw::c_void,
                           Some(probe_cb::<P>),
                           Some(attach_cb::<P>),
                           None) == 0 {
            Ok(())
        } else {
            Err(())
        }
    }
}



#[cfg(test)]
#[allow(unused_variables)]
mod tests {

    use std::convert::{TryFrom, TryInto};
    use super::*;
    use super::super::*;

    impl<'a> CommandCallback for &'a std::sync::atomic::AtomicUsize {
        fn callback(&mut self, cpl: CompletionQueueEntry) {
            self.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    struct Uninitialized {
        ctrlr: Option<Controller>
    }

    struct Initialized(Controller);

    use test::Bencher;

    impl Initialized {
        pub fn test(&self, b: &mut Bencher) {
            use std::rc::Rc;
            let ref ctrlr = self.0;
            let qpair = Rc::new(ctrlr.alloc_io_queue_pair(QueuePriority::SPDK_NVME_QPRIO_URGENT).unwrap());
            let buf = Rc::new(Buffer::alloc_zeroed(4096 * 1024 * 2, 512));
            (buf.as_slice_mut()[0..4]).copy_from_slice("test".as_bytes());
            let rbuf = Rc::new(Buffer::alloc_zeroed(4096 * 1024 * 2, 512));

            let namespaces = ctrlr.namespaces();

            for ns in namespaces {
                let buf_ = buf.clone();
                let rbuf_ = rbuf.clone();
                let qpair_ = qpair.clone();
                let slice = buf_.as_slice();
                let mut rslice = rbuf_.as_slice_mut();

                let sector_sz = ns.sector_size();
                let lba_count = slice.len() as u32 / sector_sz;
                println!("sector_sz {} lba count {}", sector_sz, lba_count);

                b.iter(move || {
                    let mut ctr = std::sync::atomic::AtomicUsize::new(0);
                    ns.write(&qpair_, slice,
                             0, lba_count,
                             &ctr, 0).expect("start write did not succeed");
                    ns.read(&qpair_, &mut rslice,
                             0, lba_count,
                             &ctr, 0);
                    while *ctr.get_mut() < 2 {
                        qpair_.process_completions(0);
                    }
                });
            }
        }
        pub fn cleanup(self) {
            self.0.detach()
        }

    }

    impl Uninitialized {
        pub fn new() -> Self {
            Uninitialized { ctrlr: None }
        }
    }

    impl TryFrom<Uninitialized> for Initialized {
        type Error = ();

        fn try_from(value: Uninitialized) -> Result<Self, Self::Error> {
            match value.ctrlr {
                Some(c) => Ok(Initialized(c)),
                None => Err(())
            }
        }

    }


    impl<'a> ProbeCallback for &'a mut Uninitialized {
        fn probe(&mut self, transport_id: TransportIdentifier, opts: ControllerMutableOptions) -> bool {
            println!("Attaching {:?} {}",
                     transport_id.transport_type(),
                     transport_id.address());
            true
        }

        fn attach(&mut self, transport_id: TransportIdentifier, ctrlr: Controller, opts: ControllerOptions) {
            println!("Attached {:?}", ctrlr);
            self.ctrlr = Some(ctrlr);
        }
    }



    #[bench]
    fn it_works(b: &mut Bencher) {
        use std::env;
        match env::var("nvme_pci") {
            Ok(addr) => {
                let opts = EnvOpts::new();
                init_env(&opts);
                let mut t = Uninitialized::new();
                println!("Probing...");
                let tr = OwnedTransportIdentifier::from_str(format!("trtype:PCIe traddr:{}", addr).as_str())
                         .expect("can't parse PCIe address");
                probe(&tr, &mut t).expect("failed probing NVMe controllers");
                let i : Initialized = t.try_into().expect("can't attach an NVMe controller");
                println!("Initialized.");
                i.test(b);
                i.cleanup();
            },
            Err(_) => println!("skipping the test")
        }
    }
}
