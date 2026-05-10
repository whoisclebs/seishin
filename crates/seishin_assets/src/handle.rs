use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetHandle<T> {
    id: u64,
    _asset: PhantomData<T>,
}

impl<T> AssetHandle<T> {
    pub fn from_id(id: u64) -> Self {
        Self {
            id,
            _asset: PhantomData,
        }
    }

    pub fn id(self) -> u64 {
        self.id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageAsset;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_asset_handles_keep_stable_ids() {
        let handle = AssetHandle::<ImageAsset>::from_id(42);

        assert_eq!(handle.id(), 42);
    }
}
