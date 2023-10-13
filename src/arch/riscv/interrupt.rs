//const PLIC_PRIORITY: usize = 0x0c00_0000;
//const PLIC_PENDING: usize = 0x0c00_1000;
//const PLIC_INT_ENABLE: usize = 0x0c00_2000;
//const PLIC_THRESHOLD: usize = 0x0c20_0000;
//const PLIC_CLAIM: usize = 0x0c20_0004;

const PLIC_INT_ENABLE: usize = 0x0c00_2000;
const PLIC_INT_PRIORITY: usize = 0x0c00_0000;
const PLIC_INT_THRESHOLD: usize = 0x0c20_0000;
const PLIC_INT_CLAIM: usize = 0x0c20_0004;

pub fn enable(id: u32) {
    let enables = PLIC_INT_ENABLE as *mut u32;

    unsafe {
        enables.write_volatile(enables.read_volatile() | (1 << id));
    }
}

pub fn set_priority(id: u32, prio: u8) {
    assert!(prio < 8);

    let prio_regs = PLIC_INT_PRIORITY as *mut u32;
    unsafe {
        let prio_reg = prio_regs.add(id as usize);
        prio_reg.write_volatile(prio as u32);
    }
}

pub fn set_threshold(threshold: u8) {
    assert!(threshold < 8);

    let thresh_reg = PLIC_INT_THRESHOLD as *mut u32;
    unsafe {
        thresh_reg.write_volatile(threshold as u32);
    }
}

pub fn next_interrupt() -> Option<u32> {
    let claim_reg = PLIC_INT_CLAIM as *const u32;

    let claim_no = unsafe { claim_reg.read_volatile() };

    match claim_no {
        0 => None,
        id => Some(id),
    }
}

pub fn complete(id: u32) {
    let claim_reg = PLIC_INT_CLAIM as *mut u32;

    unsafe { claim_reg.write_volatile(id) };
}
