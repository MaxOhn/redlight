use crate::redis::{RedisWrite, ToRedisArgs};

pub(crate) struct BytesArg<B>(pub(crate) B);

impl<B: AsRef<[u8]>> ToRedisArgs for BytesArg<B> {
    fn write_redis_args<W: ?Sized + RedisWrite>(&self, out: &mut W) {
        self.0.as_ref().write_redis_args(out)
    }
}
