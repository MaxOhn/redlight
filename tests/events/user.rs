use twilight_model::{
    id::Id,
    user::{PremiumType, User, UserFlags},
    util::ImageHash,
};

pub fn user() -> User {
    User {
        accent_color: Some(123),
        avatar: Some(ImageHash::new([5; 16], false)),
        avatar_decoration: Some(ImageHash::new([6; 16], false)),
        avatar_decoration_data: None,
        banner: Some(ImageHash::new([7; 16], false)),
        bot: false,
        discriminator: 1234,
        email: None,
        flags: Some(UserFlags::ACTIVE_DEVELOPER | UserFlags::VERIFIED_DEVELOPER),
        global_name: Some("global name".to_owned()),
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
