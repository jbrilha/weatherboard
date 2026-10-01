pub enum ByteReaderError {
    NotEnoughBytes,
}

pub struct ByteReader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> ByteReader<'a> {
    pub fn new(bytes: &'a [u8], cursor: usize) -> Self {
        Self { bytes, cursor }
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N], ByteReaderError> {
        let end = self.cursor + N;
        let chunk = self
            .bytes
            .get(self.cursor..end)
            .ok_or(ByteReaderError::NotEnoughBytes)?
            .try_into()
            .unwrap();
        self.cursor = end;
        Ok(chunk)
    }

    pub fn u16_le(&mut self) -> Result<u16, ByteReaderError> {
        let bytes = self.take()?;
        Ok(u16::from_le_bytes(bytes))
    }

    pub fn u32_le(&mut self) -> Result<u32, ByteReaderError> {
        let bytes = self.take()?;
        Ok(u32::from_le_bytes(bytes))
    }

    pub fn u16_be(&mut self) -> Result<u16, ByteReaderError> {
        let bytes = self.take()?;
        Ok(u16::from_be_bytes(bytes))
    }

    pub fn u32_be(&mut self) -> Result<u32, ByteReaderError> {
        let bytes = self.take()?;
        Ok(u32::from_be_bytes(bytes))
    }

    pub fn bytes<const N: usize>(&mut self) -> Result<[u8; N], ByteReaderError> {
        self.take()
    }
}
