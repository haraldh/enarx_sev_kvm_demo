#![allow(dead_code)]
#![allow(non_upper_case_globals)]

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct __BindgenBitfieldUnit<Storage, Align>
where
    Storage: AsRef<[u8]> + AsMut<[u8]>,
{
    storage: Storage,
    align: [Align; 0],
}
impl<Storage, Align> __BindgenBitfieldUnit<Storage, Align>
where
    Storage: AsRef<[u8]> + AsMut<[u8]>,
{
    #[inline]
    pub fn new(storage: Storage) -> Self {
        Self { storage, align: [] }
    }
    #[inline]
    pub fn get_bit(&self, index: usize) -> bool {
        debug_assert!(index / 8 < self.storage.as_ref().len());
        let byte_index = index / 8;
        let byte = self.storage.as_ref()[byte_index];
        let bit_index = if cfg!(target_endian = "big") {
            7 - (index % 8)
        } else {
            index % 8
        };
        let mask = 1 << bit_index;
        byte & mask == mask
    }
    #[inline]
    pub fn set_bit(&mut self, index: usize, val: bool) {
        debug_assert!(index / 8 < self.storage.as_ref().len());
        let byte_index = index / 8;
        let byte = &mut self.storage.as_mut()[byte_index];
        let bit_index = if cfg!(target_endian = "big") {
            7 - (index % 8)
        } else {
            index % 8
        };
        let mask = 1 << bit_index;
        if val {
            *byte |= mask;
        } else {
            *byte &= !mask;
        }
    }
    #[inline]
    pub fn get(&self, bit_offset: usize, bit_width: u8) -> u64 {
        debug_assert!(bit_width <= 64);
        debug_assert!(bit_offset / 8 < self.storage.as_ref().len());
        debug_assert!((bit_offset + (bit_width as usize)) / 8 <= self.storage.as_ref().len());
        let mut val = 0;
        for i in 0..(bit_width as usize) {
            if self.get_bit(i + bit_offset) {
                let index = if cfg!(target_endian = "big") {
                    bit_width as usize - 1 - i
                } else {
                    i
                };
                val |= 1 << index;
            }
        }
        val
    }
    #[inline]
    pub fn set(&mut self, bit_offset: usize, bit_width: u8, val: u64) {
        debug_assert!(bit_width <= 64);
        debug_assert!(bit_offset / 8 < self.storage.as_ref().len());
        debug_assert!((bit_offset + (bit_width as usize)) / 8 <= self.storage.as_ref().len());
        for i in 0..(bit_width as usize) {
            let mask = 1 << i;
            let val_bit_is_set = val & mask == mask;
            let index = if cfg!(target_endian = "big") {
                bit_width as usize - 1 - i
            } else {
                i
            };
            self.set_bit(index + bit_offset, val_bit_is_set);
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct desc64 {
    pub limit0: u16,
    pub base0: u16,
    pub _bitfield_1: __BindgenBitfieldUnit<[u8; 4usize], u8>,
    pub base3: u32,
    pub zero1: u32,
}
#[test]
fn bindgen_test_layout_desc64() {
    assert_eq!(
        ::std::mem::size_of::<desc64>(),
        16usize,
        concat!("Size of: ", stringify!(desc64))
    );
    assert_eq!(
        ::std::mem::align_of::<desc64>(),
        1usize,
        concat!("Alignment of ", stringify!(desc64))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<desc64>())).limit0 as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(desc64),
            "::",
            stringify!(limit0)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<desc64>())).base0 as *const _ as usize },
        2usize,
        concat!(
            "Offset of field: ",
            stringify!(desc64),
            "::",
            stringify!(base0)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<desc64>())).base3 as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(desc64),
            "::",
            stringify!(base3)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<desc64>())).zero1 as *const _ as usize },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(desc64),
            "::",
            stringify!(zero1)
        )
    );
}
impl desc64 {
    #[inline]
    pub fn base1(&self) -> ::std::os::raw::c_uint {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(0usize, 8u8) as u32) }
    }
    #[inline]
    pub fn set_base1(&mut self, val: ::std::os::raw::c_uint) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(0usize, 8u8, val as u64)
        }
    }
    #[inline]
    pub fn s(&self) -> ::std::os::raw::c_uint {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(8usize, 1u8) as u32) }
    }
    #[inline]
    pub fn set_s(&mut self, val: ::std::os::raw::c_uint) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(8usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn type_(&self) -> ::std::os::raw::c_uint {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(9usize, 4u8) as u32) }
    }
    #[inline]
    pub fn set_type(&mut self, val: ::std::os::raw::c_uint) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(9usize, 4u8, val as u64)
        }
    }
    #[inline]
    pub fn dpl(&self) -> ::std::os::raw::c_uint {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(13usize, 2u8) as u32) }
    }
    #[inline]
    pub fn set_dpl(&mut self, val: ::std::os::raw::c_uint) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(13usize, 2u8, val as u64)
        }
    }
    #[inline]
    pub fn p(&self) -> ::std::os::raw::c_uint {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(15usize, 1u8) as u32) }
    }
    #[inline]
    pub fn set_p(&mut self, val: ::std::os::raw::c_uint) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(15usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn limit1(&self) -> ::std::os::raw::c_uint {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(16usize, 4u8) as u32) }
    }
    #[inline]
    pub fn set_limit1(&mut self, val: ::std::os::raw::c_uint) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(16usize, 4u8, val as u64)
        }
    }
    #[inline]
    pub fn avl(&self) -> ::std::os::raw::c_uint {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(20usize, 1u8) as u32) }
    }
    #[inline]
    pub fn set_avl(&mut self, val: ::std::os::raw::c_uint) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(20usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn l(&self) -> ::std::os::raw::c_uint {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(21usize, 1u8) as u32) }
    }
    #[inline]
    pub fn set_l(&mut self, val: ::std::os::raw::c_uint) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(21usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn db(&self) -> ::std::os::raw::c_uint {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(22usize, 1u8) as u32) }
    }
    #[inline]
    pub fn set_db(&mut self, val: ::std::os::raw::c_uint) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(22usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn g(&self) -> ::std::os::raw::c_uint {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(23usize, 1u8) as u32) }
    }
    #[inline]
    pub fn set_g(&mut self, val: ::std::os::raw::c_uint) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(23usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn base2(&self) -> ::std::os::raw::c_uint {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(24usize, 8u8) as u32) }
    }
    #[inline]
    pub fn set_base2(&mut self, val: ::std::os::raw::c_uint) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(24usize, 8u8, val as u64)
        }
    }
    #[inline]
    pub fn new_bitfield_1(
        base1: ::std::os::raw::c_uint,
        s: ::std::os::raw::c_uint,
        type_: ::std::os::raw::c_uint,
        dpl: ::std::os::raw::c_uint,
        p: ::std::os::raw::c_uint,
        limit1: ::std::os::raw::c_uint,
        avl: ::std::os::raw::c_uint,
        l: ::std::os::raw::c_uint,
        db: ::std::os::raw::c_uint,
        g: ::std::os::raw::c_uint,
        base2: ::std::os::raw::c_uint,
    ) -> __BindgenBitfieldUnit<[u8; 4usize], u8> {
        let mut __bindgen_bitfield_unit: __BindgenBitfieldUnit<[u8; 4usize], u8> =
            Default::default();
        __bindgen_bitfield_unit.set(0usize, 8u8, {
            let base1: u32 = unsafe { ::std::mem::transmute(base1) };
            base1 as u64
        });
        __bindgen_bitfield_unit.set(8usize, 1u8, {
            let s: u32 = unsafe { ::std::mem::transmute(s) };
            s as u64
        });
        __bindgen_bitfield_unit.set(9usize, 4u8, {
            let type_: u32 = unsafe { ::std::mem::transmute(type_) };
            type_ as u64
        });
        __bindgen_bitfield_unit.set(13usize, 2u8, {
            let dpl: u32 = unsafe { ::std::mem::transmute(dpl) };
            dpl as u64
        });
        __bindgen_bitfield_unit.set(15usize, 1u8, {
            let p: u32 = unsafe { ::std::mem::transmute(p) };
            p as u64
        });
        __bindgen_bitfield_unit.set(16usize, 4u8, {
            let limit1: u32 = unsafe { ::std::mem::transmute(limit1) };
            limit1 as u64
        });
        __bindgen_bitfield_unit.set(20usize, 1u8, {
            let avl: u32 = unsafe { ::std::mem::transmute(avl) };
            avl as u64
        });
        __bindgen_bitfield_unit.set(21usize, 1u8, {
            let l: u32 = unsafe { ::std::mem::transmute(l) };
            l as u64
        });
        __bindgen_bitfield_unit.set(22usize, 1u8, {
            let db: u32 = unsafe { ::std::mem::transmute(db) };
            db as u64
        });
        __bindgen_bitfield_unit.set(23usize, 1u8, {
            let g: u32 = unsafe { ::std::mem::transmute(g) };
            g as u64
        });
        __bindgen_bitfield_unit.set(24usize, 8u8, {
            let base2: u32 = unsafe { ::std::mem::transmute(base2) };
            base2 as u64
        });
        __bindgen_bitfield_unit
    }
}
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct desc_ptr {
    pub size: u16,
    pub address: u64,
}
#[test]
fn bindgen_test_layout_desc_ptr() {
    assert_eq!(
        ::std::mem::size_of::<desc_ptr>(),
        10usize,
        concat!("Size of: ", stringify!(desc_ptr))
    );
    assert_eq!(
        ::std::mem::align_of::<desc_ptr>(),
        1usize,
        concat!("Alignment of ", stringify!(desc_ptr))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<desc_ptr>())).size as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(desc_ptr),
            "::",
            stringify!(size)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<desc_ptr>())).address as *const _ as usize },
        2usize,
        concat!(
            "Offset of field: ",
            stringify!(desc_ptr),
            "::",
            stringify!(address)
        )
    );
}
