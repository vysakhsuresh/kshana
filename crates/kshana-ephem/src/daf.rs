//! NAIF Double-precision Array File (DAF) structure, the container for SPK data.

/// A DAF segment summary (SPK flavour: 2 doubles + 6 integers).
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    /// Start of coverage, TDB seconds past J2000.
    pub start_et: f64,
    /// End of coverage, TDB seconds past J2000.
    pub end_et: f64,
    /// NAIF id of the target.
    pub target: i32,
    /// NAIF id of the centre.
    pub center: i32,
    /// Reference frame id (1 = J2000).
    pub frame: i32,
    /// SPK data type.
    pub data_type: i32,
    /// First double address (1-based).
    pub start_addr: i32,
    /// Last double address (1-based).
    pub end_addr: i32,
}

/// Parsed DAF with its bytes retained for random access.
#[derive(Debug)]
pub struct Daf {
    bytes: Vec<u8>,
    little_endian: bool,
    /// Segment summaries in file order.
    pub summaries: Vec<Summary>,
}

/// Why a DAF could not be parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DafError {
    /// Not a `DAF/SPK` file.
    NotSpk,
    /// Byte order marker is neither `LTL-IEEE` nor `BIG-IEEE`.
    UnknownByteOrder,
    /// ND/NI do not match the SPK layout (2 doubles, 6 integers).
    UnexpectedLayout,
    /// File shorter than its own records claim.
    Truncated,
}

const RECORD: usize = 1024;

impl Daf {
    /// Parse a DAF from its full contents.
    pub fn parse(bytes: Vec<u8>) -> Result<Self, DafError> {
        if bytes.len() < RECORD {
            return Err(DafError::Truncated);
        }
        if &bytes[0..8] != b"DAF/SPK " {
            return Err(DafError::NotSpk);
        }
        let little_endian = match &bytes[88..96] {
            b"LTL-IEEE" => true,
            b"BIG-IEEE" => false,
            _ => return Err(DafError::UnknownByteOrder),
        };
        let mut daf = Daf {
            bytes,
            little_endian,
            summaries: Vec::new(),
        };
        let nd = daf.i32_at(8);
        let ni = daf.i32_at(12);
        if nd != 2 || ni != 6 {
            return Err(DafError::UnexpectedLayout);
        }
        let mut record = daf.i32_at(76) as usize; // FWARD, 1-based record number
        while record != 0 {
            let base = (record - 1) * RECORD;
            if base + RECORD > daf.bytes.len() {
                return Err(DafError::Truncated);
            }
            let next = daf.f64_at(base) as usize;
            let nsum = daf.f64_at(base + 16) as usize;
            // each summary: 2 doubles + 6 ints packed into 3 doubles = 40 bytes
            for s in 0..nsum {
                let off = base + 24 + s * 40;
                daf.summaries.push(Summary {
                    start_et: daf.f64_at(off),
                    end_et: daf.f64_at(off + 8),
                    target: daf.i32_at(off + 16),
                    center: daf.i32_at(off + 20),
                    frame: daf.i32_at(off + 24),
                    data_type: daf.i32_at(off + 28),
                    start_addr: daf.i32_at(off + 32),
                    end_addr: daf.i32_at(off + 36),
                });
            }
            record = next;
        }
        Ok(daf)
    }

    /// Double at 1-based DAF address.
    pub fn double(&self, addr: usize) -> f64 {
        self.f64_at((addr - 1) * 8)
    }

    /// Slice of doubles starting at 1-based address.
    pub fn doubles(&self, addr: usize, count: usize) -> Vec<f64> {
        (0..count).map(|i| self.double(addr + i)).collect()
    }

    fn f64_at(&self, off: usize) -> f64 {
        let b: [u8; 8] = self.bytes[off..off + 8].try_into().expect("8 bytes");
        if self.little_endian {
            f64::from_le_bytes(b)
        } else {
            f64::from_be_bytes(b)
        }
    }

    fn i32_at(&self, off: usize) -> i32 {
        let b: [u8; 4] = self.bytes[off..off + 4].try_into().expect("4 bytes");
        if self.little_endian {
            i32::from_le_bytes(b)
        } else {
            i32::from_be_bytes(b)
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Build a minimal synthetic little-endian DAF/SPK with one Type 2 segment.
    pub(crate) fn synthetic(coeffs_xyz: [&[f64]; 3], init: f64, intlen: f64) -> Vec<u8> {
        let n = coeffs_xyz[0].len();
        let rsize = 2 + 3 * n;
        let mut file = vec![0u8; RECORD * 3];
        file[0..8].copy_from_slice(b"DAF/SPK ");
        file[8..12].copy_from_slice(&2i32.to_le_bytes());
        file[12..16].copy_from_slice(&6i32.to_le_bytes());
        file[76..80].copy_from_slice(&2i32.to_le_bytes()); // FWARD
        file[80..84].copy_from_slice(&2i32.to_le_bytes()); // BWARD
        file[88..96].copy_from_slice(b"LTL-IEEE");
        // summary record (record 2)
        let base = RECORD;
        file[base..base + 8].copy_from_slice(&0f64.to_le_bytes()); // next
        file[base + 8..base + 16].copy_from_slice(&0f64.to_le_bytes()); // prev
        file[base + 16..base + 24].copy_from_slice(&1f64.to_le_bytes()); // nsum
        let start_addr = (RECORD * 2 / 8 + 1) as i32; // first double of record 3
        let data: Vec<f64> = {
            let mut d = vec![init + intlen / 2.0, intlen / 2.0];
            for c in coeffs_xyz.iter() {
                d.extend_from_slice(c);
            }
            d.extend_from_slice(&[init, intlen, rsize as f64, 1.0]);
            d
        };
        let end_addr = start_addr + data.len() as i32 - 1;
        let s = base + 24;
        file[s..s + 8].copy_from_slice(&init.to_le_bytes());
        file[s + 8..s + 16].copy_from_slice(&(init + intlen).to_le_bytes());
        for (i, v) in [399i32, 3, 1, 2, start_addr, end_addr].iter().enumerate() {
            file[s + 16 + i * 4..s + 20 + i * 4].copy_from_slice(&v.to_le_bytes());
        }
        // data record (record 3)
        for (i, v) in data.iter().enumerate() {
            let o = RECORD * 2 + i * 8;
            file[o..o + 8].copy_from_slice(&v.to_le_bytes());
        }
        file
    }

    #[test]
    fn parses_synthetic_summary() {
        let bytes = synthetic([&[1.0, 2.0], &[3.0, 4.0], &[5.0, 6.0]], 0.0, 100.0);
        let daf = Daf::parse(bytes).unwrap();
        assert_eq!(daf.summaries.len(), 1);
        let s = &daf.summaries[0];
        assert_eq!((s.target, s.center, s.data_type), (399, 3, 2));
        assert_eq!(s.end_et, 100.0);
    }

    #[test]
    fn rejects_non_spk() {
        assert_eq!(Daf::parse(vec![0u8; 2048]).unwrap_err(), DafError::NotSpk);
    }
}
