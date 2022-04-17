//! MBR Partition Metadata
//!
//! This struct represents partitions' metadata.
pub struct MBRPartitions<'a> {
    pub partitions: [PartitionMetaData<'a>; 4]
}

pub struct PartitionMetaData<'a> {
    data: &'a [u8; 16]
}

impl<'a> PartitionMetaData<'a> {
    /// Attempt to parse a Boot Parameter Block from a 512 byte sector.
    pub fn create_from_bytes(data: &[u8; 16]) -> Result<PartitionMetaData, &'static str> {
        Ok(PartitionMetaData { data })
    }

    define_field!( u8, 0x00, status);
    define_field!( u8, 0x01, begin_head);
    // 0x02 - 0x03 begin sector & begin cylinder
    define_field!( u8, 0x04, filesystem_flag);
    define_field!( u8, 0x05, end_head);
    // 0x06 - 0x07 end sector & edn cylinder
    define_field!(u32, 0x08, begin_lba);
    define_field!(u32, 0x0c, total_lba);

    pub fn is_active(&self) -> bool {
        self.status() == 0x80
    }

    pub fn is_extended(&self) -> bool {
        self.filesystem_flag() == 0x05
    }

    pub fn begin_sector(&self) -> u8 {
        self.data[2] & 0x3f
    }

    pub fn begin_cylinder(&self) -> u16 {
        (self.data[2] as u16 & 0xc0) << 2 | (self.data[3] as u16)
    }

    pub fn end_sector(&self) -> u8 {
        self.data[6] & 0x3f
    }

    pub fn end_cylinder(&self) -> u16 {
        (self.data[6] as u16 & 0xc0) << 2 | (self.data[7] as u16)
    }
}

impl<'a> core::fmt::Debug for PartitionMetaData<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Partition Meta Data: {{\n")?;
        write!(f, "  Active: {}\n", self.is_active())?;
        write!(f, "  Begin Head: 0x{:02x}\n", self.begin_head())?;
        write!(f, "  Begin Sector: 0x{:04x}\n", self.begin_sector())?;
        write!(f, "  Begin Cylinder: 0x{:04x}\n", self.begin_cylinder())?;
        write!(f, "  Filesystem Flag: 0x{:02x}\n", self.filesystem_flag())?;
        write!(f, "  End Head: 0x{:02x}\n", self.end_head())?;
        write!(f, "  End Sector: 0x{:04x}\n", self.end_sector())?;
        write!(f, "  End Cylinder: 0x{:04x}\n", self.end_cylinder())?;
        write!(f, "  Begin LBA: 0x{:08x}\n", self.begin_lba())?;
        write!(f, "  Total LBA: 0x{:08x}\n", self.total_lba())?;
        write!(f, "}}")
    }
}
