use std::any;

#[derive(Eq, PartialEq, PartialOrd, Debug, Ord)]
pub struct PyVersion {
    pub major: u8,
    pub minor: u8,
}

impl PyVersion {
    pub fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }
}

impl From<(u8, u8)> for PyVersion {
    fn from(vers: (u8, u8)) -> Self {
        Self {
            major: vers.0,
            minor: vers.1,
        }
    }
}

impl TryFrom<u32> for PyVersion {
    type Error = anyhow::Error;

    fn try_from(vers: u32) -> Result<Self, Self::Error> {
        match vers {
            0x0A0D0C3B => Ok(PyVersion::new(3, 0)), // 0x0A0D0C3A + 1 == 0x0A0D0C3B (Python does it because of legacy reasons)
            0x0A0D0C4F => Ok(PyVersion::new(3, 1)), // 0x0A0D0C4E + 1 == 0x0A0D0C4F
            0x0A0D0C6C => Ok(PyVersion::new(3, 2)),
            0x0A0D0C9E => Ok(PyVersion::new(3, 3)),
            0x0A0D0CEE => Ok(PyVersion::new(3, 4)),
            0x0A0D0D16 => Ok(PyVersion::new(3, 5)),
            0x0A0D0D33 => Ok(PyVersion::new(3, 6)),
            0x0A0D0D42 => Ok(PyVersion::new(3, 7)),
            0x0A0D0D55 => Ok(PyVersion::new(3, 8)),
            0x0A0D0D61 => Ok(PyVersion::new(3, 9)),
            0x0A0D0D6F => Ok(PyVersion::new(3, 10)),
            0x0A0D0DA7 => Ok(PyVersion::new(3, 11)),
            0x0A0D0DCB => Ok(PyVersion::new(3, 12)),
            0x0A0D0DF3 => Ok(PyVersion::new(3, 13)),
            _ => Err(anyhow::anyhow!(format!(
                "unsupported magic number: 0x{:08X}",
                vers
            ))),
        }
    }
}

impl PartialEq<(u8, u8)> for PyVersion {
    fn eq(&self, other: &(u8, u8)) -> bool {
        self.major == other.0 && self.minor == other.1
    }
}

impl PartialOrd<(u8, u8)> for PyVersion {
    fn partial_cmp(&self, other: &(u8, u8)) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&PyVersion::new(other.0, other.1)))
    }
}
