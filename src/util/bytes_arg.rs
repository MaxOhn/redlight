use rkyv::AlignedVec;

use crate::redis::{RedisWrite, ToRedisArgs};

pub(crate) struct BytesArg(pub(crate) AlignedVec);

impl ToRedisArgs for BytesArg {
    fn write_redis_args<W: ?Sized + RedisWrite>(&self, out: &mut W) {
        self.0.as_slice().write_redis_args(out)
    }
}
