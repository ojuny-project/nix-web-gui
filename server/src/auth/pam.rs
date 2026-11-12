use pam_client::conv_mock::Conversation;
use pam_client::{Context, Flag};

use super::AuthError;

pub fn authenticate(service: &str, username: &str, password: &str) -> Result<(), AuthError> {
    let conversation = Conversation::with_credentials(username, password);
    let mut context = Context::new(service, Some(username), conversation)
        .map_err(|error| AuthError::Denied(error.to_string()))?;

    context
        .authenticate(Flag::NONE)
        .map_err(|_| AuthError::InvalidCredentials)?;
    context
        .acct_mgmt(Flag::NONE)
        .map_err(|error| AuthError::Denied(error.to_string()))?;

    Ok(())
}
