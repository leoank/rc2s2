//! EXP-0009: read the GPU `PerformanceStatistics` dictionary of every
//! `IOAccelerator` service through IOKit, no sudo, no vendor API.
//! Prints utilisation and memory in use, and the cost of one read.

use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use io_kit_sys::types::{io_iterator_t, io_object_t};
use io_kit_sys::{
    kIOMasterPortDefault, IOIteratorNext, IOObjectRelease, IORegistryEntryCreateCFProperty,
    IOServiceGetMatchingServices, IOServiceMatching,
};
use std::time::Instant;

const KEYS: [&str; 5] = [
    "Device Utilization %",
    "Renderer Utilization %",
    "Tiler Utilization %",
    "In use system memory",
    "Alloc system memory",
];

/// One sample: (key, value) for every IOAccelerator service found.
fn sample() -> Vec<(String, i64)> {
    let mut out = Vec::new();
    unsafe {
        let matching = IOServiceMatching(c"IOAccelerator".as_ptr());
        let mut iter: io_iterator_t = 0;
        let kr = IOServiceGetMatchingServices(kIOMasterPortDefault, matching, &mut iter);
        assert_eq!(kr, 0, "IOServiceGetMatchingServices failed: {kr}");
        loop {
            let svc: io_object_t = IOIteratorNext(iter);
            if svc == 0 {
                break;
            }
            let key = CFString::from_static_string("PerformanceStatistics");
            let raw = IORegistryEntryCreateCFProperty(
                svc,
                key.as_concrete_TypeRef(),
                core_foundation::base::kCFAllocatorDefault,
                0,
            );
            if !raw.is_null() {
                let stats: CFDictionary<CFString, CFType> =
                    CFDictionary::wrap_under_create_rule(raw as _);
                for k in KEYS {
                    if let Some(v) = stats.find(CFString::from_static_string(k)) {
                        if let Some(n) = v.downcast::<CFNumber>().and_then(|n| n.to_i64()) {
                            out.push((k.to_string(), n));
                        }
                    }
                }
            }
            IOObjectRelease(svc);
        }
        IOObjectRelease(iter);
    }
    out
}

fn main() {
    let t = Instant::now();
    let s = sample();
    let first = t.elapsed();
    for (k, v) in &s {
        println!("{k:<26} {v}");
    }
    let n = 200;
    let t = Instant::now();
    for _ in 0..n {
        std::hint::black_box(sample());
    }
    let mean = t.elapsed() / n;
    println!("first read: {first:?}; mean of {n}: {mean:?}");
}
