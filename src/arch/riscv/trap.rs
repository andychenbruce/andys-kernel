use super::csr_stuff;
use crate::kprintln;

#[derive(Debug)]
enum InterruptExeption {
    SSoftware,
    MSoftware,
    STimer,
    MTimer,
    SExternal,
    MExternal,
}

#[derive(Debug)]
enum ExeptionCode {
    Interrupt(InterruptExeption),
    InstructionAddressMisaligned,
    InstructionAccessFault,
    IllegalInstruction,
    Breakpoint,
    LoadAddressMisaligned,
    LoadAccessFault,
    StoreOrAMOAddressMisaligned,
    StoreOrAMOAccessFault,
    EnvironmentCallFromUMode,
    EnvironmentCallFromSMode,
    EnvironmentCallFromMMode,
    InstructionPageFault,
    LoadPageFault,
    StoreOrAMOPageFault,
}

#[no_mangle]
extern "C" fn rust_andy_trap(
    mepc: usize,
    _mtval: usize,
    mcause: usize,
    _mhart: usize,
    mstatus: usize,
) -> usize {
    let _mstatus = csr_stuff::Mstatus::new(mstatus);

    let from_interrupt: bool = ((mcause >> 63) & 1) == 1;
    let exeption_code = mcause & 0x7fffffff;

    use ExeptionCode as Ex;
    use InterruptExeption as Inr;

    let code = if from_interrupt {
        use Ex::Interrupt as I;
        match exeption_code {
            1 => I(Inr::SSoftware),
            3 => I(Inr::MSoftware),
            5 => I(Inr::STimer),
            7 => I(Inr::MTimer),
            9 => I(Inr::SExternal),
            11 => I(Inr::MExternal),
            _ => unreachable!(),
        }
    } else {
        match exeption_code {
            0 => Ex::InstructionAddressMisaligned,
            1 => Ex::InstructionAccessFault,
            2 => Ex::IllegalInstruction,
            3 => Ex::Breakpoint,
            4 => Ex::LoadAddressMisaligned,
            5 => Ex::LoadAccessFault,
            6 => Ex::StoreOrAMOAddressMisaligned,
            7 => Ex::StoreOrAMOAccessFault,
            8 => Ex::EnvironmentCallFromUMode,
            9 => Ex::EnvironmentCallFromSMode,
            11 => Ex::EnvironmentCallFromMMode,
            12 => Ex::InstructionPageFault,
            13 => Ex::LoadPageFault,
            15 => Ex::StoreOrAMOPageFault,
            _ => unreachable!(),
        }
    };

    match code {
        Ex::InstructionAccessFault => panic!("bruh"),
        Ex::Interrupt(interrupt) => match interrupt {
            InterruptExeption::MExternal => machine_external_interrupt_handler(),
            _ => panic!("unhandled interrupt: {:?}", interrupt),
        },
        int => panic!("unhandled exception: {:?}", int),
    }

    mepc
}

fn machine_external_interrupt_handler() {
    if let Some(interrupt) = super::interrupt::next_interrupt() {
        match interrupt {
            10 => {
                let maybe_byte = {
                    let mut lock = crate::WRITER.try_lock().unwrap();
                    lock.read_byte()
                };
                if let Some(byte) = maybe_byte {
                    kprintln!("got byte {}", byte);
                } else {
                    panic!("sfwefefw");
                }
            }
            _ => todo!(),
        }
        super::interrupt::complete(interrupt);
    } else {
        panic!("idk lol");
    }
}
