use rkyv::util::AlignedVec;

use crate::redis::{
    ErrorKind, FromRedisValue, RedisError, RedisResult, RedisWrite, ToRedisArgs, Value,
};

pub(crate) struct BytesWrap<B>(pub(crate) B);

impl<B: AsRef<[u8]>> ToRedisArgs for BytesWrap<B> {
    fn write_redis_args<W: ?Sized + RedisWrite>(&self, out: &mut W) {
        self.0.as_ref().write_redis_args(out);
    }
}

impl FromRedisValue for BytesWrap<AlignedVec<16>> {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        match v {
            Value::Data(data) => {
                let mut bytes = AlignedVec::new();
                bytes.reserve_exact(data.len());
                bytes.extend_from_slice(data);

                Ok(Self(bytes))
            }
            value => Err(RedisError::from((
                ErrorKind::TypeError,
                "Response was of incompatible type",
                format!("Response type not byte list compatible. (response was {value:?})"),
            ))),
        }
    }
}
