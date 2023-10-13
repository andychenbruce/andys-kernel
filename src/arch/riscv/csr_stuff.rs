#[derive(Debug, PartialEq)]
pub enum Privilege {
    Machine,
    Supervisor,
    User,
}

pub struct Mstatus {
    pub mpp: Privilege,
}

impl Mstatus {
    pub fn new(bits: usize) -> Self {
        let mpp_bit0 = (bits & (1 << 11)) != 0;
        let mpp_bit1 = (bits & (1 << 12)) != 0;

        let mpp = match (mpp_bit1, mpp_bit0) {
            (false, false) => Privilege::User,
            (false, true) => Privilege::Supervisor,
            (true, true) => Privilege::Machine,
            _ => unreachable!(),
        };

        Mstatus { mpp }
    }
}
