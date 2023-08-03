use twilight_model::{
    id::Id,
    user::{CurrentUser, PremiumType, User, UserFlags},
    util::ImageHash,
};

pub fn current_user() -> CurrentUser {
    CurrentUser {
        accent_color: Some(234),
        avatar: Some(ImageHash::new([4; 16], true)),
        banner: Some(ImageHash::new([4; 16], false)),
        bot: true,
        discriminator: 2345,
        email: None,
        flags: Some(UserFlags::ACTIVE_DEVELOPER | UserFlags::VERIFIED_DEVELOPER),
        id: Id::new(789),
        locale: Some("en-US".to_owned()),
        mfa_enabled: false,
        name: "current_user".to_owned(),
        premium_type: Some(PremiumType::None),
        public_flags: Some(UserFlags::ACTIVE_DEVELOPER),
        verified: Some(true),
    }
}

pub fn user() -> User {
    User {
        accent_color: Some(123),
        avatar: Some(ImageHash::new([5; 16], false)),
        banner: Some(ImageHash::new([5; 16], false)),
        bot: false,
        discriminator: 1234,
        email: None,
        flags: Some(UserFlags::ACTIVE_DEVELOPER | UserFlags::VERIFIED_DEVELOPER),
        id: Id::new(1),
        locale: Some("en-US".to_owned()),
        mfa_enabled: Some(false),
        name: "user".to_owned(),
        premium_type: Some(PremiumType::None),
        public_flags: Some(UserFlags::ACTIVE_DEVELOPER),
        system: Some(false),
        verified: Some(false),
    }
}
