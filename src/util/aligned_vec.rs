use crate::redis::{RedisWrite, ToRedisArgs};

pub(crate) struct BytesRedisArgs<B>(pub(crate) B);

impl<B: AsRef<[u8]>> ToRedisArgs for BytesRedisArgs<B> {
    #[inline]
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        self.0.as_ref().write_redis_args(out)
    }
}
