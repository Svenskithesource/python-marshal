#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Debug, Ord)]
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

impl PyVersion {
    const MAGIC_NUMBERS: &'static [(u32, PyVersion)] = &[
        (0x0A0D0C3B, PyVersion { major: 3, minor: 0 }),
        (0x0A0D0C4F, PyVersion { major: 3, minor: 1 }),
        (0x0A0D0C6C, PyVersion { major: 3, minor: 2 }),
        (0x0A0D0C9E, PyVersion { major: 3, minor: 3 }),
        (0x0A0D0CEE, PyVersion { major: 3, minor: 4 }),
        (0x0A0D0D16, PyVersion { major: 3, minor: 5 }),
        (0x0A0D0D33, PyVersion { major: 3, minor: 6 }),
        (0x0A0D0D42, PyVersion { major: 3, minor: 7 }),
        (0x0A0D0D55, PyVersion { major: 3, minor: 8 }),
        (0x0A0D0D61, PyVersion { major: 3, minor: 9 }),
        (
            0x0A0D0D6F,
            PyVersion {
                major: 3,
                minor: 10,
            },
        ),
        (
            0x0A0D0DA7,
            PyVersion {
                major: 3,
                minor: 11,
            },
        ),
        (
            0x0A0D0DCB,
            PyVersion {
                major: 3,
                minor: 12,
            },
        ),
        (
            0x0A0D0DF3,
            PyVersion {
                major: 3,
                minor: 13,
            },
        ),
    ];

    fn from_magic(magic: u32) -> Result<Self, crate::Error> {
        Self::MAGIC_NUMBERS
            .iter()
            .find(|&&(num, _)| num == magic)
            .map(|&(_, version)| version)
            .ok_or(crate::Error::UnsupportedMagicNumber(magic))
    }

    fn to_magic(&self) -> Result<u32, crate::Error> {
        Self::MAGIC_NUMBERS
            .iter()
            .find(|&&(_, ref version)| version == self)
            .map(|&(num, _)| num)
            .ok_or(crate::Error::UnsupportedPyVersion(self.clone()))
    }
}

impl TryFrom<u32> for PyVersion {
    type Error = crate::Error;

    fn try_from(vers: u32) -> Result<Self, Self::Error> {
        PyVersion::from_magic(vers)
    }
}

impl TryFrom<PyVersion> for u32 {
    type Error = crate::Error;

    fn try_from(vers: PyVersion) -> Result<Self, Self::Error> {
        vers.to_magic()
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
