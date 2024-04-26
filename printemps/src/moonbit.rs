#[derive(Debug)]
pub struct MoonMem {
    id: i32,
    offset: usize,
    length: usize,
}

impl MoonMem {
    pub fn new(offset: usize, length: usize) -> Self {
        Self { id: rand::random(), offset, length }
    }

    pub fn id(self: &Self) -> i32 {
        self.id
    }

    pub fn offset(self: &Self) -> usize {
        self.offset
    }

    pub fn length(self: &Self) -> usize {
        self.length
    }
}

#[derive(Debug)]
pub struct HostString(String);

impl HostString {
    pub fn new<T: AsRef<str>>(string: T) -> Self {
        Self(string.as_ref().into())
    }

    pub fn utf16_words(&self) -> usize {
        self.0.chars().map(|c| { c.len_utf16() }).sum()
    }

    pub fn fill_mem(&self, mem: &mut [u8]) -> anyhow::Result<()> {
        if mem.len() < (self.utf16_words() * 2) {
            return Err(anyhow::anyhow!("insufficient memory"));
        }
        let mut ptr = 0usize;
        for utf16_word in self.0.encode_utf16() {
            mem[ptr] = (utf16_word & 0x00ffu16) as u8;
            mem[ptr + 1] = ((utf16_word & 0xff00u16) >> 8) as u8;
            ptr += 2;
        }
        Ok(())
    }
}

impl Into<String> for HostString {
    fn into(self) -> String {
        self.0
    }
}

impl AsRef<str> for HostString {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
