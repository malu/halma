use std::ops::{BitAnd, BitOr, BitXor, Not, Shr, Shl};

// We interpret the values in the following layout:
//  **00**01**02**03**04**05*/06\*07**08**09**0A**0B**0C****
//  0D**0E**0F**10**11**12*/13  14\*15**16**17**18**19**1A**
//  **1B**1C**1D**1E**1F*/20  21  22\*23**24**25**26**27****
//  28**29**2A**2B**2C*/2D  2E  2F  30\*31**32**33**34**35**
//  *\36  37  38  39  3A  3B  3C  3D  3E  3F  40  41  42/***
//  43*\44  45  46  47  48  49  4A  4B  4C  4D  4E  4F/*50**
//  **51*\52  53  54  55  56  57  58  59  5A  5B  5C/*5D****
//  5E**5F*\60  61  62  63  64  65  66  67  68  69/*6A**6B**
//  **6C**6D*|6E  6F  70  71  72  73  74  75  76|*77**78****
//  79**7A*/7B  7C  7D  7E  7F  80  81  82  83  84\*85**86**
//  **87*/88  89  8A  8B  8C  8D  8E  8F  90  91  92\*93****
//  94*/95  96  97  98  99  9A  9B  9C  9D  9E  9F  A0\*A1**
//  */A2  A3  A4  A5  A6  A7  A8  A9  AA  AB  AC  AD  AE\***
//  AF**B0**B1**B2**B3*\B4  B5  B6  B7/*B8**B9**BA**BB**BC**
//  **BD**BE**BF**C0**C1*\C2  C3  C4/*C5**C6**C7**C8**C9****
//  CA**CB**CC**CD**CE**CF*\D0  D1/*D2**D3**D4**D5**D6**D7**
//  **D8**D9**DA**DB**DC**DD*\DE/*DF**E0**E1**E2**E3**E4****
//  E5**E6**E7**E8**E9**EA**EB**EC**ED**EE**EF**F0**F1**F2**
//  **F3**F4**F5**F6**F7**F8**F9**FA**FB**FC**FD**FE**FF****

pub type BitIndex = u8;

const LONG_ROW: u8 = 13;
const TWO_ROWS: u8 = 27;

pub fn pos_to_index(x: u8, y: u8) -> BitIndex {
    y/2 * TWO_ROWS + (y%2)*LONG_ROW + x
}

pub fn index_to_pos(index: BitIndex) -> (i8, i8) {
    let mut index = index;
    let mut y = 0;

    y += 2*(index/TWO_ROWS);
    index %= TWO_ROWS;

    y += index/LONG_ROW;
    index -= (index/LONG_ROW)*LONG_ROW;

    let x = index;
    (x as i8, y as i8)
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bitboard([u64; 4]);

pub const BB_INVALID: Bitboard = Bitboard(
    [
    0b00000000_00111110_00011111_11111000_11111111_11100111_11111111_10111111,
    0b00000111_10000000_00111100_00000000_11100000_00000011_00000000_00001000,
    0b11111111_00001111_10000000_00000010_00000000_00011000_00000000_11100000,
    0b11111111_11111111_11111111_11111111_10111111_11111100_11111111_11100011,
    ]
);

pub const BB_TARGET: [Bitboard; 2] = [
    Bitboard(
        [
        0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
        0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
        0b00000000_11110000_00000111_11000000_00000000_00000000_00000000_00000000,
        0b00000000_00000000_00000000_00000000_01000000_00000011_00000000_00011100,
        ]
    ),
    Bitboard(
        [
        0b01111100_00000001_11100000_00000111_00000000_00011000_00000000_01000000,
        0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
        0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
        0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
        ]
    ),
];


impl Bitboard {
    pub fn bit(index: BitIndex) -> Self {
        let mut result = Bitboard::default();
        result.set_bit(index);
        result
    }

    pub fn pop(&mut self) -> Option<BitIndex> {
        for i in 0..4 {
            if self.0[i] == 0 {
                continue;
            }

            let index = 64*i as u8 + self.0[i].trailing_zeros() as u8;
            self.0[i] &= self.0[i] - 1;
            return Some(index);
        }

        None
    }

    pub fn is_empty(&self) -> bool {
        self.0[0] == 0 &&
        self.0[1] == 0 &&
        self.0[2] == 0 &&
        self.0[3] == 0
    }

    pub fn set_bit(&mut self, i: BitIndex) {
        let i = i as usize;
        self.0[i >> 6] |= 1 << (i & 0b00111111);
    }

    pub fn unset_bit(&mut self, i: BitIndex) {
        let i = i as usize;
        self.0[i >> 6] &= !(1 << (i & 0b00111111));
    }

    pub fn get_bit(&self, i: BitIndex) -> bool {
        let i = i as usize;
        self.0[i >> 6] & (1 << (i & 0b00111111)) > 0
    }

    pub fn ones(self) -> OnesIterator {
        OnesIterator {
            bb: self,
            i: 0,
        }
    }
}

pub struct OnesIterator {
    bb: Bitboard,
    i: usize,
}

impl Iterator for OnesIterator {
    type Item = BitIndex;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.bb.0[0].count_ones()
            + self.bb.0[1].count_ones()
            + self.bb.0[2].count_ones()
            + self.bb.0[3].count_ones();
        (remaining as usize, Some(remaining as usize))
    }

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.i < 4 && self.bb.0[self.i] == 0 {
            self.i += 1;
        }

        if self.i == 4 {
            return None;
        }

        let index = 64 * self.i as u8 + self.bb.0[self.i].trailing_zeros() as u8;
        // Clear the least significant one bit.
        self.bb.0[self.i] &= self.bb.0[self.i]-1;
        return Some(index);
    }
}

