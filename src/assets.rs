#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum AssetSource
{
    Path(String),
    Bytes(usize, usize) // (address, length)
}

impl AssetSource
{
    pub(crate) fn path(path: &str) -> Self { AssetSource::Path(path.to_string()) }
    pub(crate) fn bytes(bytes: &'static [u8]) -> Self { AssetSource::Bytes(bytes.as_ptr() as usize, bytes.len()) }
}
