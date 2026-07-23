#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, Copy)]
pub struct RpcRequestId(pub u64);

impl From<u64> for RpcRequestId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[cfg(test)]
impl PartialEq<u64> for RpcRequestId {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}