impl BitAnd for Bitboard {
    type Output = Self;

    fn bitand(self, other: Self) -> Self {
        Bitboard([
                 self.0[0] & other.0[0], 
                 self.0[1] & other.0[1], 
                 self.0[2] & other.0[2], 
                 self.0[3] & other.0[3], 
        ])
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Bitboard([
                 self.0[0] | other.0[0], 
                 self.0[1] | other.0[1], 
                 self.0[2] | other.0[2], 
                 self.0[3] | other.0[3], 
        ])
    }
}

impl BitXor for Bitboard {
    type Output = Self;

    fn bitxor(self, other: Self) -> Self {
        Bitboard([
                 self.0[0] ^ other.0[0], 
                 self.0[1] ^ other.0[1], 
                 self.0[2] ^ other.0[2], 
                 self.0[3] ^ other.0[3], 
        ])
    }
}

impl Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self {
        Bitboard([
                 !self.0[0], 
                 !self.0[1], 
                 !self.0[2], 
                 !self.0[3], 
        ])
    }
}

impl Shr<u8> for Bitboard {
    type Output = Self;

    fn shr(self, bits: u8) -> Self {
        assert!(bits < 64);
        Bitboard([
                 (self.0[0] >> bits) | (self.0[1] << (64 - bits)),
                 (self.0[1] >> bits) | (self.0[2] << (64 - bits)),
                 (self.0[2] >> bits) | (self.0[3] << (64 - bits)),
                 self.0[3] >> bits,
        ])
    }
}

impl Shl<u8> for Bitboard {
    type Output = Self;

    fn shl(self, bits: u8) -> Self {
        assert!(bits < 64);
        Bitboard([
                 self.0[0] << bits,
                 (self.0[1] << bits) | (self.0[0] >> (64 - bits)),
                 (self.0[2] << bits) | (self.0[1] >> (64 - bits)),
                 (self.0[3] << bits) | (self.0[2] >> (64 - bits)),
        ])
    }
}

mod tests {
    #[test]
    fn test_bitboard_shr() {
        use ai::bitboard::Bitboard;
        let bb =       Bitboard([0x0123456789ABCDEF, 0x23456789ABCDEF12, 0x456789ABCDEF0123, 0x6789ABCDEF012345]);
        let result8 =  Bitboard([0x120123456789ABCD, 0x2323456789ABCDEF, 0x45456789ABCDEF01, 0x006789ABCDEF0123]);
        let result32 = Bitboard([0xABCDEF1201234567, 0xCDEF012323456789, 0xEF012345456789AB, 0x000000006789ABCD]);
        assert_eq!(bb >> 8, result8);
        assert_eq!(bb >> 32, result32);
    }

    #[test]
    fn test_bitboard_shl() {
        use ai::bitboard::Bitboard;
        let bb =       Bitboard([0xABCDEF1201234567, 0xCDEF012323456789, 0xEF012345456789AB, 0x000000006789ABCD]);
        let result32 = Bitboard([0x0123456700000000, 0x23456789ABCDEF12, 0x456789ABCDEF0123, 0x6789ABCDEF012345]);
        assert_eq!(bb << 32, result32);
    }
}
